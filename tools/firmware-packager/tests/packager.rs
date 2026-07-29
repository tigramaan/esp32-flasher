#[path = "../src/builder.rs"]
mod builder;
#[path = "../src/idf.rs"]
mod idf;

use builder::{build_package, validate_directory, BuildKind, BuildOptions};
use programmer_core::ErrorCode;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_idf_build(root: &Path) {
    fs::create_dir_all(root.join("bootloader")).unwrap();
    fs::create_dir_all(root.join("partition_table")).unwrap();
    fs::write(root.join("bootloader/bootloader.bin"), b"bootloader").unwrap();
    fs::write(
        root.join("partition_table/partition-table.bin"),
        b"partitions",
    )
    .unwrap();
    let mut application = vec![0; 32];
    application[0] = 0xE9;
    fs::write(root.join("app.bin"), application).unwrap();
    fs::write(
        root.join("flasher_args.json"),
        r#"{
          "flash_settings": {"flash_mode":"dio","flash_size":"4MB","flash_freq":"40m"},
          "flash_files": {
            "0x1000": "bootloader/bootloader.bin",
            "0x8000": "partition_table/partition-table.bin",
            "0x10000": "app.bin"
          },
          "bootloader": {"offset":"0x1000","file":"bootloader/bootloader.bin","encrypted":"false"},
          "partition-table": {"offset":"0x8000","file":"partition_table/partition-table.bin","encrypted":"false"},
          "app": {"offset":"0x10000","file":"app.bin","encrypted":"false"},
          "extra_esptool_args": {"chip":"esp32"}
        }"#,
    )
    .unwrap();
}

fn options(temp: &TempDir, kind: BuildKind) -> BuildOptions {
    BuildOptions {
        kind,
        build_dir: temp.path().join("build"),
        output_dir: temp.path().join("package"),
        package_id: "nova".into(),
        display_name: "NOVA".into(),
        firmware_version: "1.0.0".into(),
        monitor_baud: 115_200,
        success_marker: "APP_READY".into(),
        success_timeout_ms: 15_000,
        rollback_enabled: false,
        dry_run: false,
        force: false,
    }
}

#[test]
fn builds_factory_package() {
    let temp = TempDir::new().unwrap();
    write_idf_build(&temp.path().join("build"));
    let result = build_package(&options(&temp, BuildKind::Factory)).unwrap();
    assert_eq!(result.summary.segment_count, 3);
    assert_eq!(result.manifest.package_id, "nova");
    validate_directory(&result.output_dir).unwrap();
}

#[test]
fn builds_single_bin_update_package() {
    let temp = TempDir::new().unwrap();
    write_idf_build(&temp.path().join("build"));
    let result = build_package(&options(&temp, BuildKind::Update)).unwrap();
    assert_eq!(result.summary.segment_count, 1);
    assert!(result.output_dir.join("application.bin").is_file());
}

#[test]
fn dry_run_does_not_create_output() {
    let temp = TempDir::new().unwrap();
    write_idf_build(&temp.path().join("build"));
    let mut args = options(&temp, BuildKind::Factory);
    args.dry_run = true;
    let result = build_package(&args).unwrap();
    assert!(result.dry_run);
    assert!(!args.output_dir.exists());
}

#[test]
fn detects_corrupted_output() {
    let temp = TempDir::new().unwrap();
    write_idf_build(&temp.path().join("build"));
    let result = build_package(&options(&temp, BuildKind::Update)).unwrap();
    fs::write(result.output_dir.join("application.bin"), b"corrupt").unwrap();
    assert!(validate_directory(&result.output_dir).is_err());
}

#[test]
fn accepts_legacy_partition_key_and_infers_missing_chip() {
    let temp = TempDir::new().unwrap();
    let build = temp.path().join("build");
    write_idf_build(&build);
    let args_path = build.join("flasher_args.json");
    let legacy = fs::read_to_string(&args_path)
        .unwrap()
        .replace("\"partition-table\"", "\"partition_table\"")
        .replace(
            "\"extra_esptool_args\": {\"chip\":\"esp32\"}",
            "\"extra_esptool_args\": {\"after\":\"hard_reset\"}",
        );
    fs::write(args_path, legacy).unwrap();

    let result = build_package(&options(&temp, BuildKind::Update)).unwrap();
    assert_eq!(result.summary.target_chips, ["esp32"]);
}

#[test]
fn rejects_metadata_and_image_chip_mismatch() {
    let temp = TempDir::new().unwrap();
    let build = temp.path().join("build");
    write_idf_build(&build);
    let args_path = build.join("flasher_args.json");
    let mismatch = fs::read_to_string(&args_path)
        .unwrap()
        .replace("\"chip\":\"esp32\"", "\"chip\":\"esp32s3\"");
    fs::write(args_path, mismatch).unwrap();

    let error = build_package(&options(&temp, BuildKind::Update)).unwrap_err();
    assert_eq!(error.code, ErrorCode::ChipMismatch);
}
