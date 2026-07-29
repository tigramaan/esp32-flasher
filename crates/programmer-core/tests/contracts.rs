use programmer_core::{ErrorCode, OperationError};

#[test]
fn error_code_serialization_is_stable() {
    let error = OperationError::new(ErrorCode::ChipMismatch, "Другой чип");
    let json = serde_json::to_value(error).unwrap();
    assert_eq!(json["code"], "CHIP_MISMATCH");
    assert_eq!(json["retryable"], false);
}

#[test]
fn every_error_code_has_uppercase_public_name() {
    let codes = [
        ErrorCode::PackageInvalid,
        ErrorCode::PackageUnsupported,
        ErrorCode::PackagePathInvalid,
        ErrorCode::PackageFileMissing,
        ErrorCode::HashMismatch,
        ErrorCode::PortBusy,
        ErrorCode::DeviceNotFound,
        ErrorCode::ChipMismatch,
        ErrorCode::FlashConnectFailed,
        ErrorCode::FlashEraseFailed,
        ErrorCode::FlashWriteFailed,
        ErrorCode::FlashVerifyFailed,
        ErrorCode::PartitionInvalid,
        ErrorCode::OtaStateInvalid,
        ErrorCode::BootMarkerTimeout,
        ErrorCode::DeviceDisconnected,
        ErrorCode::DataDirectoryUnwritable,
        ErrorCode::OperationInProgress,
        ErrorCode::InvalidState,
        ErrorCode::InPlaceConfirmationRequired,
        ErrorCode::IoError,
        ErrorCode::InternalError,
    ];
    for code in codes {
        assert!(code
            .as_str()
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte == b'_'));
    }
}
