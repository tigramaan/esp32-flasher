use md5::{Digest, Md5};
use programmer_core::{
    build_otadata_sector, parse_partition_table, select_factory_application, select_update_layout,
    ChipFamily, ErrorCode, UpdateLayout,
};

fn entry(kind: u8, subtype: u8, offset: u32, size: u32, label: &str) -> [u8; 32] {
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

fn ota_table() -> Vec<u8> {
    let mut table = vec![0xFF; 0x1000];
    let rows = [
        entry(1, 0, 0xD000, 0x2000, "otadata"),
        entry(0, 0, 0x10000, 0x100000, "factory"),
        entry(0, 0x10, 0x110000, 0x100000, "ota_0"),
        entry(0, 0x11, 0x210000, 0x100000, "ota_1"),
    ];
    for (index, row) in rows.iter().enumerate() {
        table[index * 32..(index + 1) * 32].copy_from_slice(row);
    }
    table
}

fn with_md5(mut table: Vec<u8>, entry_count: usize) -> Vec<u8> {
    let start = entry_count * 32;
    let digest = Md5::digest(&table[..start]);
    table[start..start + 16].copy_from_slice(&[
        0xEB, 0xEB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF,
    ]);
    table[start + 16..start + 32].copy_from_slice(&digest);
    table
}

#[test]
fn selects_ota_zero_from_factory_state() {
    let entries = parse_partition_table(&ota_table()).unwrap();
    let erased = vec![0xFF; 0x2000];
    let layout = select_update_layout(&entries, Some(&erased), 0x400000, 0x80000, false)
        .expect("valid layout");
    match layout {
        UpdateLayout::Ota { target, switch, .. } => {
            assert_eq!(target.label, "ota_0");
            assert_eq!(switch.target_sequence, 1);
            assert_eq!(switch.metadata_sector, 0);
            let sector = build_otadata_sector(&switch);
            assert_eq!(
                u32::from_le_bytes(sector[0..4].try_into().unwrap()),
                switch.target_sequence
            );
        }
        _ => panic!("expected OTA"),
    }
}

#[test]
fn rejects_image_larger_than_slot() {
    let entries = parse_partition_table(&ota_table()).unwrap();
    let erased = vec![0xFF; 0x2000];
    let error =
        select_update_layout(&entries, Some(&erased), 0x400000, 0x200000, false).unwrap_err();
    assert_eq!(error.code, ErrorCode::PartitionInvalid);
}

#[test]
fn rejects_overlapping_partitions() {
    let mut table = vec![0xFF; 0x1000];
    table[0..32].copy_from_slice(&entry(0, 0, 0x10000, 0x20000, "factory"));
    table[32..64].copy_from_slice(&entry(1, 2, 0x20000, 0x10000, "nvs"));
    assert!(parse_partition_table(&table).is_err());
}

#[test]
fn verifies_partition_table_md5() {
    let mut valid = with_md5(ota_table(), 4);
    assert_eq!(parse_partition_table(&valid).unwrap().len(), 4);
    valid[8] ^= 1;
    let error = parse_partition_table(&valid).unwrap_err();
    assert_eq!(error.code, ErrorCode::PartitionInvalid);
    assert!(error.message.contains("MD5"));
}

#[test]
fn selects_factory_then_first_ota_for_initial_flash() {
    let entries = parse_partition_table(&ota_table()).unwrap();
    assert_eq!(
        select_factory_application(&entries, 0x80000).unwrap().label,
        "factory"
    );

    let ota_only: Vec<_> = entries
        .into_iter()
        .filter(|entry| !entry.is_factory_app())
        .collect();
    assert_eq!(
        select_factory_application(&ota_only, 0x80000)
            .unwrap()
            .label,
        "ota_0"
    );
}

#[test]
fn single_ota_is_safe_when_factory_is_active_then_in_place_after_switch() {
    let mut table = ota_table();
    table[3 * 32..4 * 32].fill(0xFF);
    let entries = parse_partition_table(&table).unwrap();
    let erased = vec![0xFF; 0x2000];
    assert!(matches!(
        select_update_layout(&entries, Some(&erased), 0x400000, 0x80000, false).unwrap(),
        UpdateLayout::Ota { .. }
    ));

    let mut active = erased;
    active[0..4].copy_from_slice(&1_u32.to_le_bytes());
    active[28..32].copy_from_slice(&0x4743_989A_u32.to_le_bytes());
    assert!(matches!(
        select_update_layout(&entries, Some(&active), 0x400000, 0x80000, false).unwrap(),
        UpdateLayout::InPlace { .. }
    ));
}

#[test]
fn exposes_chip_specific_bootloader_addresses() {
    assert_eq!(ChipFamily::Esp32.bootloader_address(), 0x1000);
    assert_eq!(ChipFamily::Esp32s3.bootloader_address(), 0x0000);
    assert_eq!(ChipFamily::Esp32p4.bootloader_address(), 0x2000);
}
