use crate::error::{ErrorCode, OperationError, Result};
use crate::manifest::{FirmwareManifest, PackageKind, SegmentRole};
use crate::validate_esp_image;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};

pub const MAX_SEGMENT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_PACKAGE_BYTES: u64 = 256 * 1024 * 1024;

pub trait PackageReader {
    fn read_file(&self, relative_path: &str, max_bytes: u64) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPackage {
    pub manifest: FirmwareManifest,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentSummary {
    pub role: SegmentRole,
    pub file: String,
    pub offset: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareSource {
    Platformio,
    Standalone,
    LegacyManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageSummary {
    pub package_id: String,
    pub display_name: String,
    pub firmware_version: String,
    pub kind: PackageKind,
    pub target_chips: Vec<String>,
    pub segment_count: usize,
    pub total_bytes: u64,
    pub monitor_baud: u32,
    pub success_timeout_ms: u64,
    pub success_marker_configured: bool,
    pub source: FirmwareSource,
    pub requires_device_layout: bool,
    pub segments: Vec<SegmentSummary>,
}

impl From<&ValidatedPackage> for PackageSummary {
    fn from(value: &ValidatedPackage) -> Self {
        Self {
            package_id: value.manifest.package_id.clone(),
            display_name: value.manifest.display_name.clone(),
            firmware_version: value.manifest.firmware_version.clone(),
            kind: value.manifest.kind,
            target_chips: value
                .manifest
                .target_chips
                .iter()
                .map(ToString::to_string)
                .collect(),
            segment_count: value.manifest.segments.len(),
            total_bytes: value.total_bytes,
            monitor_baud: value.manifest.monitor.baud,
            success_timeout_ms: value.manifest.monitor.success_timeout_ms,
            success_marker_configured: !value.manifest.monitor.success_marker.is_empty(),
            source: FirmwareSource::LegacyManifest,
            requires_device_layout: value.manifest.kind == PackageKind::Update,
            segments: value
                .manifest
                .segments
                .iter()
                .map(|segment| SegmentSummary {
                    role: segment.role,
                    file: segment.file.clone(),
                    offset: segment.offset.map(|value| value.to_string()),
                    size: segment.size,
                })
                .collect(),
        }
    }
}

pub fn validate_package(
    manifest_bytes: &[u8],
    reader: &impl PackageReader,
) -> Result<ValidatedPackage> {
    let manifest = FirmwareManifest::from_json(manifest_bytes)?;
    let mut total_bytes = 0_u64;

    for segment in &manifest.segments {
        validate_relative_path(&segment.file)?;
        if segment.size > MAX_SEGMENT_BYTES {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "Сегмент превышает допустимый размер",
            )
            .with_detail(segment.file.clone()));
        }
        total_bytes = total_bytes
            .checked_add(segment.size)
            .filter(|value| *value <= MAX_PACKAGE_BYTES)
            .ok_or_else(|| {
                OperationError::new(
                    ErrorCode::PackageInvalid,
                    "Суммарный размер пакета превышает допустимый",
                )
            })?;

        let bytes = reader.read_file(&segment.file, MAX_SEGMENT_BYTES)?;
        if bytes.len() as u64 != segment.size {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "Размер BIN не совпадает с firmware.json",
            )
            .with_detail(segment.file.clone()));
        }
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != segment.sha256 {
            return Err(OperationError::new(
                ErrorCode::HashMismatch,
                "SHA-256 BIN не совпадает с firmware.json",
            )
            .with_detail(segment.file.clone()));
        }
        if segment.role == SegmentRole::Application {
            validate_esp_image(&bytes, &manifest.target_chips)?;
        }
    }

    Ok(ValidatedPackage {
        manifest,
        total_bytes,
    })
}

pub fn validate_relative_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains('\\')
        || value.contains(':')
        || value.contains('\0')
        || Path::new(value).is_absolute()
    {
        return Err(invalid_path(value));
    }
    if Path::new(value)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(invalid_path(value));
    }
    Ok(())
}

fn invalid_path(value: &str) -> OperationError {
    OperationError::new(
        ErrorCode::PackagePathInvalid,
        "Путь BIN должен находиться внутри папки пакета",
    )
    .with_detail(value)
}
