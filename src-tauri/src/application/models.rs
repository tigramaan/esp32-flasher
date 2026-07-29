use programmer_core::{ErrorCode, OperationError, OperationStage, PackageSummary};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppMode {
    Update,
    Factory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableSettings {
    pub schema_version: u32,
    pub mode: AppMode,
    pub last_update_package: Option<String>,
    pub last_factory_package: Option<String>,
    #[serde(default)]
    pub factory_success_marker: String,
    #[serde(default = "default_monitor_baud")]
    pub monitor_baud: u32,
}

impl Default for PortableSettings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            mode: AppMode::Update,
            last_update_package: None,
            last_factory_package: None,
            factory_success_marker: String::new(),
            monitor_baud: default_monitor_baud(),
        }
    }
}

const fn default_monitor_baud() -> u32 {
    115_200
}

#[derive(Debug, Clone, Serialize)]
pub struct DataDirectoryStatus {
    pub path: Option<String>,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortCandidate {
    pub port: String,
    pub description: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub serial_number: Option<String>,
    pub known_bridge: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectedDevice {
    pub port: String,
    pub description: String,
    pub chip: String,
    pub mac: String,
    pub flash_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationProgress {
    pub stage: OperationStage,
    pub current: u64,
    pub total: u64,
    pub percentage: f32,
    pub segment_index: usize,
    pub segment_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationLog {
    pub timestamp: String,
    pub level: String,
    pub stage: OperationStage,
    pub message: String,
    pub error_code: Option<ErrorCode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerialData {
    pub text: String,
    pub base64: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MonitorEvent {
    Data { data: SerialData },
    Disconnected { port: String, message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OperationEvent {
    State {
        operation_id: String,
        state: OperationStage,
        message: String,
        error: Option<OperationError>,
    },
    Progress {
        operation_id: String,
        progress: OperationProgress,
    },
    Log {
        operation_id: String,
        entry: OperationLog,
    },
    Serial {
        operation_id: String,
        data: SerialData,
    },
    MonitorDisconnected {
        operation_id: String,
        port: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationResult {
    pub operation_id: String,
    pub success: bool,
    pub boot_confirmed: bool,
    pub duration_ms: u64,
    pub device: DetectedDevice,
    pub package: PackageSummary,
    pub error: Option<OperationError>,
    pub report_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorySessionSummary {
    pub session_id: String,
    pub started_at: String,
    pub package_id: String,
    pub firmware_version: String,
    pub total: u64,
    pub passed: u64,
    pub failed: u64,
    pub report_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateRequest {
    pub package_path: String,
    pub port: String,
    pub confirm_in_place: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactoryFlashRequest {
    pub package_path: String,
    pub port: String,
    pub full_erase: bool,
    #[serde(default)]
    pub success_marker: String,
}
