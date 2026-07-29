use programmer_core::{validate_relative_path, ErrorCode, FLASH_BAUD};

#[test]
fn flash_baud_is_fixed_without_fallback() {
    assert_eq!(FLASH_BAUD, 921_600);
}

#[test]
fn package_paths_cannot_escape_the_selected_directory() {
    for value in [
        "../secret.bin",
        "nested/../../secret.bin",
        r"C:\secret.bin",
        r"nested\secret.bin",
        "/absolute.bin",
    ] {
        let error = validate_relative_path(value).unwrap_err();
        assert_eq!(error.code, ErrorCode::PackagePathInvalid, "{value}");
    }
}

#[test]
fn runtime_sources_have_no_network_client() {
    let runtime_sources = [
        include_str!("../../../src-tauri/src/application/update.rs"),
        include_str!("../../../src-tauri/src/application/factory.rs"),
        include_str!("../../../src-tauri/src/platform/esp.rs"),
        include_str!("../../../src-tauri/src/platform/package.rs"),
        include_str!("../../../src-tauri/src/platform/serial.rs"),
    ];
    for source in runtime_sources {
        for forbidden in ["reqwest", "ureq", "TcpStream", "http://", "https://"] {
            assert!(
                !source.contains(forbidden),
                "runtime source contains forbidden network token {forbidden}"
            );
        }
    }
}
