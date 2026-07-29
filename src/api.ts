import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  DataDirectoryStatus,
  FactoryFlashRequest,
  FactorySessionSummary,
  MonitorEvent,
  OperationEvent,
  OperationResult,
  PackageSummary,
  PortableSettings,
  PortCandidate,
  UpdateRequest,
} from "./types";

export function listDevices(): Promise<PortCandidate[]> {
  return invoke("list_devices");
}

export function systemLocale(): Promise<string> {
  return invoke("system_locale");
}

export function startSerialMonitor(
  port: string,
  baud: number,
  onEvent: (event: MonitorEvent) => void,
): Promise<void> {
  const channel = new Channel<MonitorEvent>();
  channel.onmessage = onEvent;
  return invoke("start_serial_monitor", { port, baud, onEvent: channel });
}

export function validatePackage(path: string): Promise<PackageSummary> {
  return invoke("validate_package", { path });
}

export function getDataDirectory(): Promise<DataDirectoryStatus> {
  return invoke("data_directory");
}

export function setDataDirectory(path: string): Promise<DataDirectoryStatus> {
  return invoke("set_data_directory", { path });
}

export function getSettings(): Promise<PortableSettings> {
  return invoke("get_settings");
}

export function saveSettings(
  settings: PortableSettings,
): Promise<PortableSettings> {
  return invoke("update_settings", { settings });
}

export function startFactorySession(
  packagePath: string,
): Promise<FactorySessionSummary> {
  return invoke("start_factory_session", { packagePath });
}

export function getFactorySession(): Promise<
  FactorySessionSummary | undefined
> {
  return invoke("factory_session");
}

export function startUpdate(
  request: UpdateRequest,
  onEvent: (event: OperationEvent) => void,
): Promise<OperationResult> {
  const channel = new Channel<OperationEvent>();
  channel.onmessage = onEvent;
  return invoke("start_update", { request, onEvent: channel });
}

export function startFactoryFlash(
  request: FactoryFlashRequest,
  onEvent: (event: OperationEvent) => void,
): Promise<OperationResult> {
  const channel = new Channel<OperationEvent>();
  channel.onmessage = onEvent;
  return invoke("start_factory_flash", { request, onEvent: channel });
}

export function sendSerial(data: string): Promise<void> {
  return invoke("send_serial", { data });
}

export function resetMonitor(): Promise<void> {
  return invoke("reset_monitor");
}

export function disconnectMonitor(): Promise<void> {
  return invoke("disconnect_monitor");
}
