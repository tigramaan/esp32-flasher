use crate::error::{ErrorCode, OperationError, Result};
use md5::{Digest, Md5};

const PARTITION_MAGIC: u16 = 0x50AA;
const PARTITION_MD5_MAGIC: u16 = 0xEBEB;
const PARTITION_ENTRY_SIZE: usize = 32;
const OTA_SECTOR_SIZE: usize = 0x1000;
const OTA_RECORD_SIZE: usize = 32;
const TYPE_APP: u8 = 0x00;
const TYPE_DATA: u8 = 0x01;
const SUBTYPE_FACTORY: u8 = 0x00;
const SUBTYPE_OTA_DATA: u8 = 0x00;
const SUBTYPE_OTA_MIN: u8 = 0x10;
const SUBTYPE_OTA_MAX: u8 = 0x1F;
const OTA_STATE_NEW: u32 = 0;
const OTA_STATE_INVALID: u32 = 3;
const OTA_STATE_ABORTED: u32 = 4;
const OTA_STATE_UNDEFINED: u32 = u32::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionEntry {
    pub partition_type: u8,
    pub subtype: u8,
    pub offset: u32,
    pub size: u32,
    pub label: String,
    pub flags: u32,
}

impl PartitionEntry {
    pub const fn is_factory_app(&self) -> bool {
        self.partition_type == TYPE_APP && self.subtype == SUBTYPE_FACTORY
    }

    pub const fn is_ota_app(&self) -> bool {
        self.partition_type == TYPE_APP
            && self.subtype >= SUBTYPE_OTA_MIN
            && self.subtype <= SUBTYPE_OTA_MAX
    }

    pub const fn is_ota_data(&self) -> bool {
        self.partition_type == TYPE_DATA && self.subtype == SUBTYPE_OTA_DATA
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtaSwitch {
    pub target_index: usize,
    pub target_sequence: u32,
    pub metadata_sector: usize,
    pub rollback_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateLayout {
    InPlace {
        target: PartitionEntry,
    },
    Ota {
        target: PartitionEntry,
        ota_data: PartitionEntry,
        switch: OtaSwitch,
    },
}

#[derive(Debug, Clone, Copy)]
struct OtaRecord {
    sequence: u32,
    state: u32,
    crc: u32,
}

pub fn parse_partition_table(bytes: &[u8]) -> Result<Vec<PartitionEntry>> {
    if bytes.len() < PARTITION_ENTRY_SIZE || !bytes.len().is_multiple_of(PARTITION_ENTRY_SIZE) {
        return Err(partition_error("Таблица разделов слишком короткая"));
    }
    let mut entries = Vec::new();
    let mut checksum = Md5::new();
    let mut terminated = false;
    for raw in bytes.chunks_exact(PARTITION_ENTRY_SIZE) {
        let magic = u16::from_le_bytes([raw[0], raw[1]]);
        if magic == u16::MAX {
            terminated = true;
            break;
        }
        if magic == PARTITION_MD5_MAGIC {
            const MD5_MARKER: [u8; 16] = [
                0xEB, 0xEB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF,
            ];
            if raw[..16] != MD5_MARKER || checksum.finalize().as_slice() != &raw[16..32] {
                return Err(partition_error("MD5 partition table не совпадает"));
            }
            terminated = true;
            break;
        }
        if magic != PARTITION_MAGIC {
            return Err(partition_error("Неверная сигнатура записи partition table"));
        }
        checksum.update(raw);
        let offset = u32::from_le_bytes(raw[4..8].try_into().expect("fixed slice"));
        let size = u32::from_le_bytes(raw[8..12].try_into().expect("fixed slice"));
        if size == 0 {
            return Err(partition_error("Раздел нулевого размера"));
        }
        let label_bytes = &raw[12..28];
        let end = label_bytes
            .iter()
            .position(|byte| *byte == 0 || *byte == 0xFF)
            .unwrap_or(label_bytes.len());
        let label = std::str::from_utf8(&label_bytes[..end])
            .map_err(|_| partition_error("Метка раздела не является UTF-8"))?
            .to_string();
        entries.push(PartitionEntry {
            partition_type: raw[2],
            subtype: raw[3],
            offset,
            size,
            label,
            flags: u32::from_le_bytes(raw[28..32].try_into().expect("fixed slice")),
        });
    }
    if entries.is_empty() {
        return Err(partition_error("Таблица разделов пуста"));
    }
    if !terminated {
        return Err(partition_error("Таблица разделов не имеет end marker"));
    }
    validate_partition_ranges(&entries)?;
    Ok(entries)
}

pub fn select_update_layout(
    entries: &[PartitionEntry],
    otadata: Option<&[u8]>,
    flash_size: u64,
    image_size: u64,
    rollback_enabled: bool,
) -> Result<UpdateLayout> {
    validate_against_flash(entries, flash_size)?;
    let factory = entries.iter().find(|entry| entry.is_factory_app()).cloned();
    let ota_data = entries.iter().find(|entry| entry.is_ota_data()).cloned();
    let mut ota_apps: Vec<_> = entries
        .iter()
        .filter(|entry| entry.is_ota_app())
        .cloned()
        .collect();
    ota_apps.sort_unstable_by_key(|entry| entry.subtype);

    if ota_apps.is_empty() {
        let target = factory
            .ok_or_else(|| partition_error("Не найден application-раздел для обновления"))?;
        ensure_image_fits(&target, image_size)?;
        return Ok(UpdateLayout::InPlace { target });
    }
    let ota_data = ota_data.ok_or_else(|| partition_error("Не найден раздел otadata"))?;
    if ota_data.size < (OTA_SECTOR_SIZE * 2) as u32 {
        return Err(partition_error("Раздел otadata меньше двух erase-секторов"));
    }
    let raw = otadata.ok_or_else(|| ota_error("Не прочитаны OTA-метаданные"))?;
    if raw.len() < OTA_SECTOR_SIZE * 2 {
        return Err(ota_error("OTA-метаданные короче 0x2000 байт"));
    }

    let records = [
        parse_ota_record(&raw[..OTA_RECORD_SIZE])?,
        parse_ota_record(&raw[OTA_SECTOR_SIZE..OTA_SECTOR_SIZE + OTA_RECORD_SIZE])?,
    ];
    if let (Some(left), Some(right)) = (records[0], records[1]) {
        if left.sequence == right.sequence && record_bootable(left) && record_bootable(right) {
            return Err(ota_error("Две записи otadata имеют одинаковый sequence"));
        }
    }
    let active_record = records
        .iter()
        .flatten()
        .filter(|record| record_bootable(**record))
        .max_by_key(|record| record.sequence)
        .copied();

    let current_index = active_record
        .map(|record| ((record.sequence - 1) as usize) % ota_apps.len())
        .or_else(|| if factory.is_some() { None } else { Some(0) });
    if ota_apps.len() == 1 && current_index.is_some() {
        let target = ota_apps.remove(0);
        ensure_image_fits(&target, image_size)?;
        return Ok(UpdateLayout::InPlace { target });
    }
    let target_index = current_index
        .map(|index| (index + 1) % ota_apps.len())
        .unwrap_or(0);
    let target = ota_apps[target_index].clone();
    ensure_image_fits(&target, image_size)?;

    let max_sequence = active_record.map(|record| record.sequence).unwrap_or(0);
    let base = target_index as u32 + 1;
    let count = ota_apps.len() as u32;
    let target_sequence = if base > max_sequence {
        base
    } else {
        base + ((max_sequence - base) / count + 1) * count
    };
    let metadata_sector = choose_metadata_sector(records);

    Ok(UpdateLayout::Ota {
        target,
        ota_data,
        switch: OtaSwitch {
            target_index,
            target_sequence,
            metadata_sector,
            rollback_enabled,
        },
    })
}

pub fn build_otadata_sector(switch: &OtaSwitch) -> Vec<u8> {
    let mut sector = vec![0xFF; OTA_SECTOR_SIZE];
    sector[..4].copy_from_slice(&switch.target_sequence.to_le_bytes());
    let state = if switch.rollback_enabled {
        OTA_STATE_NEW
    } else {
        OTA_STATE_UNDEFINED
    };
    sector[24..28].copy_from_slice(&state.to_le_bytes());
    sector[28..32].copy_from_slice(
        &esp_crc32_le(u32::MAX, &switch.target_sequence.to_le_bytes()).to_le_bytes(),
    );
    sector
}

fn parse_ota_record(raw: &[u8]) -> Result<Option<OtaRecord>> {
    if raw.iter().all(|byte| *byte == 0xFF) {
        return Ok(None);
    }
    let record = OtaRecord {
        sequence: u32::from_le_bytes(raw[0..4].try_into().expect("fixed slice")),
        state: u32::from_le_bytes(raw[24..28].try_into().expect("fixed slice")),
        crc: u32::from_le_bytes(raw[28..32].try_into().expect("fixed slice")),
    };
    if record.sequence == u32::MAX {
        return Err(ota_error("Непустая запись otadata имеет пустой sequence"));
    }
    let expected = esp_crc32_le(u32::MAX, &record.sequence.to_le_bytes());
    if record.crc != expected {
        return Err(ota_error("CRC записи otadata не совпадает"));
    }
    Ok(Some(record))
}

fn record_bootable(record: OtaRecord) -> bool {
    record.state != OTA_STATE_INVALID && record.state != OTA_STATE_ABORTED
}

fn choose_metadata_sector(records: [Option<OtaRecord>; 2]) -> usize {
    match records {
        [None, _] => 0,
        [Some(_), None] => 1,
        [Some(left), Some(right)] => usize::from(left.sequence >= right.sequence),
    }
}

fn esp_crc32_le(initial: u32, bytes: &[u8]) -> u32 {
    let mut crc = initial ^ u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    crc ^ u32::MAX
}

fn ensure_image_fits(partition: &PartitionEntry, image_size: u64) -> Result<()> {
    if image_size == 0 || image_size > u64::from(partition.size) {
        return Err(partition_error(
            "Application BIN не помещается в целевой раздел",
        ));
    }
    Ok(())
}

pub fn select_factory_application(
    entries: &[PartitionEntry],
    image_size: u64,
) -> Result<PartitionEntry> {
    let target = entries
        .iter()
        .find(|entry| entry.is_factory_app())
        .or_else(|| {
            entries
                .iter()
                .filter(|entry| entry.is_ota_app())
                .min_by_key(|entry| entry.subtype)
        })
        .cloned()
        .ok_or_else(|| partition_error("Не найден application-раздел для factory flash"))?;
    ensure_image_fits(&target, image_size)?;
    Ok(target)
}

fn validate_partition_ranges(entries: &[PartitionEntry]) -> Result<()> {
    let mut ranges = Vec::with_capacity(entries.len());
    for entry in entries {
        let end = entry
            .offset
            .checked_add(entry.size)
            .ok_or_else(|| partition_error("Диапазон раздела переполняется"))?;
        ranges.push((entry.offset, end, entry.label.as_str()));
    }
    ranges.sort_unstable_by_key(|range| range.0);
    for pair in ranges.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(partition_error("Разделы пересекаются")
                .with_detail(format!("{} и {}", pair[0].2, pair[1].2)));
        }
    }
    Ok(())
}

fn validate_against_flash(entries: &[PartitionEntry], flash_size: u64) -> Result<()> {
    if entries
        .iter()
        .any(|entry| u64::from(entry.offset) + u64::from(entry.size) > flash_size)
    {
        return Err(partition_error("Раздел выходит за физический размер flash"));
    }
    Ok(())
}

fn partition_error(message: impl Into<String>) -> OperationError {
    OperationError::new(ErrorCode::PartitionInvalid, message)
}

fn ota_error(message: impl Into<String>) -> OperationError {
    OperationError::new(ErrorCode::OtaStateInvalid, message)
}

#[cfg(test)]
mod tests {
    use super::esp_crc32_le;

    #[test]
    fn crc_matches_rom_algorithm_vector() {
        assert_eq!(esp_crc32_le(u32::MAX, &1_u32.to_le_bytes()), 0x4743_989A);
    }
}
