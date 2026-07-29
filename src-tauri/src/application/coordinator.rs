use crate::application::{OperationEvent, OperationLog};
use crate::platform::logging::OperationLogger;
use crate::platform::reports::FactoryReporter;
use crate::platform::serial::MonitorHandle;
use crate::platform::storage::PortableDataStore;
use chrono::Utc;
use programmer_core::{ErrorCode, OperationError, OperationStage, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub data: PortableDataStore,
    pub reports: FactoryReporter,
    operation_active: AtomicBool,
    monitor: Mutex<Option<MonitorHandle>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            data: PortableDataStore::discover(),
            reports: FactoryReporter::new(),
            operation_active: AtomicBool::new(false),
            monitor: Mutex::new(None),
        }
    }

    pub fn acquire(self: &Arc<Self>) -> Result<OperationLease> {
        let mut monitor = self.monitor.lock().expect("monitor lock poisoned");
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
        let lease = OperationLease {
            state: self.clone(),
        };
        if let Some(previous) = monitor.take() {
            previous.stop_and_wait()?;
        }
        Ok(lease)
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
}

impl Drop for OperationLease {
    fn drop(&mut self) {
        self.state.operation_active.store(false, Ordering::Release);
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
