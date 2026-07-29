use crate::error::{ErrorCode, OperationError, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt;

pub const SCHEMA_VERSION: u32 = 1;
const MAX_TEXT_LEN: usize = 120;
const MAX_VERSION_LEN: usize = 64;
const MAX_MARKER_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageKind {
    Factory,
    Update,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChipFamily {
    Esp32,
    Esp32c2,
    Esp32c3,
    Esp32c5,
    Esp32c6,
    Esp32h2,
    Esp32p4,
    Esp32s2,
    Esp32s3,
}

impl ChipFamily {
    pub const ALL: [Self; 9] = [
        Self::Esp32,
        Self::Esp32c2,
        Self::Esp32c3,
        Self::Esp32c5,
        Self::Esp32c6,
        Self::Esp32h2,
        Self::Esp32p4,
        Self::Esp32s2,
        Self::Esp32s3,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Esp32 => "esp32",
            Self::Esp32c2 => "esp32c2",
            Self::Esp32c3 => "esp32c3",
            Self::Esp32c5 => "esp32c5",
            Self::Esp32c6 => "esp32c6",
            Self::Esp32h2 => "esp32h2",
            Self::Esp32p4 => "esp32p4",
            Self::Esp32s2 => "esp32s2",
            Self::Esp32s3 => "esp32s3",
        }
    }

    pub const fn bootloader_address(self) -> u32 {
        match self {
            Self::Esp32c2 | Self::Esp32c3 | Self::Esp32c6 | Self::Esp32h2 | Self::Esp32s3 => 0x0000,
            Self::Esp32 | Self::Esp32s2 => 0x1000,
            Self::Esp32c5 | Self::Esp32p4 => 0x2000,
        }
    }
}

impl fmt::Display for ChipFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for ChipFamily {
    type Error = OperationError;

    fn try_from(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "esp32" => Ok(Self::Esp32),
            "esp32c2" => Ok(Self::Esp32c2),
            "esp32c3" => Ok(Self::Esp32c3),
            "esp32c5" => Ok(Self::Esp32c5),
            "esp32c6" => Ok(Self::Esp32c6),
            "esp32h2" => Ok(Self::Esp32h2),
            "esp32p4" => Ok(Self::Esp32p4),
            "esp32s2" => Ok(Self::Esp32s2),
            "esp32s3" => Ok(Self::Esp32s3),
            _ => Err(OperationError::new(
                ErrorCode::PackageUnsupported,
                "Неподдерживаемое семейство ESP32",
            )
            .with_detail(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HexAddress(pub u32);

impl fmt::Display for HexAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:X}", self.0)
    }
}

impl Serialize for HexAddress {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for HexAddress {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let digits = value
            .strip_prefix("0x")
            .ok_or_else(|| serde::de::Error::custom("address must start with 0x"))?;
        if digits.is_empty() || digits.len() > 8 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(serde::de::Error::custom("invalid hexadecimal address"));
        }
        u32::from_str_radix(digits, 16)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentRole {
    Bootloader,
    PartitionTable,
    Application,
    OtaData,
    Data,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirmwareSegment {
    pub role: SegmentRole,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<HexAddress>,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorPolicy {
    pub baud: u32,
    pub success_marker: String,
    pub success_timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtaPolicy {
    pub rollback_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirmwareManifest {
    pub schema_version: u32,
    pub package_id: String,
    pub display_name: String,
    pub firmware_version: String,
    pub kind: PackageKind,
    pub target_chips: Vec<ChipFamily>,
    pub partition_table_offset: HexAddress,
    pub monitor: MonitorPolicy,
    pub ota: OtaPolicy,
    pub segments: Vec<FirmwareSegment>,
}

impl FirmwareManifest {
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|error| {
            OperationError::new(
                ErrorCode::PackageInvalid,
                "Не удалось прочитать firmware.json",
            )
            .with_detail(error.to_string())
        })?;
        manifest.validate_structure()?;
        Ok(manifest)
    }

    pub fn validate_structure(&self) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(OperationError::new(
                ErrorCode::PackageUnsupported,
                "Версия firmware.json не поддерживается",
            )
            .with_detail(self.schema_version.to_string()));
        }
        if !is_package_id(&self.package_id) {
            return Err(invalid("package_id имеет недопустимый формат"));
        }
        validate_text("display_name", &self.display_name, MAX_TEXT_LEN)?;
        validate_text("firmware_version", &self.firmware_version, MAX_VERSION_LEN)?;
        if self.target_chips.is_empty() {
            return Err(invalid("target_chips не должен быть пустым"));
        }
        let mut unique_chips = HashSet::new();
        if self
            .target_chips
            .iter()
            .any(|chip| !unique_chips.insert(*chip))
        {
            return Err(invalid("target_chips содержит повторяющиеся значения"));
        }
        self.monitor.validate()?;
        if self.segments.is_empty() {
            return Err(invalid("segments не должен быть пустым"));
        }

        match self.kind {
            PackageKind::Factory => {
                if self.segments.iter().any(|segment| segment.offset.is_none()) {
                    return Err(invalid(
                        "factory-сегмент должен содержать абсолютный offset",
                    ));
                }
                validate_factory_ranges(&self.segments)?;
                if let Some(partition) = self
                    .segments
                    .iter()
                    .find(|segment| segment.role == SegmentRole::PartitionTable)
                {
                    if partition.offset != Some(self.partition_table_offset) {
                        return Err(invalid(
                            "offset partition table не совпадает с partition_table_offset",
                        ));
                    }
                }
            }
            PackageKind::Update => {
                if self.segments.len() != 1
                    || self.segments[0].role != SegmentRole::Application
                    || self.segments[0].offset.is_some()
                {
                    return Err(invalid(
                        "update-пакет должен содержать один application без offset",
                    ));
                }
            }
        }

        for segment in &self.segments {
            segment.validate()?;
        }
        Ok(())
    }
}

impl MonitorPolicy {
    fn validate(&self) -> Result<()> {
        const BAUD_RATES: [u32; 8] = [
            9_600, 19_200, 38_400, 57_600, 115_200, 230_400, 460_800, 921_600,
        ];
        if !BAUD_RATES.contains(&self.baud) {
            return Err(invalid("monitor.baud не поддерживается"));
        }
        let marker_len = self.success_marker.len();
        if marker_len > MAX_MARKER_BYTES {
            return Err(invalid(
                "monitor.success_marker должен занимать не более 256 байт",
            ));
        }
        if !(1_000..=120_000).contains(&self.success_timeout_ms) {
            return Err(invalid(
                "monitor.success_timeout_ms должен быть от 1000 до 120000",
            ));
        }
        Ok(())
    }
}

impl FirmwareSegment {
    fn validate(&self) -> Result<()> {
        if self.file.is_empty() || self.file.len() > 240 {
            return Err(invalid("segment.file имеет недопустимую длину"));
        }
        if self.size == 0 {
            return Err(invalid("segment.size должен быть больше нуля"));
        }
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(invalid("segment.sha256 должен быть lowercase SHA-256"));
        }
        Ok(())
    }
}

fn validate_factory_ranges(segments: &[FirmwareSegment]) -> Result<()> {
    let mut ranges = Vec::with_capacity(segments.len());
    for segment in segments {
        let start = segment.offset.expect("offset checked for factory").0;
        let size = u32::try_from(segment.size)
            .map_err(|_| invalid("размер сегмента не помещается в адресное пространство"))?;
        let end = start
            .checked_add(size)
            .ok_or_else(|| invalid("диапазон сегмента переполняет адресное пространство"))?;
        ranges.push((start, end, segment.file.as_str()));
    }
    ranges.sort_unstable_by_key(|range| range.0);
    for pair in ranges.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(invalid("factory-сегменты пересекаются")
                .with_detail(format!("{} и {}", pair[0].2, pair[1].2)));
        }
    }
    Ok(())
}

fn is_package_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

fn validate_text(field: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty() || value.chars().count() > max_len {
        return Err(invalid(format!("{field} имеет недопустимую длину")));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid(format!("{field} содержит управляющие символы")));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> OperationError {
    OperationError::new(ErrorCode::PackageInvalid, message)
}
