use programmer_core::{
    validate_package, ChipFamily, ErrorCode, FirmwareManifest, PackageReader, Result,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

struct MemoryReader(HashMap<String, Vec<u8>>);

impl PackageReader for MemoryReader {
    fn read_file(&self, relative_path: &str, _max_bytes: u64) -> Result<Vec<u8>> {
        self.0.get(relative_path).cloned().ok_or_else(|| {
            programmer_core::OperationError::new(ErrorCode::PackageFileMissing, "Файл не найден")
        })
    }
}

fn update_json(file: &str, bytes: &[u8]) -> Vec<u8> {
    let hash = format!("{:x}", Sha256::digest(bytes));
    format!(
        r#"{{
          "schema_version": 1,
          "package_id": "nova",
          "display_name": "NOVA",
          "firmware_version": "1.2.3",
          "kind": "update",
          "target_chips": ["esp32", "esp32s3"],
          "partition_table_offset": "0x8000",
          "monitor": {{
            "baud": 115200,
            "success_marker": "APP_READY",
            "success_timeout_ms": 15000
          }},
          "ota": {{ "rollback_enabled": false }},
          "segments": [{{
            "role": "application",
            "file": "{file}",
            "size": {},
            "sha256": "{hash}"
          }}]
        }}"#,
        bytes.len()
    )
    .into_bytes()
}

fn esp32_image() -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[0] = 0xE9;
    bytes
}

#[test]
fn validates_update_package() {
    let bytes = esp32_image();
    let reader = MemoryReader(HashMap::from([("app.bin".into(), bytes.clone())]));
    let package = validate_package(&update_json("app.bin", &bytes), &reader).unwrap();
    assert_eq!(package.total_bytes, bytes.len() as u64);
    assert_eq!(package.manifest.target_chips[0], ChipFamily::Esp32);
}

#[test]
fn rejects_hash_mismatch() {
    let expected = esp32_image();
    let mut changed = expected.clone();
    changed[31] = 1;
    let reader = MemoryReader(HashMap::from([("app.bin".into(), changed)]));
    let error = validate_package(&update_json("app.bin", &expected), &reader).unwrap_err();
    assert_eq!(error.code, ErrorCode::HashMismatch);
}

#[test]
fn rejects_traversal_before_read() {
    let bytes = esp32_image();
    let reader = MemoryReader(HashMap::new());
    let error = validate_package(&update_json("../app.bin", &bytes), &reader).unwrap_err();
    assert_eq!(error.code, ErrorCode::PackagePathInvalid);
}

#[test]
fn rejects_unknown_fields() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&update_json("app.bin", &esp32_image())).unwrap();
    value["unexpected"] = serde_json::Value::Bool(true);
    let error = FirmwareManifest::from_json(&serde_json::to_vec(&value).unwrap()).unwrap_err();
    assert_eq!(error.code, ErrorCode::PackageInvalid);
}

#[test]
fn accepts_empty_optional_success_marker() {
    let bytes = esp32_image();
    let reader = MemoryReader(HashMap::from([("app.bin".into(), bytes.clone())]));
    let mut value: serde_json::Value =
        serde_json::from_slice(&update_json("app.bin", &bytes)).unwrap();
    value["monitor"]["success_marker"] = serde_json::Value::String(String::new());
    let package = validate_package(&serde_json::to_vec(&value).unwrap(), &reader).unwrap();
    assert!(package.manifest.monitor.success_marker.is_empty());
}
