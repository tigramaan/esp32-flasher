import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { cwd } from "node:process";
import { beforeEach, describe, expect, it } from "vitest";
import { initialState } from "./state";
import { Esp32FlasherView } from "./view";

const baseCss = readFileSync(resolve(cwd(), "src/styles/base.css"), "utf8");
const componentsCss = readFileSync(
  resolve(cwd(), "src/styles/components.css"),
  "utf8",
);
const responsiveCss = readFileSync(
  resolve(cwd(), "src/styles/responsive.css"),
  "utf8",
);

describe("responsive layout contract", () => {
  beforeEach(() => {
    document.body.innerHTML = '<div id="app"></div>';
  });

  it("uses shrinkable grid tracks and blocks known horizontal overflow patterns", () => {
    const css = `${baseCss}\n${componentsCss}\n${responsiveCss}`;
    expect(baseCss).toContain("repeat(2, minmax(0, 1fr))");
    expect(baseCss).toContain(".workspace > *");
    expect(componentsCss).toContain("flex: 1 1 auto");
    expect(css).not.toContain("width: 100vw");
  });

  it("defines narrow, short, reduced-motion and mobile target rules", () => {
    expect(responsiveCss).toContain("@media (max-width: 820px), (max-height: 650px)");
    expect(responsiveCss).toContain("@media (max-width: 620px)");
    expect(responsiveCss).toContain("@media (max-width: 360px)");
    expect(responsiveCss).toContain("min-height: 44px");
    expect(responsiveCss).toContain("@media (prefers-reduced-motion: reduce)");
  });

  it("gives every button a visible or explicit accessible name", () => {
    new Esp32FlasherView(document.querySelector("#app"));
    const buttons = [...document.querySelectorAll("button")];
    expect(buttons.length).toBeGreaterThan(0);
    for (const button of buttons) {
      const name =
        button.getAttribute("aria-label") ??
        button.textContent?.trim() ??
        button.getAttribute("title");
      expect(name, button.outerHTML).toBeTruthy();
    }
  });

  it("shows a single BIN file action in update mode", () => {
    const view = new Esp32FlasherView(document.querySelector("#app"));
    view.render(initialState());
    expect(
      document.querySelector('[data-ref="choosePackage"]')?.textContent,
    ).toBe("Выбрать BIN-файл");
    expect(
      document.querySelector('[data-ref="packageMeta"]')?.textContent,
    ).toContain("с любым именем");
  });

  it("keeps UART lines unwrapped inside the terminal scroll container", () => {
    const view = new Esp32FlasherView(document.querySelector("#app"));
    view.render({
      ...initialState(),
      terminalTab: "uart",
      serialLines: ["a".repeat(500)],
      serialCharCount: 500,
      serialPendingCr: false,
    });
    expect(
      document.querySelector('[data-ref="terminalOutput"]')?.classList,
    ).toContain("is-uart");
    expect(componentsCss).toContain(".terminal-output.is-uart");
    expect(componentsCss).toContain("white-space: pre;");
    expect(componentsCss).toContain("overflow-wrap: normal;");
    expect(componentsCss).toContain("max-width: 100%;");
  });

  it("provides an optional, labelled factory UART marker", () => {
    const view = new Esp32FlasherView(document.querySelector("#app"));
    view.render({
      ...initialState(),
      mode: "factory",
      factorySuccessMarker: "NOVA_READY",
    });
    const input = document.querySelector<HTMLInputElement>(
      '[data-ref="factoryMarker"]',
    );
    expect(input?.value).toBe("NOVA_READY");
    expect(input?.closest("label")?.textContent).toContain(
      "Маркер готовности UART",
    );
    expect(input?.maxLength).toBe(256);
  });

  it("renders direct UART connection, baud and reset controls", () => {
    const view = new Esp32FlasherView(document.querySelector("#app"));
    view.render({
      ...initialState(),
      selectedPort: "COM5",
      terminalTab: "uart",
      monitorStatus: "connected",
      monitorBaud: 230_400,
    });

    expect(
      document.querySelector('[data-ref="monitorToggle"]')?.textContent,
    ).toBe("Отключить");
    expect(
      document.querySelector<HTMLSelectElement>('[data-ref="monitorBaud"]')
        ?.value,
    ).toBe("230400");
    expect(
      document.querySelector<HTMLButtonElement>('[data-ref="monitorReset"]')
        ?.disabled,
    ).toBe(false);
    expect(
      document.querySelector('[data-ref="monitorFooterPort"]')?.textContent,
    ).toContain("COM5");
  });

  it("locks the UART tab and controls during a flash operation", () => {
    const view = new Esp32FlasherView(document.querySelector("#app"));
    view.render({
      ...initialState(),
      selectedPort: "COM5",
      operationActive: true,
      stage: "writing",
      monitorStatus: "busy",
    });

    expect(
      document.querySelector<HTMLButtonElement>('[data-ref="tabUart"]')
        ?.disabled,
    ).toBe(true);
    expect(
      document.querySelector('[data-ref="monitorFooterStatus"]')?.textContent,
    ).toBe("Занят прошивкой");
  });

  it("renders the computed PlatformIO flash map with clear user-facing copy", () => {
    const view = new Esp32FlasherView(document.querySelector("#app"));
    view.render({
      ...initialState(),
      packagePath: "D:\\firmware",
      package: {
        package_id: "nova",
        display_name: "NOVA",
        firmware_version: "sha256-123456789abc",
        kind: "factory",
        target_chips: ["esp32s3"],
        segment_count: 3,
        total_bytes: 1024,
        monitor_baud: 115_200,
        success_timeout_ms: 15_000,
        success_marker_configured: false,
        source: "platformio",
        requires_device_layout: false,
        segments: [
          {
            role: "bootloader",
            file: "bootloader.bin",
            offset: "0x0",
            size: 128,
          },
          {
            role: "partition_table",
            file: "partitions.bin",
            offset: "0x8000",
            size: 128,
          },
          {
            role: "application",
            file: "firmware.bin",
            offset: "0x10000",
            size: 768,
          },
        ],
      },
    });
    const text = document.querySelector('[data-ref="packageMap"]')?.textContent;
    expect(text).toContain("bootloader.bin → 0x0");
    expect(text).toContain("partitions.bin → 0x8000");
    expect(text).toContain("firmware.bin → 0x10000");
    expect(document.body.textContent).toContain(
      "Папка PlatformIO или application BIN",
    );
    expect(document.body.textContent).not.toContain("JSON");
  });

  it("renders the complete English interface without Russian copy", () => {
    const view = new Esp32FlasherView(document.querySelector("#app"), "en");
    view.render(initialState("en"));

    expect(document.body.textContent).toContain("ESP32 Flasher");
    expect(document.body.textContent).toContain("Choose BIN file");
    expect(document.body.textContent).toContain("UART monitor");
    expect(document.body.textContent).not.toMatch(/[А-Яа-яЁё]/);
  });
});
