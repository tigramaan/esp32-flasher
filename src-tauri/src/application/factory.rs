use crate::application::worker::{execute_supervised, open_monitor_after_flash, WorkerRequest};
use crate::application::{
    AppState, DetectedDevice, EventSink, FactoryFlashRequest, OperationEvent, OperationResult,
};
use crate::platform::logging::OperationLogger;
use crate::platform::package::load_package;
use programmer_core::{
    ErrorCode, MarkerDetector, OperationError, OperationStage, PackageKind, Result,
};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

pub fn run_factory_flash(
    state: Arc<AppState>,
    request: FactoryFlashRequest,
    callback: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationResult> {
    let operation_id = Uuid::new_v4().to_string();
    let lease = state.acquire(operation_id.clone())?;
    let logger = OperationLogger::create(&state.data, &operation_id)?;
    let sink = EventSink::new(operation_id, callback, logger);
    let started = Instant::now();

    sink.state(OperationStage::Validating, "Проверка factory-пакета", None);
    let package = load_package(&request.package_path)?;
    if package.validated.manifest.kind != PackageKind::Factory {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "В производственном режиме нужен factory-пакет",
        ));
    }
    let marker = match package.source {
        programmer_core::FirmwareSource::Platformio => request.success_marker.as_str(),
        _ => package.validated.manifest.monitor.success_marker.as_str(),
    };
    if !marker.is_empty() {
        MarkerDetector::new(marker.as_bytes())?;
    }
    let summary = package.summary();
    let session_summary = state
        .reports
        .ensure_matching_session(&state.data, &summary)?;
    let placeholder = placeholder_device(&request.port);

    sink.state(
        OperationStage::Detecting,
        "Определение подключённой ESP32",
        None,
    );
    let outcome = execute_supervised(
        &state,
        &lease,
        WorkerRequest::Factory {
            request: request.clone(),
        },
        &sink,
    )
    .and_then(|worker_result| {
        let device = worker_result.device.clone();
        open_monitor_after_flash(
            &state,
            &lease,
            &request.port,
            &worker_result,
            &sink,
            "Ожидание производственного UART-маркера",
        )
        .map(|boot_confirmed| (device, boot_confirmed))
    });

    let duration_ms = started.elapsed().as_millis() as u64;
    let mut result = match outcome {
        Ok((device, boot_confirmed)) => {
            state
                .reports
                .record(&summary, &device, duration_ms, request.full_erase, None)?;
            sink.state(
                OperationStage::Passed,
                if boot_confirmed {
                    "Плата прошита и подтвердила запуск"
                } else {
                    "Плата прошита и проверена, UART-монитор открыт"
                },
                None,
            );
            OperationResult {
                operation_id: sink.operation_id().to_string(),
                success: true,
                boot_confirmed,
                duration_ms,
                device,
                package: summary,
                error: None,
                report_path: Some(session_summary.report_path.clone()),
            }
        }
        Err(error) => {
            let stage = if error.code == ErrorCode::OperationCancelled {
                OperationStage::Cancelled
            } else {
                OperationStage::Failed
            };
            state.reports.record(
                &summary,
                &placeholder,
                duration_ms,
                request.full_erase,
                Some(&error),
            )?;
            sink.state(stage, error.message.clone(), Some(error.clone()));
            OperationResult {
                operation_id: sink.operation_id().to_string(),
                success: false,
                boot_confirmed: false,
                duration_ms,
                device: placeholder,
                package: summary,
                error: Some(error),
                report_path: Some(session_summary.report_path.clone()),
            }
        }
    };
    result.report_path = Some(session_summary.report_path);
    Ok(result)
}

fn placeholder_device(port: &str) -> DetectedDevice {
    DetectedDevice {
        port: port.to_string(),
        description: "ESP32".to_string(),
        chip: "не определён".to_string(),
        mac: "недоступен".to_string(),
        flash_size_bytes: 0,
    }
}
