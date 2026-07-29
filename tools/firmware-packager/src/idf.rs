use programmer_core::{ErrorCode, HexAddress, OperationError, Result, SegmentRole};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct IdfSegment {
    pub role: SegmentRole,
    pub offset: HexAddress,
    pub source: PathBuf,
}

#[derive(Debug, Clone)]
pub struct IdfBuild {
    pub chip: String,
    pub partition_table_offset: HexAddress,
    pub factory_segments: Vec<IdfSegment>,
    pub application: IdfSegment,
}

#[derive(Debug, Deserialize)]
struct FlasherArgs {
    #[serde(default)]
    flash_files: HashMap<String, String>,
    #[serde(default)]
    app: Option<FlasherSegment>,
    #[serde(default)]
    bootloader: Option<FlasherSegment>,
    #[serde(rename = "partition-table", alias = "partition_table", default)]
    partition_table: Option<FlasherSegment>,
    #[serde(default)]
    otadata: Option<FlasherSegment>,
    #[serde(default)]
    extra_esptool_args: Option<ExtraArgs>,
    #[serde(default)]
    flash_settings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct FlasherSegment {
    offset: String,
    file: String,
    #[serde(default)]
    encrypted: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ExtraArgs {
    #[serde(default)]
    chip: Option<String>,
    #[serde(flatten)]
    ignored: HashMap<String, serde_json::Value>,
}

pub fn parse_idf_build(build_dir: &Path) -> Result<IdfBuild> {
    let root = canonical_directory(build_dir)?;
    let args_path = root.join("flasher_args.json");
    let bytes = fs::read(&args_path).map_err(|error| {
        OperationError::new(
            ErrorCode::PackageFileMissing,
            "В ESP-IDF build-папке нет flasher_args.json",
        )
        .with_detail(error.to_string())
    })?;
    let args: FlasherArgs = serde_json::from_slice(&bytes).map_err(|error| {
        OperationError::new(
            ErrorCode::PackageInvalid,
            "flasher_args.json имеет неверный формат",
        )
        .with_detail(error.to_string())
    })?;
    let _ = &args.flash_settings;
    if let Some(extra) = &args.extra_esptool_args {
        let _ = &extra.ignored;
    }

    let application_raw = args.app.ok_or_else(|| {
        OperationError::new(
            ErrorCode::PackageInvalid,
            "flasher_args.json не содержит application",
        )
    })?;
    let partition_raw = args.partition_table.ok_or_else(|| {
        OperationError::new(
            ErrorCode::PackageInvalid,
            "flasher_args.json не содержит partition-table",
        )
    })?;
    reject_encryption(&application_raw)?;
    reject_encryption(&partition_raw)?;

    let application = to_segment(&root, SegmentRole::Application, &application_raw)?;
    let partition_table = to_segment(&root, SegmentRole::PartitionTable, &partition_raw)?;
    let partition_table_offset = partition_table.offset;
    let application_bytes = fs::read(&application.source).map_err(|error| {
        OperationError::new(
            ErrorCode::PackageFileMissing,
            "Application BIN из build-папки недоступен",
        )
        .with_detail(error.to_string())
    })?;
    let supported_chips = [
        programmer_core::ChipFamily::Esp32,
        programmer_core::ChipFamily::Esp32c2,
        programmer_core::ChipFamily::Esp32c3,
        programmer_core::ChipFamily::Esp32c5,
        programmer_core::ChipFamily::Esp32c6,
        programmer_core::ChipFamily::Esp32h2,
        programmer_core::ChipFamily::Esp32p4,
        programmer_core::ChipFamily::Esp32s2,
        programmer_core::ChipFamily::Esp32s3,
    ];
    let image_chip = programmer_core::validate_esp_image(&application_bytes, &supported_chips)?;
    if let Some(declared) = args
        .extra_esptool_args
        .as_ref()
        .and_then(|extra| extra.chip.as_deref())
        .filter(|value| !value.trim().is_empty())
    {
        let declared = programmer_core::ChipFamily::try_from(declared)?;
        if declared != image_chip {
            return Err(OperationError::new(
                ErrorCode::ChipMismatch,
                "Target chip в flasher_args.json не совпадает с application BIN",
            )
            .with_detail(format!("metadata={declared}, image={image_chip}")));
        }
    }
    let chip = image_chip.to_string();

    let mut segments = Vec::new();
    if let Some(raw) = &args.bootloader {
        reject_encryption(raw)?;
        segments.push(to_segment(&root, SegmentRole::Bootloader, raw)?);
    }
    segments.push(partition_table.clone());
    if let Some(raw) = &args.otadata {
        reject_encryption(raw)?;
        segments.push(to_segment(&root, SegmentRole::OtaData, raw)?);
    }
    segments.push(application.clone());

    let named_files: Vec<PathBuf> = segments.iter().map(|item| item.source.clone()).collect();
    for (offset, relative) in args.flash_files {
        let source = checked_source(&root, &relative)?;
        if named_files.contains(&source) {
            continue;
        }
        segments.push(IdfSegment {
            role: SegmentRole::Data,
            offset: parse_offset(&offset)?,
            source,
        });
    }
    segments.sort_unstable_by_key(|segment| segment.offset.0);
    if segments.is_empty() {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "ESP-IDF build не содержит flash segments",
        ));
    }

    Ok(IdfBuild {
        chip,
        partition_table_offset,
        factory_segments: segments,
        application,
    })
}

fn to_segment(root: &Path, role: SegmentRole, raw: &FlasherSegment) -> Result<IdfSegment> {
    Ok(IdfSegment {
        role,
        offset: parse_offset(&raw.offset)?,
        source: checked_source(root, &raw.file)?,
    })
}

fn parse_offset(value: &str) -> Result<HexAddress> {
    let json = format!("\"{value}\"");
    serde_json::from_str(&json).map_err(|error| {
        OperationError::new(
            ErrorCode::PackageInvalid,
            "Неверный offset в flasher_args.json",
        )
        .with_detail(error.to_string())
    })
}

fn reject_encryption(segment: &FlasherSegment) -> Result<()> {
    if segment.encrypted.as_ref().is_some_and(|value| {
        !value.is_null()
            && value != &serde_json::Value::Bool(false)
            && value != &serde_json::Value::String("false".to_string())
    }) {
        return Err(OperationError::new(
            ErrorCode::PackageUnsupported,
            "Encrypted flash segments не поддерживаются в v1",
        )
        .with_detail(segment.file.clone()));
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize().map_err(|error| {
        OperationError::new(ErrorCode::PackagePathInvalid, "Build-папка недоступна")
            .with_detail(error.to_string())
    })?;
    if !canonical.is_dir() {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Build path не является папкой",
        ));
    }
    Ok(canonical)
}

fn checked_source(root: &Path, value: &str) -> Result<PathBuf> {
    if value.contains('\\')
        || value.contains(':')
        || Path::new(value).is_absolute()
        || Path::new(value)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Путь в flasher_args.json выходит за build-папку",
        )
        .with_detail(value));
    }
    let source = root.join(value).canonicalize().map_err(|error| {
        OperationError::new(
            ErrorCode::PackageFileMissing,
            "BIN из build-папки не найден",
        )
        .with_detail(error.to_string())
    })?;
    if !source.starts_with(root) || !source.is_file() {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "BIN должен быть обычным файлом внутри build-папки",
        )
        .with_detail(value));
    }
    Ok(source)
}
