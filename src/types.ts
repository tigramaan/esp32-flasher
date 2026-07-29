export type AppMode = "update" | "factory";

export type OperationStage =
  | "idle"
  | "validating"
  | "detecting"
  | "connecting"
  | "erasing"
  | "writing"
  | "verifying"
  | "resetting"
  | "monitoring"
  | "passed"
  | "failed"
  | "disconnected";

export interface BackendError {
  code: string;
  message: string;
  detail?: string;
  retryable: boolean;
}

export interface DataDirectoryStatus {
  path?: string;
  writable: boolean;
}

export interface PortableSettings {
  schema_version: number;
  mode: AppMode;
  last_update_package?: string;
  last_factory_package?: string;
  factory_success_marker: string;
  monitor_baud: number;
}

export interface PortCandidate {
  port: string;
  description: string;
  vid?: number;
  pid?: number;
  serial_number?: string;
  known_bridge: boolean;
}

export interface DetectedDevice {
  port: string;
  description: string;
  chip: string;
  mac: string;
  flash_size_bytes: number;
}

export interface PackageSummary {
  package_id: string;
  display_name: string;
  firmware_version: string;
  kind: AppMode;
  target_chips: string[];
  segment_count: number;
  total_bytes: number;
  monitor_baud: number;
  success_timeout_ms: number;
  success_marker_configured: boolean;
  source: "platformio" | "standalone" | "legacy_manifest";
  requires_device_layout: boolean;
  segments: Array<{
    role: "bootloader" | "partition_table" | "application" | "ota_data" | "data";
    file: string;
    offset?: string;
    size: number;
  }>;
}

export interface OperationProgress {
  stage: OperationStage;
  current: number;
  total: number;
  percentage: number;
  segment_index: number;
  segment_count: number;
  message: string;
}

export interface OperationLog {
  timestamp: string;
  level: string;
  stage: OperationStage;
  message: string;
  error_code?: string;
}

export type OperationEvent =
  | {
      type: "state";
      operation_id: string;
      state: OperationStage;
      message: string;
      error?: BackendError;
    }
  | {
      type: "progress";
      operation_id: string;
      progress: OperationProgress;
    }
  | {
      type: "log";
      operation_id: string;
      entry: OperationLog;
    }
  | {
      type: "serial";
      operation_id: string;
      data: { text: string; base64: string };
    }
  | {
      type: "monitor_disconnected";
      operation_id: string;
      port: string;
      message: string;
    };

export type MonitorEvent =
  | {
      type: "data";
      data: { text: string; base64: string };
    }
  | {
      type: "disconnected";
      port: string;
      message: string;
    };

export interface OperationResult {
  operation_id: string;
  success: boolean;
  boot_confirmed: boolean;
  duration_ms: number;
  device: DetectedDevice;
  package: PackageSummary;
  error?: BackendError;
  report_path?: string;
}

export interface FactorySessionSummary {
  session_id: string;
  started_at: string;
  package_id: string;
  firmware_version: string;
  total: number;
  passed: number;
  failed: number;
  report_path: string;
}

export interface UpdateRequest {
  package_path: string;
  port: string;
  confirm_in_place: boolean;
}

export interface FactoryFlashRequest {
  package_path: string;
  port: string;
  full_erase: boolean;
  success_marker: string;
}
