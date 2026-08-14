use super::WorkerResult;
use crate::application::coordinator::OperationLease;
use crate::application::{AppState, EventSink, OperationEvent, SerialData};
use crate::platform::serial::{start_monitor, MonitorSignal, MonitorStartMode};
use programmer_core::{ErrorCode, OperationError, OperationStage, Result};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const MONITOR_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub fn open_monitor_after_flash(
    state: &Arc<AppState>,
    lease: &OperationLease,
    port: &str,
    worker_result: &WorkerResult,
    sink: &EventSink,
    marker_wait_message: &str,
) -> Result<bool> {
    lease.check_cancelled()?;
    sink.state(
        OperationStage::Resetting,
        "Запуск приложения и UART-монитора",
        None,
    );
    let serial_sink = sink.clone();
    let disconnect_sink = sink.clone();
    let disconnect_port = port.to_string();
    let (monitor, signals) = start_monitor(
        port,
        worker_result.monitor_baud,
        &worker_result.success_marker,
        MonitorStartMode::ResetToNormalBoot,
        Arc::new(move |data: SerialData| {
            serial_sink.send(OperationEvent::Serial {
                operation_id: serial_sink.operation_id().to_string(),
                data,
            });
        }),
        Arc::new(move |message| {
            disconnect_sink.send(OperationEvent::MonitorDisconnected {
                operation_id: disconnect_sink.operation_id().to_string(),
                port: disconnect_port.clone(),
                message,
            });
        }),
    )?;
    state.install_monitor(monitor)?;
    let marker_required = !worker_result.success_marker.is_empty();
    sink.state(
        OperationStage::Monitoring,
        if marker_required {
            marker_wait_message
        } else {
            "UART-монитор открыт; маркер готовности не настроен"
        },
        None,
    );
    if !marker_required {
        return Ok(false);
    }
    wait_for_marker(lease, &signals, worker_result.success_timeout_ms)
}

fn wait_for_marker(
    lease: &OperationLease,
    signals: &mpsc::Receiver<MonitorSignal>,
    timeout_ms: u64,
) -> Result<bool> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        lease.check_cancelled()?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(OperationError::new(
                ErrorCode::BootMarkerTimeout,
                "UART-маркер не получен до истечения таймаута",
            ));
        }
        match signals.recv_timeout(remaining.min(MONITOR_POLL_INTERVAL)) {
            Ok(MonitorSignal::MarkerMatched) => return Ok(true),
            Ok(MonitorSignal::Disconnected(detail)) => {
                return Err(OperationError::new(
                    ErrorCode::DeviceDisconnected,
                    "Плата отключилась до UART-маркера",
                )
                .with_detail(detail));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(OperationError::new(
                    ErrorCode::DeviceDisconnected,
                    "UART-монитор завершился до маркера",
                ));
            }
        }
    }
}
