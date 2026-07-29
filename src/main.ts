import { open } from "@tauri-apps/plugin-dialog";
import {
  disconnectMonitor,
  getDataDirectory,
  getFactorySession,
  getSettings,
  listDevices,
  resetMonitor,
  saveSettings,
  setDataDirectory,
  startFactoryFlash,
  startFactorySession,
  startSerialMonitor,
  startUpdate,
  systemLocale,
  validatePackage,
} from "./api";
import {
  createTranslator,
  detectUiLanguage,
  languageFromLocale,
  type Translator,
  type UiLanguage,
} from "./i18n";
import {
  applyOperationEvent,
  applyMonitorEvent,
  chooseEnumeratedPort,
  initialState,
  isMonitorLocked,
  normalizeBackendError,
  type AppState,
} from "./state";
import { firmwarePickerOptions } from "./firmware-picker";
import type {
  AppMode,
  MonitorEvent,
  OperationEvent,
  PortableSettings,
} from "./types";
import { Esp32FlasherView } from "./view";
import "./styles.css";

class Esp32FlasherApp {
  private state: AppState;
  private readonly view: Esp32FlasherView;
  private readonly t: Translator;
  private knownPorts = new Set<string>();
  private pollTimer?: number;
  private portsPolling = false;
  private monitorGeneration = 0;
  private renderPending = false;

  constructor(private readonly language: UiLanguage) {
    this.state = initialState(language);
    this.t = createTranslator(language);
    this.view = new Esp32FlasherView(
      document.querySelector("#app"),
      language,
    );
    document.documentElement.lang = language;
    document.title = this.t("app.title");
  }

  async start(): Promise<void> {
    this.bind();
    this.scheduleRender();
    await this.loadPortableState();
    await this.pollPorts();
    this.pollTimer = window.setInterval(() => void this.pollPorts(), 1000);
  }

  async stop(): Promise<void> {
    if (this.pollTimer !== undefined) {
      window.clearInterval(this.pollTimer);
      this.pollTimer = undefined;
    }
    await disconnectMonitor();
  }

  private bind(): void {
    this.view.onMode((mode) => void this.changeMode(mode));
    this.view.onChooseData(() => void this.chooseDataDirectory());
    this.view.onChoosePackage(() => void this.choosePackage());
    this.view.onPort((port) => this.selectPort(port));
    this.view.onRefresh(() => void this.pollPorts(true));
    this.view.onStart(() => void this.startOperation());
    this.view.onNewSession(() => void this.newFactorySession());
    this.view.onFullErase((enabled) => this.setFullErase(enabled));
    this.view.onFactoryMarker((marker) => void this.setFactoryMarker(marker));
    this.view.onTerminalTab((tab) => void this.changeTerminalTab(tab));
    this.view.onClearTerminal(() => {
      this.state =
        this.state.terminalTab === "uart"
          ? {
              ...this.state,
              serialLines: [],
              serialCharCount: 0,
              serialPendingCr: false,
            }
          : { ...this.state, processLines: [] };
      this.scheduleRender();
    });
    this.view.onMonitorToggle(() => void this.toggleMonitor());
    this.view.onMonitorReset(() => void this.restartMonitorDevice());
    this.view.onMonitorBaud((baud) => void this.changeMonitorBaud(baud));
  }

  private async loadPortableState(): Promise<void> {
    try {
      const dataDirectory = await getDataDirectory();
      this.state = { ...this.state, dataDirectory };
      if (!dataDirectory.writable) return;
      const settings = await getSettings();
      const packagePath =
        settings.mode === "factory"
          ? settings.last_factory_package ?? ""
          : settings.last_update_package ?? "";
      this.state = {
        ...this.state,
        mode: settings.mode,
        packagePath,
        factorySuccessMarker: settings.factory_success_marker,
        monitorBaud: settings.monitor_baud,
      };
      if (packagePath) await this.loadPackage(packagePath, false);
      this.state = {
        ...this.state,
        session: await getFactorySession(),
      };
    } catch (error) {
      this.showError(error);
    } finally {
      this.scheduleRender();
    }
  }

  private async chooseDataDirectory(): Promise<void> {
    const selected = await open({
      directory: true,
      multiple: false,
      title: this.t("data.choose_title"),
    });
    if (typeof selected !== "string") return;
    try {
      const dataDirectory = await setDataDirectory(selected);
      this.state = { ...this.state, dataDirectory };
      await saveSettings(this.settings());
      this.toast(this.t("data.saved"), "success");
    } catch (error) {
      this.showError(error);
    }
    this.scheduleRender();
  }

  private async changeMode(mode: AppMode): Promise<void> {
    if (this.state.operationActive || this.state.mode === mode) return;
    this.state = {
      ...this.state,
      mode,
      package: undefined,
      packagePath: "",
      error: undefined,
      fullErase: false,
      statusMessage:
        mode === "factory"
          ? this.t("status.factory_choose")
          : this.t("status.update_choose"),
    };
    const settings = await this.safeSettings();
    const remembered =
      mode === "factory"
        ? settings?.last_factory_package
        : settings?.last_update_package;
    if (remembered) await this.loadPackage(remembered, false);
    await this.persistSettings();
    this.scheduleRender();
  }

  private async choosePackage(): Promise<void> {
    const selected = await open(
      firmwarePickerOptions(this.state.mode, this.language),
    );
    if (typeof selected === "string") await this.loadPackage(selected, true);
  }

  private async loadPackage(path: string, notify: boolean): Promise<void> {
    this.state = {
      ...this.state,
      validatingPackage: true,
      packagePath: path,
      package: undefined,
      error: undefined,
    };
    this.scheduleRender();
    try {
      const summary = await validatePackage(path);
      if (summary.kind !== this.state.mode) {
        throw {
          code: "PACKAGE_INVALID",
          message:
            this.state.mode === "factory"
              ? this.t("package.invalid_factory")
              : this.t("package.invalid_update"),
          retryable: false,
        };
      }
      this.state = {
        ...this.state,
        package: summary,
        statusMessage: this.t("status.package_ready", {
          name: summary.display_name,
        }),
      };
      await this.persistSettings();
      if (notify) {
        this.toast(
          this.state.mode === "factory"
            ? this.t("package.checked_factory")
            : this.t("package.checked_update"),
          "success",
        );
      }
    } catch (error) {
      this.state = {
        ...this.state,
        packagePath: "",
        package: undefined,
        error: normalizeBackendError(error, this.language),
      };
    } finally {
      this.state = { ...this.state, validatingPackage: false };
      this.scheduleRender();
    }
  }

  private async pollPorts(force = false): Promise<void> {
    if (this.portsPolling) return;
    this.portsPolling = true;
    try {
      const ports = await listDevices();
      const current = new Set(ports.map((port) => port.port));
      const selectedRemoved =
        this.state.selectedPort && !current.has(this.state.selectedPort);
      const selectedPort = chooseEnumeratedPort(
        ports,
        selectedRemoved ? "" : this.state.selectedPort,
        this.knownPorts,
      );
      this.knownPorts = current;
      if (selectedRemoved) {
        this.monitorGeneration += 1;
        this.state = {
          ...this.state,
          device: undefined,
          awaitingDisconnect: false,
          monitorStatus: "disconnected",
          monitorManuallyDisconnected: true,
          monitorError: undefined,
          stage: "disconnected",
          statusMessage: this.t("status.board_disconnected"),
        };
      }
      this.state = {
        ...this.state,
        ports,
        selectedPort,
      };
      if (
        selectedPort &&
        selectedPort !== this.state.device?.port &&
        !this.state.operationActive &&
        !this.state.awaitingDisconnect
      ) {
        this.state = {
          ...this.state,
          device: undefined,
          error: force ? undefined : this.state.error,
          statusMessage: this.t("status.port_selected", {
            port: selectedPort,
          }),
        };
      }
      this.scheduleRender();
      if (
        this.state.terminalTab === "uart" &&
        selectedPort &&
        this.state.monitorStatus === "disconnected" &&
        !this.state.monitorManuallyDisconnected &&
        !this.state.operationActive
      ) {
        await this.connectMonitor(false);
      }
    } catch (error) {
      if (force) this.showError(error);
    } finally {
      this.portsPolling = false;
    }
  }

  private async selectPort(port: string): Promise<void> {
    if (this.state.operationActive) return;
    const reconnect =
      this.state.terminalTab === "uart" &&
      !this.state.monitorManuallyDisconnected &&
      Boolean(port);
    if (
      this.state.monitorStatus === "connected" ||
      this.state.monitorStatus === "connecting"
    ) {
      await this.closeMonitor(false);
    }
    const candidate = this.state.ports.find((item) => item.port === port);
    this.state = {
      ...this.state,
      selectedPort: port,
      device: undefined,
      error: undefined,
      monitorError: undefined,
      statusMessage: candidate
        ? this.t("status.port_selected", { port })
        : this.t("status.select_port"),
    };
    this.scheduleRender();
    if (reconnect) await this.connectMonitor(false);
  }

  private async startOperation(confirmInPlace = false): Promise<void> {
    const { selectedPort, packagePath } = this.state;
    if (!selectedPort || !packagePath || this.state.operationActive) return;
    this.monitorGeneration += 1;
    this.state = {
      ...this.state,
      operationActive: true,
      progress: undefined,
      error: undefined,
      processLines: [],
      serialLines: [],
      serialCharCount: 0,
      serialPendingCr: false,
      terminalTab: "process",
      monitorStatus: "busy",
      monitorBaud: this.state.package?.monitor_baud ?? this.state.monitorBaud,
      monitorManuallyDisconnected: false,
      monitorError: undefined,
    };
    this.scheduleRender();
    try {
      if (this.state.mode === "factory") {
        if (!this.state.session || !this.sessionMatchesPackage()) {
          this.state = {
            ...this.state,
            session: await startFactorySession(packagePath),
          };
        }
        const result = await startFactoryFlash(
          {
            package_path: packagePath,
            port: selectedPort,
            full_erase: this.state.fullErase,
            success_marker: this.state.factorySuccessMarker,
          },
          (event) => this.onOperationEvent(event),
        );
        this.state = {
          ...this.state,
          awaitingDisconnect: this.state.monitorStatus === "connected",
          device: result.device,
          error: result.error,
          session: await getFactorySession(),
          terminalTab:
            this.state.monitorStatus === "connected" ? "uart" : "process",
        };
      } else {
        const result = await startUpdate(
          {
            package_path: packagePath,
            port: selectedPort,
            confirm_in_place: confirmInPlace,
          },
          (event) => this.onOperationEvent(event),
        );
        this.state = {
          ...this.state,
          awaitingDisconnect: this.state.monitorStatus === "connected",
          device: result.device,
          error: result.error,
          terminalTab:
            this.state.monitorStatus === "connected" ? "uart" : "process",
        };
      }
    } catch (error) {
      const normalized = normalizeBackendError(error, this.language);
      if (
        this.state.mode === "update" &&
        normalized.code === "IN_PLACE_CONFIRMATION_REQUIRED" &&
        !confirmInPlace
      ) {
        this.state = {
          ...this.state,
          operationActive: false,
          monitorStatus: "disconnected",
        };
        if (
          window.confirm(
            this.t("confirm.in_place"),
          )
        ) {
          await this.startOperation(true);
          return;
        }
      } else {
        this.state = {
          ...this.state,
          error: normalized,
          stage: "failed",
          statusMessage: normalized.message,
          monitorStatus: "disconnected",
        };
      }
    } finally {
      this.state = {
        ...this.state,
        operationActive: false,
        monitorStatus:
          this.state.monitorStatus === "busy"
            ? "disconnected"
            : this.state.monitorStatus,
      };
      this.scheduleRender();
    }
  }

  private onOperationEvent(event: OperationEvent): void {
    this.state = applyOperationEvent(this.state, event);
    if (
      event.type === "serial" ||
      (event.type === "state" && event.state === "monitoring")
    ) {
      this.state = {
        ...this.state,
        monitorStatus: "connected",
        monitorManuallyDisconnected: false,
        monitorError: undefined,
      };
    }
    this.scheduleRender();
  }

  private async newFactorySession(): Promise<void> {
    if (!this.state.packagePath || this.state.mode !== "factory") return;
    try {
      this.state = {
        ...this.state,
        session: await startFactorySession(this.state.packagePath),
      };
      this.toast(this.t("factory.session_started"), "success");
    } catch (error) {
      this.showError(error);
    }
    this.scheduleRender();
  }

  private setFullErase(enabled: boolean): void {
    if (
      enabled &&
      !window.confirm(
        this.t("confirm.full_erase"),
      )
    ) {
      this.state = { ...this.state, fullErase: false };
    } else {
      this.state = { ...this.state, fullErase: enabled };
    }
    this.scheduleRender();
  }

  private async changeTerminalTab(tab: "process" | "uart"): Promise<void> {
    if (tab === "uart" && isMonitorLocked(this.state)) return;
    this.state = { ...this.state, terminalTab: tab };
    this.scheduleRender();
    if (
      tab === "uart" &&
      this.state.selectedPort &&
      this.state.monitorStatus === "disconnected" &&
      !this.state.monitorManuallyDisconnected
    ) {
      await this.connectMonitor(false);
    }
  }

  private async toggleMonitor(): Promise<void> {
    if (
      this.state.monitorStatus === "connected" ||
      this.state.monitorStatus === "connecting"
    ) {
      await this.closeMonitor(true);
    } else {
      await this.connectMonitor(true);
    }
  }

  private async connectMonitor(userInitiated: boolean): Promise<void> {
    const { selectedPort, monitorBaud } = this.state;
    if (
      !selectedPort ||
      this.state.operationActive ||
      this.state.monitorStatus === "connecting" ||
      this.state.monitorStatus === "connected" ||
      (!userInitiated && this.state.monitorManuallyDisconnected)
    ) {
      return;
    }
    const generation = ++this.monitorGeneration;
    this.state = {
      ...this.state,
      monitorStatus: "connecting",
      monitorManuallyDisconnected: false,
      monitorError: undefined,
    };
    this.scheduleRender();
    try {
      await startSerialMonitor(selectedPort, monitorBaud, (event) =>
        this.onMonitorEvent(generation, selectedPort, event),
      );
      if (generation !== this.monitorGeneration || this.state.operationActive) return;
      this.state = {
        ...this.state,
        monitorStatus: "connected",
        monitorError: undefined,
      };
    } catch (error) {
      if (generation !== this.monitorGeneration) return;
      const monitorError = normalizeBackendError(error, this.language);
      this.state = {
        ...this.state,
        monitorStatus: "error",
        monitorManuallyDisconnected: true,
        monitorError,
      };
      this.toast(monitorError.message, "error");
    }
    this.scheduleRender();
  }

  private onMonitorEvent(
    generation: number,
    port: string,
    event: MonitorEvent,
  ): void {
    if (
      generation !== this.monitorGeneration ||
      this.state.operationActive ||
      port !== this.state.selectedPort
    ) {
      return;
    }
    this.state = applyMonitorEvent(this.state, event);
    if (event.type === "disconnected") {
      this.toast(this.t("monitor.disconnected_error"), "error");
    }
    this.scheduleRender();
  }

  private async closeMonitor(manual: boolean): Promise<void> {
    if (this.state.operationActive) return;
    this.monitorGeneration += 1;
    try {
      await disconnectMonitor();
      this.state = {
        ...this.state,
        awaitingDisconnect:
          this.state.mode === "factory" && this.state.awaitingDisconnect,
        monitorStatus: "disconnected",
        monitorManuallyDisconnected: manual,
        monitorError: undefined,
        stage:
          this.state.mode === "factory" && this.state.awaitingDisconnect
            ? this.state.stage
            : "disconnected",
        statusMessage:
          this.state.mode === "factory" && this.state.awaitingDisconnect
            ? this.t("status.disconnect_for_next")
            : this.t("status.monitor_disconnected"),
      };
    } catch (error) {
      this.state = {
        ...this.state,
        monitorStatus: "error",
        monitorManuallyDisconnected: true,
        monitorError: normalizeBackendError(error, this.language),
      };
    }
    this.scheduleRender();
  }

  private async restartMonitorDevice(): Promise<void> {
    if (
      this.state.operationActive ||
      this.state.monitorStatus !== "connected" ||
      this.state.monitorResetting
    ) {
      return;
    }
    this.state = { ...this.state, monitorResetting: true, monitorError: undefined };
    this.scheduleRender();
    try {
      await resetMonitor();
      this.toast(this.t("monitor.board_restarted"), "success");
    } catch (error) {
      const monitorError = normalizeBackendError(error, this.language);
      this.state = { ...this.state, monitorError };
      this.toast(monitorError.message, "error");
    } finally {
      this.state = { ...this.state, monitorResetting: false };
      this.scheduleRender();
    }
  }

  private async changeMonitorBaud(baud: number): Promise<void> {
    if (this.state.operationActive || baud === this.state.monitorBaud) return;
    const reconnect =
      this.state.monitorStatus === "connected" ||
      this.state.monitorStatus === "connecting";
    if (reconnect) await this.closeMonitor(false);
    this.state = {
      ...this.state,
      monitorBaud: baud,
      monitorManuallyDisconnected: false,
      monitorError: undefined,
    };
    await this.persistSettings();
    this.scheduleRender();
    if (reconnect) await this.connectMonitor(true);
  }

  private async setFactoryMarker(marker: string): Promise<void> {
    this.state = {
      ...this.state,
      factorySuccessMarker: marker.slice(0, 256),
    };
    await this.persistSettings();
    this.scheduleRender();
  }

  private sessionMatchesPackage(): boolean {
    return Boolean(
      this.state.session &&
        this.state.package &&
        this.state.session.package_id === this.state.package.package_id &&
        this.state.session.firmware_version ===
          this.state.package.firmware_version,
    );
  }

  private settings(): PortableSettings {
    return {
      schema_version: 1,
      mode: this.state.mode,
      last_update_package:
        this.state.mode === "update"
          ? this.state.packagePath || undefined
          : undefined,
      last_factory_package:
        this.state.mode === "factory"
          ? this.state.packagePath || undefined
          : undefined,
      factory_success_marker: this.state.factorySuccessMarker,
      monitor_baud: this.state.monitorBaud,
    };
  }

  private async safeSettings(): Promise<PortableSettings | undefined> {
    if (!this.state.dataDirectory.writable) return undefined;
    try {
      return await getSettings();
    } catch {
      return undefined;
    }
  }

  private async persistSettings(): Promise<void> {
    if (!this.state.dataDirectory.writable) return;
    const previous = await this.safeSettings();
    await saveSettings({
      schema_version: 1,
      mode: this.state.mode,
      last_update_package:
        this.state.mode === "update"
          ? this.state.packagePath || previous?.last_update_package
          : previous?.last_update_package,
      last_factory_package:
        this.state.mode === "factory"
          ? this.state.packagePath || previous?.last_factory_package
          : previous?.last_factory_package,
      factory_success_marker: this.state.factorySuccessMarker,
      monitor_baud: this.state.monitorBaud,
    });
  }

  private showError(value: unknown): void {
    const error = normalizeBackendError(value, this.language);
    this.state = { ...this.state, error, statusMessage: error.message };
    this.toast(error.message, "error");
    this.scheduleRender();
  }

  private toast(message: string, kind: "success" | "error"): void {
    this.view.toast(message, kind);
  }

  private scheduleRender(): void {
    if (this.renderPending) return;
    this.renderPending = true;
    requestAnimationFrame(() => {
      this.renderPending = false;
      this.view.render(this.state);
    });
  }
}

async function detectLanguage(): Promise<UiLanguage> {
  try {
    return languageFromLocale(await systemLocale());
  } catch {
    return detectUiLanguage();
  }
}

void detectLanguage().then((language) => {
  const app = new Esp32FlasherApp(language);
  void app.start();
  window.addEventListener("beforeunload", () => {
    void app.stop();
  });
});
