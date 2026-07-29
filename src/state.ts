import type {
  AppMode,
  BackendError,
  DataDirectoryStatus,
  DetectedDevice,
  FactorySessionSummary,
  MonitorEvent,
  OperationEvent,
  OperationProgress,
  OperationStage,
  PackageSummary,
  PortCandidate,
} from "./types";
import {
  createTranslator,
  localizeBackendError,
  localizeDiagnosticDetail,
  localizeOperationText,
  type UiLanguage,
} from "./i18n";

export const MAX_TERMINAL_LINES = 10_000;
export const MAX_TERMINAL_CHARS = 2_000_000;
export type MonitorStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "error"
  | "busy";

export interface AppState {
  language: UiLanguage;
  mode: AppMode;
  dataDirectory: DataDirectoryStatus;
  ports: PortCandidate[];
  selectedPort: string;
  device?: DetectedDevice;
  packagePath: string;
  package?: PackageSummary;
  validatingPackage: boolean;
  operationActive: boolean;
  awaitingDisconnect: boolean;
  stage: OperationStage;
  statusMessage: string;
  progress?: OperationProgress;
  error?: BackendError;
  processLines: string[];
  serialLines: string[];
  serialCharCount: number;
  serialPendingCr: boolean;
  terminalTab: "process" | "uart";
  monitorStatus: MonitorStatus;
  monitorBaud: number;
  monitorManuallyDisconnected: boolean;
  monitorResetting: boolean;
  monitorError?: BackendError;
  fullErase: boolean;
  factorySuccessMarker: string;
  session?: FactorySessionSummary;
}

export function initialState(language: UiLanguage = "ru"): AppState {
  const t = createTranslator(language);
  return {
    language,
    mode: "update",
    dataDirectory: { writable: false },
    ports: [],
    selectedPort: "",
    packagePath: "",
    validatingPackage: false,
    operationActive: false,
    awaitingDisconnect: false,
    stage: "idle",
    statusMessage: t("status.initial"),
    processLines: [],
    serialLines: [],
    serialCharCount: 0,
    serialPendingCr: false,
    terminalTab: "process",
    monitorStatus: "disconnected",
    monitorBaud: 115_200,
    monitorManuallyDisconnected: false,
    monitorResetting: false,
    fullErase: false,
    factorySuccessMarker: "",
  };
}

export function appendBoundedLines(
  current: readonly string[],
  text: string,
  limit = MAX_TERMINAL_LINES,
): string[] {
  const normalized = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const incoming = normalized.split("\n");
  if (incoming[incoming.length - 1] === "") incoming.pop();
  if (incoming.length >= limit) return incoming.slice(-limit);
  const overflow = Math.max(0, current.length + incoming.length - limit);
  return [...current.slice(overflow), ...incoming];
}

export function appendBoundedStream(
  current: readonly string[],
  currentCharCount: number,
  currentPendingCr: boolean,
  text: string,
  lineLimit = MAX_TERMINAL_LINES,
  charLimit = MAX_TERMINAL_CHARS,
): { lines: string[]; charCount: number; pendingCr: boolean } {
  if (!text || lineLimit < 1 || charLimit < 1) {
    return {
      lines: lineLimit < 1 || charLimit < 1 ? [] : [...current],
      charCount: lineLimit < 1 || charLimit < 1 ? 0 : currentCharCount,
      pendingCr:
        lineLimit < 1 || charLimit < 1 ? false : currentPendingCr,
    };
  }
  const source =
    currentPendingCr && text.startsWith("\n") ? text.slice(1) : text;
  const pendingCr = source.endsWith("\r");
  const normalized = source.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  if (!normalized) {
    return {
      lines: [...current],
      charCount: currentCharCount,
      pendingCr,
    };
  }
  const incoming = normalized.split("\n");
  const merged =
    current.length === 0
      ? incoming
      : [
          ...current.slice(0, -1),
          `${current[current.length - 1]}${incoming[0]}`,
          ...incoming.slice(1),
        ];
  let charCount = currentCharCount + normalized.length;
  let start = Math.max(0, merged.length - lineLimit);
  for (let index = 0; index < start; index += 1) {
    charCount -= merged[index].length + 1;
  }
  while (charCount > charLimit && start < merged.length - 1) {
    charCount -= merged[start].length + 1;
    start += 1;
  }
  const lines = merged.slice(start);
  if (charCount > charLimit && lines.length === 1) {
    lines[0] = lines[0].slice(-charLimit);
    charCount = lines[0].length;
  }
  return { lines, charCount, pendingCr };
}

export function applyOperationEvent(
  state: AppState,
  event: OperationEvent,
): AppState {
  switch (event.type) {
    case "state": {
      const message = localizeOperationText(
        event.message,
        event.state,
        state.language,
      );
      return {
        ...state,
        stage: event.state,
        statusMessage: message,
        error: event.error
          ? localizeBackendError(event.error, state.language)
          : undefined,
        processLines: appendBoundedLines(
          state.processLines,
          `[${event.state.toUpperCase()}] ${message}`,
        ),
      };
    }
    case "progress": {
      const progress = {
        ...event.progress,
        message: localizeOperationText(
          event.progress.message,
          event.progress.stage,
          state.language,
        ),
      };
      return { ...state, stage: progress.stage, progress };
    }
    case "log": {
      const message = localizeOperationText(
        event.entry.message,
        event.entry.stage,
        state.language,
      );
      return {
        ...state,
        processLines: appendBoundedLines(
          state.processLines,
          `${event.entry.timestamp} ${event.entry.level} ${message}`,
        ),
      };
    }
    case "serial": {
      const stream = appendBoundedStream(
        state.serialLines,
        state.serialCharCount,
        state.serialPendingCr,
        event.data.text,
      );
      return {
        ...state,
        monitorStatus: "connected",
        monitorError: undefined,
        serialLines: stream.lines,
        serialCharCount: stream.charCount,
        serialPendingCr: stream.pendingCr,
      };
    }
    case "monitor_disconnected": {
      const t = createTranslator(state.language);
      return {
        ...state,
        awaitingDisconnect: false,
        device: undefined,
        monitorStatus: "disconnected",
        monitorManuallyDisconnected: true,
        stage: "disconnected",
        statusMessage: t("status.next_board"),
        processLines: appendBoundedLines(
          state.processLines,
          `[UART] ${localizeDiagnosticDetail(event.message, state.language) ?? t("monitor.disconnected_error")}`,
        ),
      };
    }
  }
}

export function applyMonitorEvent(
  state: AppState,
  event: MonitorEvent,
): AppState {
  if (event.type === "data") {
    const stream = appendBoundedStream(
      state.serialLines,
      state.serialCharCount,
      state.serialPendingCr,
      event.data.text,
    );
    return {
      ...state,
      monitorStatus: "connected",
      monitorError: undefined,
      serialLines: stream.lines,
      serialCharCount: stream.charCount,
      serialPendingCr: stream.pendingCr,
    };
  }
  const t = createTranslator(state.language);
  return {
    ...state,
    monitorStatus: "disconnected",
    monitorManuallyDisconnected: true,
    monitorError: {
      code: "DEVICE_DISCONNECTED",
      message: t("monitor.disconnected_error"),
      detail: localizeDiagnosticDetail(event.message, state.language),
      retryable: true,
    },
  };
}

export function isMonitorLocked(state: AppState): boolean {
  return state.operationActive;
}

export function normalizeBackendError(
  value: unknown,
  language: UiLanguage = "ru",
): BackendError {
  const t = createTranslator(language);
  if (typeof value === "object" && value !== null) {
    const candidate = value as Partial<BackendError>;
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return localizeBackendError({
        code: candidate.code,
        message: candidate.message,
        detail: typeof candidate.detail === "string" ? candidate.detail : undefined,
        retryable: candidate.retryable === true,
      }, language);
    }
  }
  return localizeBackendError({
    code: "INTERNAL_ERROR",
    message: typeof value === "string" ? value : t("error.unknown"),
    retryable: false,
  }, language);
}

export function canStart(state: AppState): boolean {
  return Boolean(
    state.dataDirectory.writable &&
      state.selectedPort &&
      state.package &&
      state.packagePath &&
      !state.validatingPackage &&
      !state.operationActive &&
      !state.awaitingDisconnect &&
      state.package.kind === state.mode,
  );
}

export function chooseEnumeratedPort(
  ports: readonly PortCandidate[],
  selectedPort: string,
  knownPorts: ReadonlySet<string>,
): string {
  if (selectedPort && ports.some((port) => port.port === selectedPort)) {
    return selectedPort;
  }
  const added = ports.filter((port) => !knownPorts.has(port.port));
  if (added.length === 1) return added[0].port;
  return ports.length === 1 ? ports[0].port : "";
}

export function formatBytes(
  bytes: number,
  language: UiLanguage = "ru",
): string {
  const t = createTranslator(language);
  if (bytes < 1024) return t("unit.bytes", { value: bytes });
  if (bytes < 1024 * 1024) {
    return t("unit.kib", { value: (bytes / 1024).toFixed(1) });
  }
  return t("unit.mib", { value: (bytes / 1024 / 1024).toFixed(2) });
}
