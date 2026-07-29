use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, OperationError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    PackageInvalid,
    PackageUnsupported,
    PackagePathInvalid,
    PackageFileMissing,
    HashMismatch,
    PortBusy,
    DeviceNotFound,
    ChipMismatch,
    FlashConnectFailed,
    FlashEraseFailed,
    FlashWriteFailed,
    FlashVerifyFailed,
    PartitionInvalid,
    OtaStateInvalid,
    BootMarkerTimeout,
    DeviceDisconnected,
    DataDirectoryUnwritable,
    OperationInProgress,
    InvalidState,
    InPlaceConfirmationRequired,
    IoError,
    InternalError,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PackageInvalid => "PACKAGE_INVALID",
            Self::PackageUnsupported => "PACKAGE_UNSUPPORTED",
            Self::PackagePathInvalid => "PACKAGE_PATH_INVALID",
            Self::PackageFileMissing => "PACKAGE_FILE_MISSING",
            Self::HashMismatch => "HASH_MISMATCH",
            Self::PortBusy => "PORT_BUSY",
            Self::DeviceNotFound => "DEVICE_NOT_FOUND",
            Self::ChipMismatch => "CHIP_MISMATCH",
            Self::FlashConnectFailed => "FLASH_CONNECT_FAILED",
            Self::FlashEraseFailed => "FLASH_ERASE_FAILED",
            Self::FlashWriteFailed => "FLASH_WRITE_FAILED",
            Self::FlashVerifyFailed => "FLASH_VERIFY_FAILED",
            Self::PartitionInvalid => "PARTITION_INVALID",
            Self::OtaStateInvalid => "OTA_STATE_INVALID",
            Self::BootMarkerTimeout => "BOOT_MARKER_TIMEOUT",
            Self::DeviceDisconnected => "DEVICE_DISCONNECTED",
            Self::DataDirectoryUnwritable => "DATA_DIRECTORY_UNWRITABLE",
            Self::OperationInProgress => "OPERATION_IN_PROGRESS",
            Self::InvalidState => "INVALID_STATE",
            Self::InPlaceConfirmationRequired => "IN_PLACE_CONFIRMATION_REQUIRED",
            Self::IoError => "IO_ERROR",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{code}: {message}")]
#[serde(deny_unknown_fields)]
pub struct OperationError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub retryable: bool,
}

impl OperationError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
            retryable: false,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn retryable(mut self, value: bool) -> Self {
        self.retryable = value;
        self
    }
}
