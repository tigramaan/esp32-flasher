use crate::idf::{parse_idf_build, IdfBuild, IdfSegment};
use programmer_core::{
    validate_package, ChipFamily, ErrorCode, FirmwareManifest, FirmwareSegment, FirmwareSource,
    MonitorPolicy, OperationError, OtaPolicy, PackageKind, PackageReader, PackageSummary, Result,
    SegmentRole, SegmentSummary, MANIFEST_FILE_NAME, MAX_SEGMENT_BYTES,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildKind {
    Factory,
    Update,
}

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub kind: BuildKind,
    pub build_dir: PathBuf,
    pub output_dir: PathBuf,
    pub package_id: String,
    pub display_name: String,
    pub firmware_version: String,
    pub monitor_baud: u32,
    pub success_marker: String,
    pub success_timeout_ms: u64,
    pub rollback_enabled: bool,
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub manifest: FirmwareManifest,
    pub summary: PackageSummary,
    pub output_dir: PathBuf,
    pub dry_run: bool,
}

pub fn build_package(options: &BuildOptions) -> Result<BuildResult> {
    let build = parse_idf_build(&options.build_dir)?;
    ensure_output_is_safe(&options.build_dir, &options.output_dir)?;
    if options.output_dir.exists() && !options.force {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Output-папка уже существует; используйте --force",
        ));
    }

    let planned = plan_manifest(&build, options)?;
    if options.dry_run {
        let summary = PackageSummary {
            package_id: planned.package_id.clone(),
            display_name: planned.display_name.clone(),
            firmware_version: planned.firmware_version.clone(),
            kind: planned.kind,
            target_chips: planned
                .target_chips
                .iter()
                .map(ToString::to_string)
                .collect(),
            segment_count: planned.segments.len(),
            total_bytes: planned.segments.iter().map(|segment| segment.size).sum(),
            monitor_baud: planned.monitor.baud,
            success_timeout_ms: planned.monitor.success_timeout_ms,
            success_marker_configured: !planned.monitor.success_marker.is_empty(),
            source: FirmwareSource::LegacyManifest,
            requires_device_layout: planned.kind == PackageKind::Update,
            segments: planned
                .segments
                .iter()
                .map(|segment| SegmentSummary {
                    role: segment.role,
                    file: segment.file.clone(),
                    offset: segment.offset.map(|value| value.to_string()),
                    size: segment.size,
                })
                .collect(),
        };
        return Ok(BuildResult {
            manifest: planned,
            summary,
            output_dir: options.output_dir.clone(),
            dry_run: true,
        });
    }

    let parent = options.output_dir.parent().ok_or_else(|| {
        OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Output-папка должна иметь родительский каталог",
        )
    })?;
    fs::create_dir_all(parent).map_err(io_error)?;
    let staging = parent.join(format!(".programmer-pack-{}.tmp", Uuid::new_v4()));
    fs::create_dir(&staging).map_err(io_error)?;

    let publish_result = (|| {
        let sources = selected_segments(&build, options.kind);
        for (segment, source) in planned.segments.iter().zip(sources.iter()) {
            copy_bounded(&source.source, &staging.join(&segment.file))?;
        }
        let manifest_bytes = serde_json::to_vec_pretty(&planned).map_err(|error| {
            OperationError::new(
                ErrorCode::InternalError,
                "Не удалось сериализовать manifest",
            )
            .with_detail(error.to_string())
        })?;
        let mut file = fs::File::create(staging.join(MANIFEST_FILE_NAME)).map_err(io_error)?;
        file.write_all(&manifest_bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        drop(file);

        let validated = validate_directory(&staging)?;
        let backup = if options.output_dir.exists() {
            let backup = parent.join(format!(".programmer-pack-{}.backup", Uuid::new_v4()));
            fs::rename(&options.output_dir, &backup).map_err(io_error)?;
            Some(backup)
        } else {
            None
        };
        if let Err(error) = fs::rename(&staging, &options.output_dir) {
            if let Some(backup) = &backup {
                if let Err(restore_error) = fs::rename(backup, &options.output_dir) {
                    return Err(OperationError::new(
                        ErrorCode::IoError,
                        "Не удалось опубликовать пакет и восстановить предыдущую папку",
                    )
                    .with_detail(format!(
                        "publish: {error}; restore: {restore_error}; backup: {}",
                        backup.display()
                    )));
                }
            }
            return Err(io_error(error));
        }
        if let Some(backup) = backup {
            fs::remove_dir_all(backup).map_err(io_error)?;
        }
        Ok(validated)
    })();

    let validated = match publish_result {
        Ok(validated) => validated,
        Err(mut error) => {
            if staging.exists() {
                if let Err(cleanup_error) = fs::remove_dir_all(&staging) {
                    let prior = error.detail.take().unwrap_or_default();
                    error.detail = Some(format!(
                        "{prior}; staging cleanup failed: {cleanup_error}; path: {}",
                        staging.display()
                    ));
                }
            }
            return Err(error);
        }
    };
    Ok(BuildResult {
        summary: PackageSummary::from(&validated),
        manifest: validated.manifest,
        output_dir: options.output_dir.clone(),
        dry_run: false,
    })
}

pub fn validate_directory(path: &Path) -> Result<programmer_core::ValidatedPackage> {
    let root = path.canonicalize().map_err(|error| {
        OperationError::new(ErrorCode::PackagePathInvalid, "Папка пакета недоступна")
            .with_detail(error.to_string())
    })?;
    if !root.is_dir() {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Путь пакета не является папкой",
        ));
    }
    let manifest_bytes = fs::read(root.join(MANIFEST_FILE_NAME)).map_err(|error| {
        OperationError::new(ErrorCode::PackageFileMissing, "В папке нет firmware.json")
            .with_detail(error.to_string())
    })?;
    validate_package(&manifest_bytes, &FsPackageReader { root })
}

fn plan_manifest(build: &IdfBuild, options: &BuildOptions) -> Result<FirmwareManifest> {
    let sources = selected_segments(build, options.kind);
    let mut segments = Vec::with_capacity(sources.len());
    for (index, source) in sources.iter().enumerate() {
        let bytes = fs::read(&source.source).map_err(io_error)?;
        if bytes.is_empty() || bytes.len() as u64 > MAX_SEGMENT_BYTES {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "BIN пуст или превышает допустимый размер",
            )
            .with_detail(source.source.display().to_string()));
        }
        let file = output_name(source.role, index, options.kind);
        segments.push(FirmwareSegment {
            role: source.role,
            file,
            offset: if options.kind == BuildKind::Factory {
                Some(source.offset)
            } else {
                None
            },
            size: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        });
    }
    let manifest = FirmwareManifest {
        schema_version: 1,
        package_id: options.package_id.clone(),
        display_name: options.display_name.clone(),
        firmware_version: options.firmware_version.clone(),
        kind: match options.kind {
            BuildKind::Factory => PackageKind::Factory,
            BuildKind::Update => PackageKind::Update,
        },
        target_chips: vec![ChipFamily::try_from(build.chip.as_str())?],
        partition_table_offset: build.partition_table_offset,
        monitor: MonitorPolicy {
            baud: options.monitor_baud,
            success_marker: options.success_marker.clone(),
            success_timeout_ms: options.success_timeout_ms,
        },
        ota: OtaPolicy {
            rollback_enabled: options.rollback_enabled,
        },
        segments,
    };
    manifest.validate_structure()?;
    Ok(manifest)
}

fn selected_segments(build: &IdfBuild, kind: BuildKind) -> Vec<&IdfSegment> {
    match kind {
        BuildKind::Factory => build.factory_segments.iter().collect(),
        BuildKind::Update => vec![&build.application],
    }
}

fn output_name(role: SegmentRole, index: usize, kind: BuildKind) -> String {
    if kind == BuildKind::Update {
        return "application.bin".to_string();
    }
    let role = match role {
        SegmentRole::Bootloader => "bootloader",
        SegmentRole::PartitionTable => "partition-table",
        SegmentRole::Application => "application",
        SegmentRole::OtaData => "ota-data",
        SegmentRole::Data => "data",
    };
    format!("{index:02}-{role}.bin")
}

fn ensure_output_is_safe(build: &Path, output: &Path) -> Result<()> {
    let build = build.canonicalize().map_err(io_error)?;
    let output_abs = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir().map_err(io_error)?.join(output)
    };
    if output_abs == build || build.starts_with(&output_abs) {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Output не может совпадать с build-папкой или быть её родителем",
        ));
    }
    Ok(())
}

fn copy_bounded(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source).map_err(io_error)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Источник BIN должен быть обычным файлом",
        ));
    }
    if metadata.len() == 0 || metadata.len() > MAX_SEGMENT_BYTES {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "Недопустимый размер BIN",
        ));
    }
    fs::copy(source, destination).map_err(io_error)?;
    Ok(())
}

struct FsPackageReader {
    root: PathBuf,
}

impl PackageReader for FsPackageReader {
    fn read_file(&self, relative_path: &str, max_bytes: u64) -> Result<Vec<u8>> {
        let path = self.root.join(relative_path);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            OperationError::new(ErrorCode::PackageFileMissing, "BIN пакета не найден")
                .with_detail(error.to_string())
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(OperationError::new(
                ErrorCode::PackagePathInvalid,
                "BIN пакета должен быть обычным файлом",
            ));
        }
        if metadata.len() > max_bytes {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "BIN пакета превышает допустимый размер",
            ));
        }
        fs::read(path).map_err(io_error)
    }
}

fn io_error(error: impl std::fmt::Display) -> OperationError {
    OperationError::new(ErrorCode::IoError, "Ошибка файловой системы")
        .with_detail(error.to_string())
}
