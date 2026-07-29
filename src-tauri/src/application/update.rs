use crate::application::{
    AppState, EventSink, OperationEvent, OperationResult, SerialData, UpdateRequest,
};
use crate::platform::esp::{EspSession, OwnedFlashSegment};
use crate::platform::logging::OperationLogger;
use crate::platform::package::load_package;
use crate::platform::serial::{start_monitor, MonitorSignal, MonitorStartMode};
use programmer_core::{
    build_otadata_sector, select_update_layout, ErrorCode, OperationError, OperationStage,
    PackageKind, Result, UpdateLayout,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub fn run_update(
    state: Arc<AppState>,
    request: UpdateRequest,
    callback: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationResult> {
    let _lease = state.acquire()?;
    let operation_id = Uuid::new_v4().to_string();
    let logger = OperationLogger::create(&state.data, &operation_id)?;
    let sink = EventSink::new(operation_id.clone(), callback, logger);
    let started = Instant::now();
    match run_update_inner(&state, &request, &sink, started) {
        Ok(result) => Ok(result),
        Err(error) => {
            sink.state(
                OperationStage::Failed,
                error.message.clone(),
                Some(error.clone()),
            );
            Err(error)
        }
    }
}

fn run_update_inner(
    state: &Arc<AppState>,
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
            "В клиентском режиме нужен пакет kind=update",
        ));
    }
    let summary = package.summary();
    let application = package
        .segment_bytes(&package.validated.manifest.segments[0].file)?
        .to_vec();

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
    let mut session = EspSession::connect(&request.port)?;
    let device = session.device().clone();
    let chip = session.chip()?;
    if !package.validated.manifest.target_chips.contains(&chip) {
        return Err(OperationError::new(
            ErrorCode::ChipMismatch,
            "Подключённая ESP32 не подходит для выбранного пакета",
        )
        .with_detail(format!("обнаружен {chip}")));
    }

    let (partition_table_offset, partitions) =
        session.read_partition_table(package.device_partition_table_offset)?;
    sink.log(
        OperationStage::Connecting,
        "INFO",
        format!("Partition table обнаружена по адресу 0x{partition_table_offset:X}"),
        None,
    );
    let ota_data_partition = partitions.iter().find(|entry| entry.is_ota_data());
    let otadata = if let Some(partition) = ota_data_partition {
        Some(session.read_region(partition.offset, 0x2000)?)
    } else {
        None
    };
    let layout = select_update_layout(
        &partitions,
        otadata.as_deref(),
        device.flash_size_bytes,
        application.len() as u64,
        package.validated.manifest.ota.rollback_enabled,
    )?;

    let segments = match layout {
        UpdateLayout::InPlace { target } => {
            if !request.confirm_in_place {
                return Err(OperationError::new(
                    ErrorCode::InPlaceConfirmationRequired,
                    "На устройстве нет резервного OTA-слота",
                )
                .with_detail(format!(
                    "Обновление будет выполнено in-place в раздел {}",
                    target.label
                )));
            }
            sink.log(
                OperationStage::Connecting,
                "WARN",
                format!("In-place update {}: не отключайте питание", target.label),
                None,
            );
            vec![OwnedFlashSegment {
                address: target.offset,
                data: application,
            }]
        }
        UpdateLayout::Ota {
            target,
            ota_data,
            switch,
        } => {
            sink.log(
                OperationStage::Connecting,
                "INFO",
                format!(
                    "Безопасное OTA: запись {} и переключение sequence {}",
                    target.label, switch.target_sequence
                ),
                None,
            );
            let metadata_address = ota_data.offset + (switch.metadata_sector as u32 * 0x1000);
            vec![
                OwnedFlashSegment {
                    address: target.offset,
                    data: application,
                },
                OwnedFlashSegment {
                    address: metadata_address,
                    data: build_otadata_sector(&switch),
                },
            ]
        }
    };

    sink.state(
        OperationStage::Writing,
        "Запись прошивки на скорости 921600",
        None,
    );
    let progress_sink = sink.clone();
    session.write_segments(
        &segments,
        Arc::new(move |progress| {
            progress_sink.send(OperationEvent::Progress {
                operation_id: progress_sink.operation_id().to_string(),
                progress,
            });
        }),
    )?;
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
    let (monitor, signals) = start_monitor(
        &request.port,
        package.validated.manifest.monitor.baud,
        &package.validated.manifest.monitor.success_marker,
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
    let marker_required = !package.validated.manifest.monitor.success_marker.is_empty();
    sink.state(
        OperationStage::Monitoring,
        if marker_required {
            "Ожидание UART-маркера успешного запуска"
        } else {
            "UART-монитор открыт; маркер готовности не настроен"
        },
        None,
    );
    if !marker_required {
        sink.state(
            OperationStage::Passed,
            "Обновление записано и проверено, UART-монитор открыт",
            None,
        );
        return Ok(OperationResult {
            operation_id: sink.operation_id().to_string(),
            success: true,
            boot_confirmed: false,
            duration_ms: started.elapsed().as_millis() as u64,
            device,
            package: summary,
            error: None,
            report_path: None,
        });
    }
    let timeout = Duration::from_millis(package.validated.manifest.monitor.success_timeout_ms);
    let boot_error = match signals.recv_timeout(timeout) {
        Ok(MonitorSignal::MarkerMatched) => None,
        Ok(MonitorSignal::Disconnected(detail)) => Some(
            OperationError::new(
                ErrorCode::DeviceDisconnected,
                "Плата отключилась до подтверждения запуска",
            )
            .with_detail(detail),
        ),
        Err(_) => Some(OperationError::new(
            ErrorCode::BootMarkerTimeout,
            "UART-маркер не получен до истечения таймаута",
        )),
    };
    let boot_confirmed = boot_error.is_none();
    sink.state(
        if boot_confirmed {
            OperationStage::Passed
        } else {
            OperationStage::Failed
        },
        if boot_confirmed {
            "Обновление завершено, приложение запущено"
        } else {
            "Прошивка записана, но запуск не подтверждён"
        },
        boot_error.clone(),
    );

    Ok(OperationResult {
        operation_id: sink.operation_id().to_string(),
        success: boot_confirmed,
        boot_confirmed,
        duration_ms: started.elapsed().as_millis() as u64,
        device,
        package: summary,
        error: boot_error,
        report_path: None,
    })
}
