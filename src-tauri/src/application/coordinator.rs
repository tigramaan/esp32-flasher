use crate::application::{OperationEvent, OperationLog};
use crate::platform::logging::OperationLogger;
use crate::platform::reports::FactoryReporter;
use crate::platform::serial::MonitorHandle;
use crate::platform::storage::PortableDataStore;
use chrono::Utc;
use programmer_core::{ErrorCode, OperationError, OperationStage, Result};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub type SharedWorker = Arc<Mutex<Child>>;

struct ActiveOperation {
    operation_id: String,
    cancelled: Arc<AtomicBool>,
    worker: Option<SharedWorker>,
}

pub struct AppState {
    pub data: PortableDataStore,
    pub reports: FactoryReporter,
    operation_active: AtomicBool,
    active_operation: Mutex<Option<ActiveOperation>>,
    monitor: Mutex<Option<MonitorHandle>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            data: PortableDataStore::discover(),
            reports: FactoryReporter::new(),
            operation_active: AtomicBool::new(false),
            active_operation: Mutex::new(None),
            monitor: Mutex::new(None),
        }
    }

    pub fn acquire(self: &Arc<Self>, operation_id: String) -> Result<OperationLease> {
        if self
            .operation_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(OperationError::new(
                ErrorCode::OperationInProgress,
                "Другая операция уже выполняется",
            ));
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        *self
            .active_operation
            .lock()
            .expect("active operation lock poisoned") = Some(ActiveOperation {
            operation_id: operation_id.clone(),
            cancelled: cancelled.clone(),
            worker: None,
        });
        let lease = OperationLease {
            state: self.clone(),
            operation_id,
            cancelled,
        };
        let mut monitor = self.monitor.lock().expect("monitor lock poisoned");
        if let Some(previous) = monitor.take() {
            previous.stop_and_wait()?;
        }
        Ok(lease)
    }

    pub fn register_worker(&self, operation_id: &str, worker: SharedWorker) -> Result<()> {
        let cancelled = {
            let mut active = self
                .active_operation
                .lock()
                .expect("active operation lock poisoned");
            let operation = active.as_mut().ok_or_else(|| {
                OperationError::new(ErrorCode::InvalidState, "Активная операция не найдена")
            })?;
            if operation.operation_id != operation_id {
                return Err(OperationError::new(
                    ErrorCode::InvalidState,
                    "Идентификатор flash worker не совпадает с активной операцией",
                ));
            }
            operation.worker = Some(worker.clone());
            operation.cancelled.load(Ordering::Acquire)
        };
        if cancelled {
            terminate_worker(&worker);
        }
        Ok(())
    }

    pub fn unregister_worker(&self, operation_id: &str) {
        let mut active = self
            .active_operation
            .lock()
            .expect("active operation lock poisoned");
        if let Some(operation) = active.as_mut() {
            if operation.operation_id == operation_id {
                operation.worker = None;
            }
        }
    }

    pub fn cancel_operation(&self) -> bool {
        let worker = {
            let active = self
                .active_operation
                .lock()
                .expect("active operation lock poisoned");
            let Some(operation) = active.as_ref() else {
                return false;
            };
            operation.cancelled.store(true, Ordering::Release);
            operation.worker.clone()
        };
        if let Some(worker) = worker {
            terminate_worker(&worker);
        }
        let monitor = self.monitor.lock().expect("monitor lock poisoned").take();
        if let Some(monitor) = monitor {
            let _ = monitor.stop_and_wait();
        }
        true
    }

    pub fn install_monitor(&self, handle: MonitorHandle) -> Result<()> {
        let mut monitor = self.monitor.lock().expect("monitor lock poisoned");
        if let Some(previous) = monitor.take() {
            previous.stop_and_wait()?;
        }
        *monitor = Some(handle);
        Ok(())
    }

    pub fn install_user_monitor(&self, open: impl FnOnce() -> Result<MonitorHandle>) -> Result<()> {
        let mut monitor = self.monitor.lock().expect("monitor lock poisoned");
        if self.operation_active.load(Ordering::Acquire) {
            return Err(OperationError::new(
                ErrorCode::OperationInProgress,
                "UART-монитор заблокирован на время прошивки",
            ));
        }
        if let Some(previous) = monitor.take() {
            previous.stop_and_wait()?;
        }
        *monitor = Some(open()?);
        Ok(())
    }

    pub fn send_serial(&self, bytes: Vec<u8>) -> Result<()> {
        let monitor = self.monitor.lock().expect("monitor lock poisoned");
        monitor
            .as_ref()
            .ok_or_else(|| {
                OperationError::new(ErrorCode::DeviceDisconnected, "UART-монитор не подключён")
            })?
            .send(bytes)
    }

    pub fn reset_monitor(&self) -> Result<()> {
        let monitor = self.monitor.lock().expect("monitor lock poisoned");
        if self.operation_active.load(Ordering::Acquire) {
            return Err(OperationError::new(
                ErrorCode::OperationInProgress,
                "Перезапуск заблокирован на время прошивки",
            ));
        }
        monitor
            .as_ref()
            .ok_or_else(|| {
                OperationError::new(ErrorCode::DeviceDisconnected, "UART-монитор не подключён")
            })?
            .reset()
    }

    pub fn disconnect_monitor(&self) -> Result<()> {
        let mut monitor = self.monitor.lock().expect("monitor lock poisoned");
        if self.operation_active.load(Ordering::Acquire) {
            return Err(OperationError::new(
                ErrorCode::OperationInProgress,
                "UART-монитор заблокирован на время прошивки",
            ));
        }
        if let Some(handle) = monitor.take() {
            handle.stop_and_wait()?;
        }
        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OperationLease {
    state: Arc<AppState>,
    operation_id: String,
    cancelled: Arc<AtomicBool>,
}

impl OperationLease {
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn check_cancelled(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(cancelled_error())
        } else {
            Ok(())
        }
    }
}

impl Drop for OperationLease {
    fn drop(&mut self) {
        let worker = {
            let mut active = self
                .state
                .active_operation
                .lock()
                .expect("active operation lock poisoned");
            match active.as_ref() {
                Some(operation) if operation.operation_id == self.operation_id => {
                    active.take().and_then(|operation| operation.worker)
                }
                _ => None,
            }
        };
        if let Some(worker) = worker {
            terminate_worker(&worker);
        }
        self.state.operation_active.store(false, Ordering::Release);
    }
}

pub fn cancelled_error() -> OperationError {
    OperationError::new(
        ErrorCode::OperationCancelled,
        "Операция остановлена пользователем",
    )
    .with_detail("Если остановка выполнена во время записи, содержимое flash может быть неполным")
}

fn terminate_worker(worker: &SharedWorker) {
    let mut child = worker.lock().expect("flash worker lock poisoned");
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        if let Some(operation) = self
            .active_operation
            .get_mut()
            .expect("active operation lock poisoned")
            .take()
        {
            if let Some(worker) = operation.worker {
                terminate_worker(&worker);
            }
        }
    }
}

#[derive(Clone)]
pub struct EventSink {
    operation_id: String,
    callback: Arc<dyn Fn(OperationEvent) + Send + Sync>,
    logger: Arc<OperationLogger>,
}

impl EventSink {
    pub fn new(
        operation_id: String,
        callback: Arc<dyn Fn(OperationEvent) + Send + Sync>,
        logger: OperationLogger,
    ) -> Self {
        Self {
            operation_id,
            callback,
            logger: Arc::new(logger),
        }
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub fn send(&self, event: OperationEvent) {
        (self.callback)(event);
    }

    pub fn state(
        &self,
        state: OperationStage,
        message: impl Into<String>,
        error: Option<OperationError>,
    ) {
        let message = message.into();
        let level = if error.is_some() { "ERROR" } else { "INFO" };
        self.persist_or_report(level, state, &message);
        self.send(OperationEvent::State {
            operation_id: self.operation_id.clone(),
            state,
            message,
            error,
        });
    }

    pub fn log(
        &self,
        stage: OperationStage,
        level: &str,
        message: impl Into<String>,
        error_code: Option<ErrorCode>,
    ) {
        let message = message.into();
        self.persist_or_report(level, stage, &message);
        self.send(OperationEvent::Log {
            operation_id: self.operation_id.clone(),
            entry: OperationLog {
                timestamp: Utc::now().to_rfc3339(),
                level: level.to_string(),
                stage,
                message,
                error_code,
            },
        });
    }

    fn persist_or_report(&self, level: &str, stage: OperationStage, message: &str) {
        if let Err(error) = self.logger.write(level, &format!("{stage:?}"), message) {
            self.send(OperationEvent::Log {
                operation_id: self.operation_id.clone(),
                entry: OperationLog {
                    timestamp: Utc::now().to_rfc3339(),
                    level: "ERROR".to_string(),
                    stage,
                    message: format!("Диагностический лог недоступен: {}", error.message),
                    error_code: Some(error.code),
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use programmer_core::ErrorCode;
    use std::sync::Arc;

    #[test]
    fn cancellation_marks_the_active_lease_and_is_idempotent() {
        let state = Arc::new(AppState::new());
        let lease = state.acquire("operation-1".to_string()).unwrap();

        assert!(state.cancel_operation());
        assert!(state.cancel_operation());
        assert_eq!(
            lease.check_cancelled().unwrap_err().code,
            ErrorCode::OperationCancelled
        );

        drop(lease);
        assert!(!state.cancel_operation());
    }
}
