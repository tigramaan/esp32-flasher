import { describe, expect, it } from "vitest";
import { firmwarePickerOptions } from "./firmware-picker";

describe("firmware picker contract", () => {
  it("selects one BIN file for update", () => {
    expect(firmwarePickerOptions("update")).toEqual({
      directory: false,
      multiple: false,
      title: "Выберите application BIN",
      filters: [{ name: "Прошивка ESP32", extensions: ["bin"] }],
    });
  });

  it("keeps the PlatformIO directory picker for factory", () => {
    expect(firmwarePickerOptions("factory")).toEqual({
      directory: true,
      multiple: false,
      title: "Выберите папку PlatformIO",
    });
  });

  it("uses English picker copy for every non-Russian Windows locale", () => {
    expect(firmwarePickerOptions("update", "en")).toEqual({
      directory: false,
      multiple: false,
      title: "Choose an application BIN",
      filters: [{ name: "ESP32 firmware", extensions: ["bin"] }],
    });
  });
});
