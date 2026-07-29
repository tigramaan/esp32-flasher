use programmer_core::{
    parse_partition_table, select_factory_application, validate_esp_image, validate_package,
    ChipFamily, ErrorCode, FirmwareManifest, FirmwareSegment, FirmwareSource, HexAddress,
    MonitorPolicy, OperationError, OtaPolicy, PackageKind, PackageReader, PackageSummary, Result,
    SegmentRole, ValidatedPackage, MANIFEST_FILE_NAME,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const DIRECT_PARTITION_TABLE_OFFSET: u32 = 0x8000;
const DEFAULT_MONITOR_BAUD: u32 = 115_200;
const DEFAULT_SUCCESS_TIMEOUT_MS: u64 = 15_000;
const GENERATED_OTADATA_FILE: &str = "_programmer_otadata.bin";

#[derive(Debug, Clone)]
pub struct LoadedPackage {
    pub validated: ValidatedPackage,
    pub source: FirmwareSource,
    pub device_partition_table_offset: Option<u32>,
    bytes: HashMap<String, Vec<u8>>,
}

impl LoadedPackage {
    pub fn segment_bytes(&self, file: &str) -> Result<&[u8]> {
        self.bytes.get(file).map(Vec::as_slice).ok_or_else(|| {
            OperationError::new(
                ErrorCode::PackageFileMissing,
                "Проверенный BIN отсутствует в снимке папки",
            )
            .with_detail(file)
        })
    }

    pub fn summary(&self) -> PackageSummary {
        let mut summary = PackageSummary::from(&self.validated);
        summary.source = self.source;
        summary.requires_device_layout = self.validated.manifest.kind == PackageKind::Update
            && self.device_partition_table_offset.is_none();
        summary
    }
}

pub fn load_package(path: impl AsRef<Path>) -> Result<LoadedPackage> {
    let selected = path.as_ref();
    let metadata = fs::symlink_metadata(selected).map_err(|error| {
        OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Выбранный файл или папка прошивки недоступны",
        )
        .with_detail(error.to_string())
    })?;
    if metadata.file_type().is_symlink() {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Symlink нельзя использовать как источник прошивки",
        ));
    }
    if metadata.is_file() {
        return load_direct_update_file(selected);
    }
    if !metadata.is_dir() {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Выбранный путь не является обычным BIN или папкой прошивки",
        ));
    }

    let root = canonical_directory(selected)?;
    let manifest_path = root.join(MANIFEST_FILE_NAME);
    match fs::symlink_metadata(&manifest_path) {
        Ok(_) => load_legacy(root, manifest_path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => load_direct(root),
        Err(error) => Err(io_error(error)),
    }
}

fn load_direct_update_file(path: &Path) -> Result<LoadedPackage> {
    let canonical = path.canonicalize().map_err(io_error)?;
    let application_file = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            OperationError::new(
                ErrorCode::PackagePathInvalid,
                "Имя BIN-файла должно быть валидным UTF-8",
            )
        })?
        .to_string();
    if !application_file.to_ascii_lowercase().ends_with(".bin") {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "Для обновления выберите файл с расширением .bin",
        )
        .with_detail(application_file));
    }
    let root = canonical
        .parent()
        .ok_or_else(|| {
            OperationError::new(
                ErrorCode::PackagePathInvalid,
                "Не удалось определить папку выбранного BIN",
            )
        })?
        .to_path_buf();
    build_direct_update(root, application_file)
}

fn load_legacy(root: PathBuf, manifest_path: PathBuf) -> Result<LoadedPackage> {
    let metadata = fs::symlink_metadata(&manifest_path).map_err(io_error)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "firmware.json должен быть обычным файлом",
        ));
    }
    let manifest_bytes = fs::read(&manifest_path).map_err(io_error)?;
    if manifest_bytes.len() > 1024 * 1024 {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "firmware.json превышает 1 MiB",
        ));
    }
    let manifest = FirmwareManifest::from_json(&manifest_bytes)?;
    let reader = FsPackageReader { root: root.clone() };
    let mut bytes = HashMap::with_capacity(manifest.segments.len());
    for segment in &manifest.segments {
        bytes.insert(
            segment.file.clone(),
            reader.read_file(&segment.file, programmer_core::MAX_SEGMENT_BYTES)?,
        );
    }
    let validated = validate_package(&manifest_bytes, &MemoryPackageReader(&bytes))?;
    let device_partition_table_offset = (validated.manifest.kind == PackageKind::Update)
        .then_some(validated.manifest.partition_table_offset.0);
    Ok(LoadedPackage {
        validated,
        source: FirmwareSource::LegacyManifest,
        device_partition_table_offset,
        bytes,
    })
}

fn load_direct(root: PathBuf) -> Result<LoadedPackage> {
    let bin_files = list_bin_files(&root)?;
    let bootloader = unique_named(&bin_files, &["bootloader.bin"])?;
    let partitions = unique_named(
        &bin_files,
        &[
            "partitions.bin",
            "partition-table.bin",
            "partition_table.bin",
        ],
    )?;
    let application = unique_named(&bin_files, &["firmware.bin"])?;
    let boot_app0 = unique_named(&bin_files, &["boot_app0.bin"])?;
    let has_factory_indicator = bootloader.is_some() || partitions.is_some() || boot_app0.is_some();

    if has_factory_indicator {
        let bootloader = required_direct_file(bootloader, "bootloader.bin")?;
        let partitions = required_direct_file(partitions, "partitions.bin")?;
        let application = required_direct_file(application, "firmware.bin")?;
        let allowed = [
            Some(&bootloader),
            Some(&partitions),
            Some(&application),
            boot_app0.as_ref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let unknown: Vec<_> = bin_files
            .iter()
            .filter(|file| !allowed.iter().any(|known| known.eq_ignore_ascii_case(file)))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "В factory-папке найдены BIN с неизвестными адресами",
            )
            .with_detail(unknown.join(", ")));
        }
        build_direct_factory(root, bootloader, partitions, application, boot_app0)
    } else if bin_files.len() == 1 {
        build_direct_update(root, bin_files[0].clone())
    } else if bin_files.is_empty() {
        Err(OperationError::new(
            ErrorCode::PackageFileMissing,
            "В выбранной папке нет BIN-файлов",
        ))
    } else {
        Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "Для обновления в папке должен быть ровно один application BIN",
        )
        .with_detail(bin_files.join(", ")))
    }
}

fn build_direct_factory(
    root: PathBuf,
    bootloader_file: String,
    partitions_file: String,
    application_file: String,
    boot_app0_file: Option<String>,
) -> Result<LoadedPackage> {
    let reader = FsPackageReader { root: root.clone() };
    let bootloader = reader.read_file(&bootloader_file, programmer_core::MAX_SEGMENT_BYTES)?;
    let partitions = reader.read_file(&partitions_file, programmer_core::MAX_SEGMENT_BYTES)?;
    let application = reader.read_file(&application_file, programmer_core::MAX_SEGMENT_BYTES)?;
    let chip = validate_esp_image(&application, &ChipFamily::ALL)?;
    validate_esp_image(&bootloader, &[chip]).map_err(|error| {
        OperationError::new(
            ErrorCode::ChipMismatch,
            "bootloader.bin и firmware.bin предназначены для разных чипов",
        )
        .with_detail(error.to_string())
    })?;
    let entries = parse_partition_table(&partitions)?;
    let app_target = select_factory_application(&entries, application.len() as u64)?;

    let mut bytes = HashMap::from([
        (bootloader_file.clone(), bootloader),
        (partitions_file.clone(), partitions),
        (application_file.clone(), application),
    ]);
    let mut segments = vec![
        segment(
            SegmentRole::Bootloader,
            &bootloader_file,
            Some(chip.bootloader_address()),
            bytes[&bootloader_file].as_slice(),
        ),
        segment(
            SegmentRole::PartitionTable,
            &partitions_file,
            Some(DIRECT_PARTITION_TABLE_OFFSET),
            bytes[&partitions_file].as_slice(),
        ),
    ];

    let ota_target = entries.iter().find(|entry| entry.is_ota_data());
    if let Some(file) = boot_app0_file {
        let ota_target = ota_target.ok_or_else(|| {
            OperationError::new(
                ErrorCode::PartitionInvalid,
                "boot_app0.bin присутствует, но в partitions.bin нет otadata",
            )
        })?;
        let data = reader.read_file(&file, programmer_core::MAX_SEGMENT_BYTES)?;
        if data.len() as u64 > u64::from(ota_target.size) {
            return Err(OperationError::new(
                ErrorCode::PartitionInvalid,
                "boot_app0.bin не помещается в otadata",
            ));
        }
        segments.push(segment(
            SegmentRole::OtaData,
            &file,
            Some(ota_target.offset),
            &data,
        ));
        bytes.insert(file, data);
    } else if let Some(ota_target) = ota_target {
        if ota_target.size < 0x2000 {
            return Err(OperationError::new(
                ErrorCode::PartitionInvalid,
                "Раздел otadata меньше двух erase-секторов",
            ));
        }
        let data = vec![0xFF; 0x2000];
        segments.push(segment(
            SegmentRole::OtaData,
            GENERATED_OTADATA_FILE,
            Some(ota_target.offset),
            &data,
        ));
        bytes.insert(GENERATED_OTADATA_FILE.to_string(), data);
    }
    segments.push(segment(
        SegmentRole::Application,
        &application_file,
        Some(app_target.offset),
        bytes[&application_file].as_slice(),
    ));
    finish_direct(
        root,
        PackageKind::Factory,
        FirmwareSource::Platformio,
        chip,
        DIRECT_PARTITION_TABLE_OFFSET,
        segments,
        bytes,
        Some(DIRECT_PARTITION_TABLE_OFFSET),
    )
}

fn build_direct_update(root: PathBuf, application_file: String) -> Result<LoadedPackage> {
    let reader = FsPackageReader { root: root.clone() };
    let application = reader.read_file(&application_file, programmer_core::MAX_SEGMENT_BYTES)?;
    let chip = validate_esp_image(&application, &ChipFamily::ALL)?;
    let segments = vec![segment(
        SegmentRole::Application,
        &application_file,
        None,
        &application,
    )];
    finish_direct(
        root,
        PackageKind::Update,
        FirmwareSource::Standalone,
        chip,
        DIRECT_PARTITION_TABLE_OFFSET,
        segments,
        HashMap::from([(application_file, application)]),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_direct(
    root: PathBuf,
    kind: PackageKind,
    source: FirmwareSource,
    chip: ChipFamily,
    partition_table_offset: u32,
    segments: Vec<FirmwareSegment>,
    bytes: HashMap<String, Vec<u8>>,
    device_partition_table_offset: Option<u32>,
) -> Result<LoadedPackage> {
    let application = segments
        .iter()
        .find(|value| value.role == SegmentRole::Application)
        .expect("direct package always has application");
    let display_name = match source {
        FirmwareSource::Standalone => Path::new(&application.file)
            .file_stem()
            .and_then(|value| value.to_str()),
        FirmwareSource::Platformio | FirmwareSource::LegacyManifest => {
            root.file_name().and_then(|value| value.to_str())
        }
    }
    .filter(|value| !value.trim().is_empty())
    .unwrap_or("Firmware")
    .to_string();
    let firmware_version = format!("sha256-{}", &application.sha256[..12]);
    let manifest = FirmwareManifest {
        schema_version: 1,
        package_id: package_id(&display_name),
        display_name,
        firmware_version,
        kind,
        target_chips: vec![chip],
        partition_table_offset: HexAddress(partition_table_offset),
        monitor: MonitorPolicy {
            baud: DEFAULT_MONITOR_BAUD,
            success_marker: String::new(),
            success_timeout_ms: DEFAULT_SUCCESS_TIMEOUT_MS,
        },
        ota: OtaPolicy {
            rollback_enabled: false,
        },
        segments,
    };
    let manifest_bytes = serde_json::to_vec(&manifest).map_err(|error| {
        OperationError::new(
            ErrorCode::InternalError,
            "Не удалось сформировать внутренний план BIN",
        )
        .with_detail(error.to_string())
    })?;
    let validated = validate_package(&manifest_bytes, &MemoryPackageReader(&bytes))?;
    Ok(LoadedPackage {
        validated,
        source,
        device_partition_table_offset,
        bytes,
    })
}

fn segment(role: SegmentRole, file: &str, offset: Option<u32>, bytes: &[u8]) -> FirmwareSegment {
    FirmwareSegment {
        role,
        file: file.to_string(),
        offset: offset.map(HexAddress),
        size: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    }
}

fn list_bin_files(root: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for item in fs::read_dir(root).map_err(io_error)? {
        let item = item.map_err(io_error)?;
        let name = item.file_name().into_string().map_err(|_| {
            OperationError::new(
                ErrorCode::PackagePathInvalid,
                "Имя файла прошивки должно быть валидным UTF-8",
            )
        })?;
        if !name.to_ascii_lowercase().ends_with(".bin") {
            continue;
        }
        let metadata = fs::symlink_metadata(item.path()).map_err(io_error)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(OperationError::new(
                ErrorCode::PackagePathInvalid,
                "BIN должен быть обычным файлом, symlink запрещён",
            )
            .with_detail(name));
        }
        files.push(name);
    }
    files.sort_by_key(|value| value.to_ascii_lowercase());
    Ok(files)
}

fn unique_named(files: &[String], aliases: &[&str]) -> Result<Option<String>> {
    let matches: Vec<_> = files
        .iter()
        .filter(|file| aliases.iter().any(|alias| file.eq_ignore_ascii_case(alias)))
        .cloned()
        .collect();
    match matches.as_slice() {
        [] => Ok(None),
        [value] => Ok(Some(value.clone())),
        _ => Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "Найдено несколько BIN для одной роли",
        )
        .with_detail(matches.join(", "))),
    }
}

fn required_direct_file(value: Option<String>, expected: &str) -> Result<String> {
    value.ok_or_else(|| {
        OperationError::new(
            ErrorCode::PackageFileMissing,
            "Factory-папка PlatformIO неполна",
        )
        .with_detail(format!("не найден {expected}"))
    })
}

fn package_id(display_name: &str) -> String {
    let mut result = String::with_capacity(display_name.len().min(64));
    let mut separator = false;
    for byte in display_name.bytes() {
        let value = if byte.is_ascii_alphanumeric() || b"._-".contains(&byte) {
            separator = false;
            byte as char
        } else if !separator {
            separator = true;
            '-'
        } else {
            continue;
        };
        if result.len() == 64 {
            break;
        }
        result.push(value);
    }
    let trimmed = result.trim_matches('-');
    if trimmed.is_empty() {
        "firmware".to_string()
    } else {
        trimmed.to_string()
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf> {
    let root = path.canonicalize().map_err(|error| {
        OperationError::new(ErrorCode::PackagePathInvalid, "Папка прошивки недоступна")
            .with_detail(error.to_string())
    })?;
    if !root.is_dir() {
        return Err(OperationError::new(
            ErrorCode::PackagePathInvalid,
            "Выбранный путь не является папкой",
        ));
    }
    Ok(root)
}

struct MemoryPackageReader<'a>(&'a HashMap<String, Vec<u8>>);

impl PackageReader for MemoryPackageReader<'_> {
    fn read_file(&self, relative_path: &str, _max_bytes: u64) -> Result<Vec<u8>> {
        self.0.get(relative_path).cloned().ok_or_else(|| {
            OperationError::new(
                ErrorCode::PackageFileMissing,
                "BIN отсутствует в снимке папки",
            )
            .with_detail(relative_path)
        })
    }
}

struct FsPackageReader {
    root: PathBuf,
}

impl PackageReader for FsPackageReader {
    fn read_file(&self, relative_path: &str, max_bytes: u64) -> Result<Vec<u8>> {
        programmer_core::validate_relative_path(relative_path)?;
        let joined = self.root.join(relative_path);
        let metadata = fs::symlink_metadata(&joined).map_err(|error| {
            OperationError::new(ErrorCode::PackageFileMissing, "BIN из папки не найден")
                .with_detail(error.to_string())
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(OperationError::new(
                ErrorCode::PackagePathInvalid,
                "BIN должен быть обычным файлом",
            )
            .with_detail(relative_path));
        }
        if metadata.len() > max_bytes {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "BIN превышает допустимый размер",
            )
            .with_detail(relative_path));
        }
        let canonical = joined.canonicalize().map_err(io_error)?;
        if !canonical.starts_with(&self.root) {
            return Err(OperationError::new(
                ErrorCode::PackagePathInvalid,
                "BIN выходит за папку прошивки",
            )
            .with_detail(relative_path));
        }
        fs::read(canonical).map_err(io_error)
    }
}

fn io_error(error: impl std::fmt::Display) -> OperationError {
    OperationError::new(ErrorCode::IoError, "Ошибка чтения папки прошивки")
        .with_detail(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::load_package;
    use programmer_core::{FirmwareSource, PackageKind};
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn image(chip_id: u16) -> Vec<u8> {
        let mut bytes = vec![0; 64];
        bytes[0] = 0xE9;
        bytes[12..14].copy_from_slice(&chip_id.to_le_bytes());
        bytes
    }

    fn partition(kind: u8, subtype: u8, offset: u32, size: u32, label: &str) -> [u8; 32] {
        let mut raw = [0xFF; 32];
        raw[0..2].copy_from_slice(&0x50AA_u16.to_le_bytes());
        raw[2] = kind;
        raw[3] = subtype;
        raw[4..8].copy_from_slice(&offset.to_le_bytes());
        raw[8..12].copy_from_slice(&size.to_le_bytes());
        raw[12..12 + label.len()].copy_from_slice(label.as_bytes());
        raw[28..32].copy_from_slice(&0_u32.to_le_bytes());
        raw
    }

    fn table() -> Vec<u8> {
        let mut bytes = vec![0xFF; 0x1000];
        bytes[..32].copy_from_slice(&partition(1, 0, 0xD000, 0x2000, "otadata"));
        bytes[32..64].copy_from_slice(&partition(0, 0, 0x10000, 0x180000, "factory"));
        bytes
    }

    #[test]
    fn detects_platformio_factory_without_json() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("bootloader.bin"), image(9)).unwrap();
        fs::write(temp.path().join("partitions.bin"), table()).unwrap();
        fs::write(temp.path().join("firmware.bin"), image(9)).unwrap();

        let package = load_package(temp.path()).unwrap();
        let summary = package.summary();
        assert_eq!(summary.kind, PackageKind::Factory);
        assert_eq!(summary.source, FirmwareSource::Platformio);
        assert_eq!(summary.segment_count, 4);
        assert_eq!(summary.segments[0].offset.as_deref(), Some("0x0"));
        assert_eq!(
            summary.segments[2].role,
            programmer_core::SegmentRole::OtaData
        );
        assert_eq!(summary.segments[3].offset.as_deref(), Some("0x10000"));
    }

    #[test]
    fn detects_arbitrary_single_application_bin() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("nova-2.0.bin"), image(0)).unwrap();

        let summary = load_package(temp.path()).unwrap().summary();
        assert_eq!(summary.kind, PackageKind::Update);
        assert_eq!(summary.source, FirmwareSource::Standalone);
        assert!(summary.requires_device_layout);
        assert!(summary.segments[0].offset.is_none());
    }

    #[test]
    fn loads_a_direct_arbitrary_name_application_bin_file() {
        let temp = TempDir::new().unwrap();
        let selected = temp.path().join("nova client 2.0.BIN");
        fs::write(&selected, image(0)).unwrap();

        let summary = load_package(&selected).unwrap().summary();

        assert_eq!(summary.kind, PackageKind::Update);
        assert_eq!(summary.source, FirmwareSource::Standalone);
        assert_eq!(summary.display_name, "nova client 2.0");
        assert_eq!(summary.segments[0].file, "nova client 2.0.BIN");
    }

    #[test]
    fn rejects_a_direct_non_bin_file() {
        let temp = TempDir::new().unwrap();
        let selected = temp.path().join("firmware.txt");
        fs::write(&selected, image(0)).unwrap();

        let error = load_package(&selected).unwrap_err();

        assert_eq!(error.code, programmer_core::ErrorCode::PackageInvalid);
        assert!(error.message.contains(".bin"));
    }

    #[test]
    fn rejects_incomplete_and_ambiguous_direct_folders() {
        let incomplete = TempDir::new().unwrap();
        fs::write(incomplete.path().join("partitions.bin"), table()).unwrap();
        fs::write(incomplete.path().join("firmware.bin"), image(0)).unwrap();
        assert!(load_package(incomplete.path()).is_err());

        let ambiguous = TempDir::new().unwrap();
        fs::write(ambiguous.path().join("first.bin"), image(0)).unwrap();
        fs::write(ambiguous.path().join("second.bin"), image(0)).unwrap();
        assert!(load_package(ambiguous.path()).is_err());

        let unknown = TempDir::new().unwrap();
        fs::write(unknown.path().join("bootloader.bin"), image(0)).unwrap();
        fs::write(unknown.path().join("partitions.bin"), table()).unwrap();
        fs::write(unknown.path().join("firmware.bin"), image(0)).unwrap();
        fs::write(unknown.path().join("filesystem.bin"), [1, 2, 3]).unwrap();
        assert!(load_package(unknown.path()).is_err());
    }

    #[test]
    fn keeps_legacy_manifest_as_optional_compatibility_path() {
        let temp = TempDir::new().unwrap();
        let application = image(0);
        fs::write(temp.path().join("application.bin"), &application).unwrap();
        let hash = format!("{:x}", Sha256::digest(&application));
        fs::write(
            temp.path().join("firmware.json"),
            format!(
                r#"{{
                    "schema_version": 1,
                    "package_id": "legacy",
                    "display_name": "Legacy",
                    "firmware_version": "1",
                    "kind": "update",
                    "target_chips": ["esp32"],
                    "partition_table_offset": "0x8000",
                    "monitor": {{
                        "baud": 115200,
                        "success_marker": "READY",
                        "success_timeout_ms": 15000
                    }},
                    "ota": {{ "rollback_enabled": false }},
                    "segments": [{{
                        "role": "application",
                        "file": "application.bin",
                        "size": {},
                        "sha256": "{hash}"
                    }}]
                }}"#,
                application.len()
            ),
        )
        .unwrap();

        let package = load_package(temp.path()).unwrap();
        let summary = package.summary();
        assert_eq!(summary.source, FirmwareSource::LegacyManifest);
        assert!(!summary.requires_device_layout);
        assert!(summary.success_marker_configured);
    }

    #[test]
    fn loads_repository_direct_fixtures_end_to_end() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let factory = load_package(root.join("tests/fixtures/direct/platformio"))
            .unwrap()
            .summary();
        let update = load_package(root.join("tests/fixtures/direct/update"))
            .unwrap()
            .summary();
        assert_eq!(factory.source, FirmwareSource::Platformio);
        assert_eq!(
            factory.segments.last().unwrap().offset.as_deref(),
            Some("0x10000")
        );
        assert_eq!(update.source, FirmwareSource::Standalone);
        assert!(update.requires_device_layout);
    }
}
