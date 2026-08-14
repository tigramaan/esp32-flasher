use super::{emit, WorkerMessage, WorkerResult};
use crate::application::{FactoryFlashRequest, UpdateRequest};
use crate::platform::esp::{EspSession, OwnedFlashSegment};
use crate::platform::package::load_package;
use programmer_core::{
    build_otadata_sector, select_update_layout, ChipFamily, ErrorCode, OperationError,
    OperationStage, PackageKind, Result, UpdateLayout,
};
use std::sync::Arc;

pub(super) fn run_factory(
    request: FactoryFlashRequest,
    connect_attempt: usize,
) -> Result<WorkerResult> {
    let package = load_package(&request.package_path)?;
    if package.validated.manifest.kind != PackageKind::Factory {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "В производственном режиме нужен factory-пакет",
        ));
    }
    let diagnostic = Arc::new(|message: &str| {
        emit(&WorkerMessage::Log {
            stage: OperationStage::Connecting,
            message: message.to_string(),
        });
    });
    let mut session =
        EspSession::connect_with_diagnostics(&request.port, connect_attempt, diagnostic)?;
    let device = session.device().clone();
    ensure_chip(&package.validated.manifest.target_chips, session.chip()?)?;
    let mut segments = Vec::with_capacity(package.validated.manifest.segments.len());
    for segment in &package.validated.manifest.segments {
        let address = segment.offset.expect("factory offsets validated").0;
        if u64::from(address) + segment.size > device.flash_size_bytes {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "Factory-сегмент выходит за размер flash устройства",
            )
            .with_detail(segment.file.clone()));
        }
        segments.push(OwnedFlashSegment {
            address,
            data: package.segment_bytes(&segment.file)?.to_vec(),
        });
    }
    if request.full_erase {
        emit(&WorkerMessage::MutationStarted);
        emit(&WorkerMessage::Stage {
            stage: OperationStage::Erasing,
            message: "Полное стирание flash — не отключайте питание".to_string(),
        });
        session.erase_all()?;
    }
    emit(&WorkerMessage::MutationStarted);
    emit(&WorkerMessage::Stage {
        stage: OperationStage::Writing,
        message: "Запись factory-пакета на скорости 921600".to_string(),
    });
    session.write_segments(
        &segments,
        Arc::new(|progress| emit(&WorkerMessage::Progress { progress })),
    )?;
    emit(&WorkerMessage::Stage {
        stage: OperationStage::Verifying,
        message: "Flash verify завершён".to_string(),
    });
    let success_marker = match package.source {
        programmer_core::FirmwareSource::Platformio => request.success_marker,
        _ => package.validated.manifest.monitor.success_marker.clone(),
    };
    Ok(WorkerResult {
        device: session.close_for_monitor(),
        monitor_baud: package.validated.manifest.monitor.baud,
        success_marker,
        success_timeout_ms: package.validated.manifest.monitor.success_timeout_ms,
    })
}

pub(super) fn run_update(request: UpdateRequest, connect_attempt: usize) -> Result<WorkerResult> {
    let package = load_package(&request.package_path)?;
    if package.validated.manifest.kind != PackageKind::Update {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "В клиентском режиме нужен update-пакет",
        ));
    }
    let application = package
        .segment_bytes(&package.validated.manifest.segments[0].file)?
        .to_vec();
    let diagnostic = Arc::new(|message: &str| {
        emit(&WorkerMessage::Log {
            stage: OperationStage::Connecting,
            message: message.to_string(),
        });
    });
    let mut session =
        EspSession::connect_with_diagnostics(&request.port, connect_attempt, diagnostic)?;
    let device = session.device().clone();
    ensure_chip(&package.validated.manifest.target_chips, session.chip()?)?;
    let (table_offset, partitions) =
        session.read_partition_table(package.device_partition_table_offset)?;
    emit(&WorkerMessage::Log {
        stage: OperationStage::Connecting,
        message: format!("Partition table обнаружена по адресу 0x{table_offset:X}"),
    });
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
            vec![OwnedFlashSegment {
                address: target.offset,
                data: application,
            }]
        }
        UpdateLayout::Ota {
            target,
            ota_data,
            switch,
        } => vec![
            OwnedFlashSegment {
                address: target.offset,
                data: application,
            },
            OwnedFlashSegment {
                address: ota_data.offset + (switch.metadata_sector as u32 * 0x1000),
                data: build_otadata_sector(&switch),
            },
        ],
    };
    emit(&WorkerMessage::MutationStarted);
    emit(&WorkerMessage::Stage {
        stage: OperationStage::Writing,
        message: "Запись прошивки на скорости 921600".to_string(),
    });
    session.write_segments(
        &segments,
        Arc::new(|progress| emit(&WorkerMessage::Progress { progress })),
    )?;
    emit(&WorkerMessage::Stage {
        stage: OperationStage::Verifying,
        message: "Flash verify завершён".to_string(),
    });
    Ok(WorkerResult {
        device: session.close_for_monitor(),
        monitor_baud: package.validated.manifest.monitor.baud,
        success_marker: package.validated.manifest.monitor.success_marker.clone(),
        success_timeout_ms: package.validated.manifest.monitor.success_timeout_ms,
    })
}

fn ensure_chip(allowed: &[ChipFamily], actual: ChipFamily) -> Result<()> {
    if allowed.contains(&actual) {
        Ok(())
    } else {
        Err(OperationError::new(
            ErrorCode::ChipMismatch,
            "Подключённая ESP32 не подходит для выбранной прошивки",
        )
        .with_detail(format!("обнаружен {actual}")))
    }
}
