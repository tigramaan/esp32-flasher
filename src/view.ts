import {
  canStart,
  formatBytes,
  isMonitorLocked,
  type AppState,
} from "./state";
import type { AppMode } from "./types";
import {
  createTranslator,
  stageText,
  type Translator,
  type UiLanguage,
} from "./i18n";

type TerminalTab = "process" | "uart";

export class Esp32FlasherView {
  private readonly root: HTMLElement;
  private readonly refs: Record<string, HTMLElement>;
  private readonly language: UiLanguage;
  private readonly t: Translator;

  constructor(root: Element | null, language: UiLanguage = "ru") {
    if (!(root instanceof HTMLElement)) throw new Error("#app not found");
    this.root = root;
    this.language = language;
    this.t = createTranslator(language);
    this.root.innerHTML = template(this.t);
    this.refs = Object.fromEntries(
      [...this.root.querySelectorAll<HTMLElement>("[data-ref]")].map((element) => [
        element.dataset.ref!,
        element,
      ]),
    );
  }

  onMode(handler: (mode: AppMode) => void): void {
    this.refs.modeUpdate.addEventListener("click", () => handler("update"));
    this.refs.modeFactory.addEventListener("click", () => handler("factory"));
  }

  onChooseData(handler: () => void): void {
    this.refs.chooseData.addEventListener("click", handler);
  }

  onChoosePackage(handler: () => void): void {
    this.refs.choosePackage.addEventListener("click", handler);
  }

  onPort(handler: (port: string) => void): void {
    this.refs.portSelect.addEventListener("change", (event) =>
      handler((event.target as HTMLSelectElement).value),
    );
  }

  onRefresh(handler: () => void): void {
    this.refs.refreshPorts.addEventListener("click", handler);
  }

  onStart(handler: () => void): void {
    this.refs.primaryAction.addEventListener("click", handler);
  }

  onNewSession(handler: () => void): void {
    this.refs.newSession.addEventListener("click", handler);
  }

  onFullErase(handler: (value: boolean) => void): void {
    this.refs.fullErase.addEventListener("change", (event) =>
      handler((event.target as HTMLInputElement).checked),
    );
  }

  onFactoryMarker(handler: (value: string) => void): void {
    this.refs.factoryMarker.addEventListener("change", (event) =>
      handler((event.target as HTMLInputElement).value),
    );
  }

  onTerminalTab(handler: (tab: TerminalTab) => void): void {
    this.refs.tabProcess.addEventListener("click", () => handler("process"));
    this.refs.tabUart.addEventListener("click", () => handler("uart"));
  }

  onClearTerminal(handler: () => void): void {
    this.refs.clearTerminal.addEventListener("click", handler);
  }

  onMonitorToggle(handler: () => void): void {
    this.refs.monitorToggle.addEventListener("click", handler);
  }

  onMonitorReset(handler: () => void): void {
    this.refs.monitorReset.addEventListener("click", handler);
  }

  onMonitorBaud(handler: (baud: number) => void): void {
    this.refs.monitorBaud.addEventListener("change", (event) =>
      handler(Number((event.target as HTMLSelectElement).value)),
    );
  }

  render(state: AppState): void {
    this.toggleMode(state);
    this.renderDataDirectory(state);
    this.renderPorts(state);
    this.renderDevice(state);
    this.renderPackage(state);
    this.renderFactory(state);
    this.renderOperation(state);
    this.renderTerminal(state);
  }

  toast(message: string, kind: "success" | "error"): void {
    const element = document.createElement("div");
    element.className = `toast toast--${kind}`;
    element.textContent = message;
    document.body.append(element);
    requestAnimationFrame(() => element.classList.add("toast--visible"));
    window.setTimeout(() => {
      element.classList.remove("toast--visible");
      window.setTimeout(() => element.remove(), 200);
    }, 3200);
  }

  private toggleMode(state: AppState): void {
    this.refs.modeUpdate.classList.toggle("is-active", state.mode === "update");
    this.refs.modeFactory.classList.toggle("is-active", state.mode === "factory");
    this.refs.modeUpdate.setAttribute(
      "aria-pressed",
      String(state.mode === "update"),
    );
    this.refs.modeFactory.setAttribute(
      "aria-pressed",
      String(state.mode === "factory"),
    );
    this.refs.factoryPanel.hidden = state.mode !== "factory";
    this.refs.clientHint.hidden = state.mode !== "update";
  }

  private renderDataDirectory(state: AppState): void {
    this.refs.dataWarning.hidden = state.dataDirectory.writable;
    this.refs.dataPath.textContent =
      state.dataDirectory.path ?? this.t("data.not_selected");
    this.refs.dataPath.title = state.dataDirectory.path ?? "";
  }

  private renderPorts(state: AppState): void {
    const select = this.refs.portSelect as HTMLSelectElement;
    const signature = state.ports
      .map((item) => `${item.port}:${item.description}`)
      .join("|");
    if (select.dataset.signature !== signature) {
      select.replaceChildren(new Option(this.t("port.choose"), ""));
      for (const port of state.ports) {
        const bridge = port.known_bridge ? " • ESP/USB" : "";
        select.add(new Option(`${port.port} — ${port.description}${bridge}`, port.port));
      }
      select.dataset.signature = signature;
    }
    select.value = state.selectedPort;
    select.disabled = state.operationActive;
    this.refs.refreshPorts.classList.remove("is-spinning");
  }

  private renderDevice(state: AppState): void {
    const ready = Boolean(state.device);
    this.refs.deviceCard.classList.toggle("is-ready", ready);
    this.refs.deviceName.textContent =
      state.operationActive &&
      (state.stage === "detecting" || state.stage === "connecting")
      ? this.t("device.detecting")
      : state.device
        ? state.device.chip.toUpperCase()
        : state.selectedPort
          ? this.t("device.selected", { port: state.selectedPort })
          : this.t("device.not_connected");
    this.refs.deviceMeta.textContent = state.device
      ? `${state.device.port} • ${state.device.mac} • ${formatBytes(state.device.flash_size_bytes, this.language)}`
      : state.selectedPort
        ? `${state.ports.find((port) => port.port === state.selectedPort)?.description ?? this.t("device.com_port")} • ${this.t("device.model_on_start")}`
      : state.ports.length > 1
        ? this.t("device.multiple")
        : this.t("device.connect_usb");
  }

  private renderPackage(state: AppState): void {
    this.refs.packageName.textContent = state.validatingPackage
      ? this.t("package.validating")
      : state.package
        ? `${state.package.display_name} ${state.package.firmware_version}`
        : state.mode === "factory"
          ? this.t("package.folder_not_selected")
          : this.t("package.file_not_selected");
    this.refs.packageMeta.textContent = state.package
      ? `${sourceLabel(state.package.source, this.t)} • ${state.package.segment_count} BIN • ${formatBytes(state.package.total_bytes, this.language)} • ${state.package.target_chips.join(", ")}`
      : state.mode === "factory"
        ? "bootloader.bin + partitions.bin + firmware.bin"
        : this.t("package.update_hint");
    this.refs.packageMap.replaceChildren();
    if (state.package) {
      for (const segment of state.package.segments) {
        const item = document.createElement("span");
        item.textContent = `${roleLabel(segment.role)}: ${segment.file} → ${segment.offset ?? this.t("package.address_on_device")}`;
        this.refs.packageMap.append(item);
      }
    }
    this.refs.packageMap.hidden = !state.package;
    this.refs.packagePath.textContent =
      state.packagePath ||
      (state.mode === "factory"
        ? this.t("package.folder_not_selected")
        : this.t("package.file_not_selected"));
    this.refs.packagePath.title = state.packagePath;
    (this.refs.choosePackage as HTMLButtonElement).disabled =
      state.operationActive || state.validatingPackage;
    this.refs.choosePackage.textContent =
      state.mode === "factory"
        ? this.t("package.choose_folder")
        : this.t("package.choose_file");
  }

  private renderFactory(state: AppState): void {
    this.refs.countTotal.textContent = String(state.session?.total ?? 0);
    this.refs.countPassed.textContent = String(state.session?.passed ?? 0);
    this.refs.countFailed.textContent = String(state.session?.failed ?? 0);
    this.refs.reportPath.textContent =
      state.session?.report_path ?? this.t("factory.report_pending");
    this.refs.reportPath.title = state.session?.report_path ?? "";
    (this.refs.fullErase as HTMLInputElement).checked = state.fullErase;
    (this.refs.fullErase as HTMLInputElement).disabled = state.operationActive;
    (this.refs.factoryMarker as HTMLInputElement).value =
      state.factorySuccessMarker;
    (this.refs.factoryMarker as HTMLInputElement).disabled =
      state.operationActive;
    (this.refs.newSession as HTMLButtonElement).disabled =
      !state.package || state.operationActive;
  }

  private renderOperation(state: AppState): void {
    const percent = Math.max(0, Math.min(100, state.progress?.percentage ?? 0));
    this.refs.statusMessage.textContent = state.statusMessage;
    this.refs.statusPill.textContent = stageText(state.stage, this.language);
    this.refs.statusPill.dataset.stage = state.stage;
    this.refs.progressFill.style.width = `${percent}%`;
    this.refs.progressValue.textContent = `${percent.toFixed(0)}%`;
    this.refs.progressMessage.textContent =
      state.progress?.message ?? this.t("operation.waiting");
    this.refs.errorBox.hidden = !state.error;
    this.refs.errorText.textContent = state.error
      ? `${state.error.message}${state.error.detail ? ` — ${state.error.detail}` : ""}`
      : "";

    const primary = this.refs.primaryAction as HTMLButtonElement;
    primary.disabled = !canStart(state);
    primary.textContent = state.operationActive
      ? this.t("operation.running")
      : state.awaitingDisconnect
        ? this.t("operation.disconnect")
        : state.mode === "factory"
          ? this.t("operation.flash")
          : this.t("operation.update");
  }

  private renderTerminal(state: AppState): void {
    const uart = state.terminalTab === "uart";
    const locked = isMonitorLocked(state);
    this.refs.tabProcess.classList.toggle("is-active", !uart);
    this.refs.tabUart.classList.toggle("is-active", uart);
    (this.refs.tabUart as HTMLButtonElement).disabled = locked;
    this.refs.tabUart.setAttribute("aria-disabled", String(locked));
    this.refs.tabUart.title = locked ? this.t("terminal.locked_title") : "";
    this.refs.monitorControls.hidden = !uart;
    const baud = this.refs.monitorBaud as HTMLSelectElement;
    baud.value = String(state.monitorBaud);
    baud.disabled =
      locked ||
      state.monitorStatus === "connecting" ||
      state.monitorStatus === "busy";
    const toggle = this.refs.monitorToggle as HTMLButtonElement;
    toggle.textContent =
      state.monitorStatus === "connected"
        ? this.t("terminal.disconnect")
        : state.monitorStatus === "error"
          ? this.t("terminal.retry")
          : this.t("terminal.connect");
    toggle.disabled =
      locked ||
      !state.selectedPort ||
      state.monitorStatus === "connecting" ||
      state.monitorStatus === "busy";
    const reset = this.refs.monitorReset as HTMLButtonElement;
    reset.textContent = state.monitorResetting
      ? this.t("terminal.resetting")
      : this.t("terminal.reset");
    reset.disabled =
      locked ||
      state.monitorStatus !== "connected" ||
      state.monitorResetting;
    const output = this.refs.terminalOutput;
    output.classList.toggle("is-uart", uart);
    const lines = uart ? state.serialLines : state.processLines;
    const text = lines.join("\n");
    if (output.textContent !== text) {
      const nearBottom =
        output.scrollHeight - output.scrollTop - output.clientHeight < 80;
      output.textContent =
        text ||
        (uart
          ? uartEmptyText(state, this.t)
          : this.t("terminal.empty_process"));
      output.classList.toggle("is-empty", lines.length === 0);
      if (nearBottom) output.scrollTop = output.scrollHeight;
    }
    this.refs.monitorFooterStatus.textContent = monitorStatusLabel(
      state,
      this.t,
    );
    this.refs.monitorStatusDot.dataset.status = state.monitorStatus;
    this.refs.monitorFooterPort.textContent = state.selectedPort
      ? `${state.selectedPort} • ${state.monitorBaud} baud`
      : `${state.monitorBaud} baud`;
  }
}

function template(t: Translator): string {
  return `
    <div class="app-shell">
      <header class="topbar">
        <div class="brand">
          <div class="brand__mark" aria-hidden="true">
            <svg viewBox="0 0 32 32"><path d="M10 5v4M16 5v4M22 5v4M10 23v4M16 23v4M22 23v4M5 10h4M5 16h4M5 22h4M23 10h4M23 16h4M23 22h4"/><rect x="9" y="9" width="14" height="14" rx="3"/><path d="m13 17 2 2 4-5"/></svg>
          </div>
          <div><strong>${t("app.title")}</strong><span>${t("app.subtitle")}</span></div>
        </div>
        <div class="mode-switch" role="group" aria-label="${t("mode.aria")}">
          <button data-ref="modeUpdate" type="button">${t("mode.update")}</button>
          <button data-ref="modeFactory" type="button">${t("mode.factory")}</button>
        </div>
        <div class="status-pill" data-ref="statusPill" data-stage="idle">${t("stage.idle")}</div>
      </header>

      <div class="data-warning" data-ref="dataWarning">
        <div><strong>${t("data.required")}</strong><span>${t("data.description")}</span></div>
        <button class="button button--small" data-ref="chooseData" type="button">${t("data.choose")}</button>
      </div>

      <main class="workspace">
        <section class="control-column">
          <div class="intro">
            <p class="eyebrow" data-ref="clientHint">${t("intro.update")}</p>
            <h1 data-ref="statusMessage">${t("status.initial")}</h1>
            <p class="data-path" data-ref="dataPath"></p>
          </div>

          <div class="cards-grid">
            <article class="card device-card" data-ref="deviceCard">
              <div class="card__heading"><span>1</span><div><strong>${t("device.title")}</strong><small>${t("device.subtitle")}</small></div></div>
              <div class="device-row">
                <div class="device-icon"><i></i></div>
                <div class="device-copy"><strong data-ref="deviceName">${t("device.not_connected")}</strong><span data-ref="deviceMeta">${t("device.connect_usb")}</span></div>
              </div>
              <div class="select-row">
                <select data-ref="portSelect" aria-label="COM port"><option value="">${t("port.choose")}</option></select>
                <button class="icon-button" data-ref="refreshPorts" type="button" title="${t("port.refresh")}" aria-label="${t("port.refresh")}">↻</button>
              </div>
            </article>

            <article class="card package-card">
              <div class="card__heading"><span>2</span><div><strong>${t("package.title")}</strong><small>${t("package.subtitle")}</small></div></div>
              <div class="package-copy"><strong data-ref="packageName">${t("package.file_not_selected")}</strong><span data-ref="packageMeta">${t("package.update_hint")}</span></div>
              <div class="package-map" data-ref="packageMap" hidden></div>
              <div class="path-chip" data-ref="packagePath">${t("package.file_not_selected")}</div>
              <button class="button button--secondary" data-ref="choosePackage" type="button">${t("package.choose_file")}</button>
            </article>
          </div>

          <section class="factory-panel" data-ref="factoryPanel" hidden>
            <div class="factory-stats">
              <div><span>${t("factory.total")}</span><strong data-ref="countTotal">0</strong></div>
              <div class="is-success"><span>OK</span><strong data-ref="countPassed">0</strong></div>
              <div class="is-error"><span>${t("factory.error")}</span><strong data-ref="countFailed">0</strong></div>
            </div>
            <label class="marker-field">
              <span>${t("factory.marker")} <small>${t("factory.optional")}</small></span>
              <input data-ref="factoryMarker" type="text" maxlength="256" placeholder="${t("factory.marker_placeholder")}"/>
            </label>
            <div class="factory-tools">
              <label class="danger-toggle"><input data-ref="fullErase" type="checkbox"/><span>${t("factory.full_erase")}</span></label>
              <button class="text-button" data-ref="newSession" type="button">${t("factory.new_session")}</button>
            </div>
            <div class="report-path" data-ref="reportPath">${t("factory.report_pending")}</div>
          </section>

          <section class="operation-card">
            <div class="progress-meta"><span data-ref="progressMessage">${t("operation.waiting")}</span><strong data-ref="progressValue">0%</strong></div>
            <div class="progress-track"><i data-ref="progressFill"></i></div>
            <div class="error-box" data-ref="errorBox" hidden><span>!</span><p data-ref="errorText"></p></div>
            <button class="primary-action" data-ref="primaryAction" type="button" disabled>${t("operation.update")}</button>
          </section>
        </section>

        <aside class="terminal-card">
          <div class="terminal-header">
            <div class="terminal-tabs">
              <button class="is-active" data-ref="tabProcess" type="button">${t("terminal.process")}</button>
              <button data-ref="tabUart" type="button">${t("terminal.uart")}</button>
            </div>
            <div class="terminal-actions">
              <div class="monitor-controls" data-ref="monitorControls" hidden>
                <select class="terminal-baud" data-ref="monitorBaud" aria-label="${t("terminal.baud")}">
                  <option value="9600">9600</option>
                  <option value="57600">57600</option>
                  <option value="115200">115200</option>
                  <option value="230400">230400</option>
                  <option value="460800">460800</option>
                  <option value="921600">921600</option>
                </select>
                <button class="terminal-tool" data-ref="monitorReset" type="button">${t("terminal.reset")}</button>
                <button class="terminal-tool terminal-tool--primary" data-ref="monitorToggle" type="button">${t("terminal.connect")}</button>
              </div>
              <button class="terminal-clear" data-ref="clearTerminal" type="button">${t("terminal.clear")}</button>
            </div>
          </div>
          <pre class="terminal-output is-empty" data-ref="terminalOutput">${t("terminal.empty_process")}</pre>
          <div class="terminal-footer"><span><i data-ref="monitorStatusDot" data-status="disconnected"></i> <span data-ref="monitorFooterStatus">${t("monitor.disconnected")}</span></span><span data-ref="monitorFooterPort">115200 baud</span></div>
        </aside>
      </main>
      <footer class="app-footer"><span>${t("app.title")} 0.1.0</span><span>${t("app.footer")}</span></footer>
    </div>`;
}

function uartEmptyText(state: AppState, t: Translator): string {
  if (!state.selectedPort) return t("terminal.choose_port");
  if (state.monitorStatus === "busy") {
    return t("terminal.busy");
  }
  if (state.monitorStatus === "connecting") {
    return t("terminal.connecting_to", { port: state.selectedPort });
  }
  if (state.monitorStatus === "error") {
    return state.monitorError
      ? `${state.monitorError.message}${state.monitorError.detail ? ` — ${state.monitorError.detail}` : ""}`
      : t("terminal.open_failed");
  }
  if (state.monitorStatus === "connected") {
    return t("terminal.connected_quiet");
  }
  return state.monitorManuallyDisconnected
    ? t("terminal.manual_disconnect")
    : t("terminal.ready");
}

function monitorStatusLabel(state: AppState, t: Translator): string {
  if (state.monitorResetting) return t("monitor.board_reset");
  return {
    disconnected: t("monitor.disconnected"),
    connecting: t("monitor.connecting"),
    connected: t("monitor.connected"),
    error: t("monitor.error"),
    busy: t("monitor.busy"),
  }[state.monitorStatus];
}

function sourceLabel(
  source: NonNullable<AppState["package"]>["source"],
  t: Translator,
): string {
  return {
    platformio: "PlatformIO",
    standalone: t("package.source_standalone"),
    legacy_manifest: "Legacy manifest",
  }[source];
}

function roleLabel(role: NonNullable<AppState["package"]>["segments"][number]["role"]): string {
  return {
    bootloader: "Bootloader",
    partition_table: "Partitions",
    application: "Application",
    ota_data: "OTA data",
    data: "Data",
  }[role];
}
