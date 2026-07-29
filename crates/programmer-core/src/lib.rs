pub mod error;
pub mod image;
pub mod manifest;
pub mod marker;
pub mod operation;
pub mod ota;
pub mod package;

pub use error::{ErrorCode, OperationError, Result};
pub use image::validate_esp_image;
pub use manifest::{
    ChipFamily, FirmwareManifest, FirmwareSegment, HexAddress, MonitorPolicy, OtaPolicy,
    PackageKind, SegmentRole,
};
pub use marker::MarkerDetector;
pub use operation::{OperationStage, OperationStateMachine};
pub use ota::{
    build_otadata_sector, parse_partition_table, select_factory_application, select_update_layout,
    OtaSwitch, PartitionEntry, UpdateLayout,
};
pub use package::{
    validate_package, validate_relative_path, FirmwareSource, PackageReader, PackageSummary,
    SegmentSummary, ValidatedPackage, MAX_PACKAGE_BYTES, MAX_SEGMENT_BYTES,
};

pub const MANIFEST_FILE_NAME: &str = "firmware.json";
pub const FLASH_BAUD: u32 = 921_600;
