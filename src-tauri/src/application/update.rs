use crate::application::worker::{execute_supervised, open_monitor_after_flash, WorkerRequest};
use crate::application::{AppState, EventSink, OperationEvent, OperationResult, UpdateRequest};
use crate::platform::logging::OperationLogger;
use crate::platform::package::load_package;
use programmer_core::{ErrorCode, OperationError, OperationStage, PackageKind, Result};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

pub fn run_update(
    state: Arc<AppState>,
    request: UpdateRequest,
    callback: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationResult> {
    let operation_id = Uuid::new_v4().to_string();
    let lease = state.acquire(operation_id.clone())?;
    let logger = OperationLogger::create(&state.data, &operation_id)?;
    let sink = EventSink::new(operation_id, callback, logger);
    let started = Instant::now();
    let outcome = run_update_inner(&state, &lease, &request, &sink, started);
    if let Err(error) = &outcome {
        let stage = if error.code == ErrorCode::OperationCancelled {
            OperationStage::Cancelled
        } else {
            OperationStage::Failed
        };
        sink.state(stage, error.message.clone(), Some(error.clone()));
    }
    outcome
}

fn run_update_inner(
    state: &Arc<AppState>,
    lease: &super::coordinator::OperationLease,
    request: &UpdateRequest,
    sink: &EventSink,
    started: Instant,
) -> Result<OperationResult> {
    sink.state(
        OperationStage::Validating,
        "Проверка пакета обновления",
        None,
    );
    let package = load_package(&request.package_path)?;
    if package.validated.manifest.kind != PackageKind::Update {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "В клиентском режиме нужен update-пакет",
        ));
    }
    let summary = package.summary();
    sink.state(
        OperationStage::Detecting,
        "Определение подключённой ESP32",
        None,
    );
    let worker_result = execute_supervised(
        state,
        lease,
        WorkerRequest::Update {
            request: request.clone(),
        },
        sink,
    )?;
    let device = worker_result.device.clone();
    let boot_confirmed = open_monitor_after_flash(
        state,
        lease,
        &request.port,
        &worker_result,
        sink,
        "Ожидание UART-маркера успешного запуска",
    )?;
    sink.state(
        OperationStage::Passed,
        if boot_confirmed {
            "Обновление завершено, приложение запущено"
        } else {
            "Обновление записано и проверено, UART-монитор открыт"
        },
        None,
    );
    Ok(OperationResult {
        operation_id: sink.operation_id().to_string(),
        success: true,
        boot_confirmed,
        duration_ms: started.elapsed().as_millis() as u64,
        device,
        package: summary,
        error: None,
        report_path: None,
    })
}
