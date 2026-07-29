use crate::application::{
    AppState, DetectedDevice, EventSink, FactoryFlashRequest, OperationEvent, OperationResult,
    SerialData,
};
use crate::platform::esp::{EspSession, OwnedFlashSegment};
use crate::platform::logging::OperationLogger;
use crate::platform::package::{load_package, LoadedPackage};
use crate::platform::serial::{start_monitor, MonitorSignal, MonitorStartMode};
use programmer_core::{
    ErrorCode, FirmwareSource, MarkerDetector, OperationError, OperationStage, PackageKind,
    PackageSummary, Result,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

type FactoryFailure = Box<(Option<DetectedDevice>, OperationError)>;

pub fn run_factory_flash(
    state: Arc<AppState>,
    request: FactoryFlashRequest,
    callback: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationResult> {
    let _lease = state.acquire()?;
    let operation_id = Uuid::new_v4().to_string();
    let logger = OperationLogger::create(&state.data, &operation_id)?;
    let sink = EventSink::new(operation_id, callback, logger);
    let started = Instant::now();

    sink.state(OperationStage::Validating, "Проверка factory-пакета", None);
    let package = load_package(&request.package_path)?;
    if package.validated.manifest.kind != PackageKind::Factory {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "В производственном режиме нужен пакет kind=factory",
        ));
    }
    let success_marker = effective_marker(&package, &request);
    if !success_marker.is_empty() {
        MarkerDetector::new(success_marker.as_bytes())?;
    }
    let summary = package.summary();
    let session_summary = state
        .reports
        .ensure_matching_session(&state.data, &summary)?;
    let placeholder = DetectedDevice {
        port: request.port.clone(),
        description: "ESP32".to_string(),
        chip: "не определён".to_string(),
        mac: "недоступен".to_string(),
        flash_size_bytes: 0,
    };

    let outcome = execute_factory(&state, &request, &package, &summary, &sink);
    let duration_ms = started.elapsed().as_millis() as u64;
    match outcome {
        Ok((device, boot_confirmed)) => {
            let report =
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
            Ok(OperationResult {
                operation_id: sink.operation_id().to_string(),
                success: true,
                boot_confirmed,
                duration_ms,
                device,
                package: summary,
                error: None,
                report_path: Some(report.report_path),
            })
        }
        Err(failure) => {
            let (device, error) = *failure;
            let report = state.reports.record(
                &summary,
                device.as_ref().unwrap_or(&placeholder),
                duration_ms,
                request.full_erase,
                Some(&error),
            )?;
            sink.state(
                OperationStage::Failed,
                error.message.clone(),
                Some(error.clone()),
            );
            Ok(OperationResult {
                operation_id: sink.operation_id().to_string(),
                success: false,
                boot_confirmed: false,
                duration_ms,
                device: device.unwrap_or(placeholder),
                package: summary,
                error: Some(error),
                report_path: Some(report.report_path),
            })
        }
    }
    .map(|mut result| {
        result.report_path = Some(session_summary.report_path);
        result
    })
}

fn execute_factory(
    state: &Arc<AppState>,
    request: &FactoryFlashRequest,
    package: &LoadedPackage,
    _summary: &PackageSummary,
    sink: &EventSink,
) -> std::result::Result<(DetectedDevice, bool), FactoryFailure> {
    sink.state(
        OperationStage::Detecting,
        "Определение подключённой ESP32",
        None,
    );
    sink.state(
        OperationStage::Connecting,
        "Подключение к ROM bootloader",
        None,
    );
    let mut session =
        EspSession::connect(&request.port).map_err(|error| Box::new((None, error)))?;
    let device = session.device().clone();
    let chip = session
        .chip()
        .map_err(|error| Box::new((Some(device.clone()), error)))?;
    if !package.validated.manifest.target_chips.contains(&chip) {
        return Err(Box::new((
            Some(device),
            OperationError::new(
                ErrorCode::ChipMismatch,
                "Подключённая ESP32 не подходит для factory-пакета",
            )
            .with_detail(format!("обнаружен {chip}")),
        )));
    }

    let mut segments = Vec::with_capacity(package.validated.manifest.segments.len());
    for segment in &package.validated.manifest.segments {
        let address = segment.offset.expect("factory offsets validated").0;
        let end = u64::from(address) + segment.size;
        if end > device.flash_size_bytes {
            return Err(Box::new((
                Some(device),
                OperationError::new(
                    ErrorCode::PackageInvalid,
                    "Factory-сегмент выходит за размер flash устройства",
                )
                .with_detail(segment.file.clone()),
            )));
        }
        let data = package
            .segment_bytes(&segment.file)
            .map_err(|error| Box::new((Some(device.clone()), error)))?
            .to_vec();
        segments.push(OwnedFlashSegment { address, data });
    }

    if request.full_erase {
        sink.state(
            OperationStage::Erasing,
            "Полное стирание flash — не отключайте питание",
            None,
        );
        session
            .erase_all()
            .map_err(|error| Box::new((Some(device.clone()), error)))?;
    }
    sink.state(
        OperationStage::Writing,
        "Запись factory-пакета на скорости 921600",
        None,
    );
    let progress_sink = sink.clone();
    session
        .write_segments(
            &segments,
            Arc::new(move |progress| {
                progress_sink.send(OperationEvent::Progress {
                    operation_id: progress_sink.operation_id().to_string(),
                    progress,
                });
            }),
        )
        .map_err(|error| Box::new((Some(device.clone()), error)))?;
    sink.state(OperationStage::Verifying, "Flash verify завершён", None);
    drop(session);

    sink.state(
        OperationStage::Resetting,
        "Запуск приложения и UART-монитора",
        None,
    );
    let serial_sink = sink.clone();
    let disconnect_sink = sink.clone();
    let disconnect_port = request.port.clone();
    let success_marker = effective_marker(package, request);
    let (monitor, signals) = start_monitor(
        &request.port,
        package.validated.manifest.monitor.baud,
        success_marker,
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
    )
    .map_err(|error| Box::new((Some(device.clone()), error)))?;
    state
        .install_monitor(monitor)
        .map_err(|error| Box::new((Some(device.clone()), error)))?;
    let marker_required = !success_marker.is_empty();
    sink.state(
        OperationStage::Monitoring,
        if marker_required {
            "Ожидание производственного UART-маркера"
        } else {
            "UART-монитор открыт; маркер готовности не настроен"
        },
        None,
    );
    if !marker_required {
        return Ok((device, false));
    }
    let timeout = Duration::from_millis(package.validated.manifest.monitor.success_timeout_ms);
    match signals.recv_timeout(timeout) {
        Ok(MonitorSignal::MarkerMatched) => Ok((device, true)),
        Ok(MonitorSignal::Disconnected(detail)) => Err(Box::new((
            Some(device),
            OperationError::new(
                ErrorCode::DeviceDisconnected,
                "Плата отключилась до UART-маркера",
            )
            .with_detail(detail),
        ))),
        Err(_) => Err(Box::new((
            Some(device),
            OperationError::new(
                ErrorCode::BootMarkerTimeout,
                "UART-маркер не получен до истечения таймаута",
            ),
        ))),
    }
}

fn effective_marker<'a>(package: &'a LoadedPackage, request: &'a FactoryFlashRequest) -> &'a str {
    select_success_marker(
        package.source,
        package.validated.manifest.monitor.success_marker.as_str(),
        request.success_marker.as_str(),
    )
}

fn select_success_marker<'a>(
    source: FirmwareSource,
    manifest_marker: &'a str,
    requested_marker: &'a str,
) -> &'a str {
    match source {
        FirmwareSource::Platformio => requested_marker,
        FirmwareSource::Standalone | FirmwareSource::LegacyManifest => manifest_marker,
    }
}

#[cfg(test)]
mod tests {
    use super::select_success_marker;
    use programmer_core::FirmwareSource;

    #[test]
    fn direct_factory_uses_operator_marker_and_legacy_keeps_manifest_policy() {
        assert_eq!(
            select_success_marker(FirmwareSource::Platformio, "", "NOVA_READY"),
            "NOVA_READY"
        );
        assert_eq!(
            select_success_marker(FirmwareSource::LegacyManifest, "LEGACY_READY", "IGNORED"),
            "LEGACY_READY"
        );
    }
}
