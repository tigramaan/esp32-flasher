import { describe, expect, it } from "vitest";
import {
  appendBoundedLines,
  appendBoundedStream,
  applyMonitorEvent,
  applyOperationEvent,
  canStart,
  chooseEnumeratedPort,
  initialState,
  isMonitorLocked,
  normalizeBackendError,
} from "./state";

describe("state guards", () => {
  it("keeps terminal buffer bounded", () => {
    const result = appendBoundedLines(["old"], "a\nb\nc", 3);
    expect(result).toEqual(["a", "b", "c"]);
  });

  it("handles sustained UART data within the UI budget", () => {
    const payload = `${"UART line\n".repeat(20_000)}`;
    const started = performance.now();
    const result = appendBoundedStream([], 0, false, payload);
    expect(result.lines).toHaveLength(10_000);
    expect(performance.now() - started).toBeLessThan(250);
  });

  it("joins UART read chunks without inventing line breaks", () => {
    let stream = appendBoundedStream([], 0, false, '{"jsonrpc":"2.');
    stream = appendBoundedStream(
      stream.lines,
      stream.charCount,
      stream.pendingCr,
      '0","result":',
    );
    stream = appendBoundedStream(
      stream.lines,
      stream.charCount,
      stream.pendingCr,
      "true}\r\nnext",
    );

    expect(stream.lines).toEqual([
      '{"jsonrpc":"2.0","result":true}',
      "next",
    ]);
    expect(stream.charCount).toBe(
      '{"jsonrpc":"2.0","result":true}\nnext'.length,
    );
  });

  it("keeps a CRLF split between UART chunks as one line break", () => {
    let stream = appendBoundedStream([], 0, false, "first\r");
    stream = appendBoundedStream(
      stream.lines,
      stream.charCount,
      stream.pendingCr,
      "\nsecond",
    );

    expect(stream.lines).toEqual(["first", "second"]);
    expect(stream.charCount).toBe("first\nsecond".length);
    expect(stream.pendingCr).toBe(false);
  });

  it("bounds a UART stream even when firmware never emits a newline", () => {
    const stream = appendBoundedStream(
      [],
      0,
      false,
      "x".repeat(100),
      10,
      32,
    );
    expect(stream.lines).toEqual(["x".repeat(32)]);
    expect(stream.charCount).toBe(32);
  });

  it("requires selected port, package and portable data without an active probe", () => {
    const state = initialState();
    expect(canStart(state)).toBe(false);
    expect(
      canStart({
        ...state,
        dataDirectory: { path: "D:\\data", writable: true },
        selectedPort: "COM5",
        packagePath: "D:\\firmware",
        package: {
          package_id: "nova",
          display_name: "NOVA",
          firmware_version: "1",
          kind: "update",
          target_chips: ["esp32"],
          segment_count: 1,
          total_bytes: 10,
          monitor_baud: 115_200,
          success_timeout_ms: 15_000,
          success_marker_configured: false,
          source: "standalone",
          requires_device_layout: true,
          segments: [
            {
              role: "application",
              file: "firmware.bin",
              size: 10,
            },
          ],
        },
      }),
    ).toBe(true);
  });

  it("auto-selects ports from enumeration without requiring device access", () => {
    const com5 = {
      port: "COM5",
      description: "CP210x",
      known_bridge: true,
    };
    const com8 = {
      port: "COM8",
      description: "USB Serial",
      known_bridge: false,
    };

    expect(chooseEnumeratedPort([com5], "", new Set())).toBe("COM5");
    expect(chooseEnumeratedPort([com5, com8], "", new Set())).toBe("");
    expect(chooseEnumeratedPort([com5, com8], "COM8", new Set(["COM5"]))).toBe(
      "COM8",
    );
    expect(
      chooseEnumeratedPort([com5, com8], "", new Set(["COM5"])),
    ).toBe("COM8");
  });

  it("reduces UART and state events without losing errors", () => {
    let state = initialState();
    state = applyOperationEvent(state, {
      type: "serial",
      operation_id: "1",
      data: { text: "READY\n", base64: "" },
    });
    state = applyOperationEvent(state, {
      type: "state",
      operation_id: "1",
      state: "failed",
      message: "timeout",
      error: {
        code: "BOOT_MARKER_TIMEOUT",
        message: "timeout",
        retryable: false,
      },
    });
    expect(state.serialLines).toContain("READY");
    expect(state.error?.code).toBe("BOOT_MARKER_TIMEOUT");
    expect(normalizeBackendError(state.error).code).toBe("BOOT_MARKER_TIMEOUT");
  });

  it("tracks direct monitor data and explicit disconnect state", () => {
    let state = {
      ...initialState(),
      selectedPort: "COM5",
      terminalTab: "uart" as const,
      monitorStatus: "connecting" as const,
    };
    state = applyMonitorEvent(state, {
      type: "data",
      data: { text: "ready\r\n", base64: "" },
    });
    expect(state.monitorStatus).toBe("connected");
    expect(state.serialLines).toEqual(["ready", ""]);

    state = applyMonitorEvent(state, {
      type: "disconnected",
      port: "COM5",
      message: "device removed",
    });
    expect(state.monitorStatus).toBe("disconnected");
    expect(state.monitorManuallyDisconnected).toBe(true);
    expect(state.monitorError?.detail).toBe("device removed");
  });

  it("locks UART controls for the full flash operation", () => {
    const state = initialState();
    expect(isMonitorLocked(state)).toBe(false);
    expect(
      isMonitorLocked({
        ...state,
        operationActive: true,
        stage: "monitoring",
      }),
    ).toBe(true);
  });
});
