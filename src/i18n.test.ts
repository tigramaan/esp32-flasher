import { describe, expect, it } from "vitest";
import {
  languageFromLocale,
  localizeBackendError,
  localizeOperationText,
} from "./i18n";

describe("Windows locale and UI localization", () => {
  it("uses Russian only for a Russian locale and English for every fallback", () => {
    expect(languageFromLocale("ru-RU")).toBe("ru");
    expect(languageFromLocale("ru")).toBe("ru");
    expect(languageFromLocale("en-US")).toBe("en");
    expect(languageFromLocale("de-DE")).toBe("en");
    expect(languageFromLocale(undefined)).toBe("en");
  });

  it("localizes stable backend errors without leaking Russian diagnostics", () => {
    expect(
      localizeBackendError(
        {
          code: "PORT_BUSY",
          message: "Не удалось открыть UART-монитор",
          detail: "Порт занят другой программой",
          retryable: true,
        },
        "en",
      ),
    ).toEqual({
      code: "PORT_BUSY",
      message: "The COM port is busy or cannot be opened",
      detail: undefined,
      retryable: true,
    });
  });

  it("never exposes legacy JSON diagnostics in either interface language", () => {
    for (const language of ["ru", "en"] as const) {
      const localized = localizeBackendError(
        {
          code: "PACKAGE_INVALID",
          message: "firmware.json повреждён",
          detail: "firmware.json превышает допустимый размер",
          retryable: false,
        },
        language,
      );

      expect(localized.message).not.toMatch(/json/i);
      expect(localized.detail).toBeUndefined();
    }
  });

  it("localizes operation progress and keeps dynamic values", () => {
    expect(localizeOperationText("Сегмент 2/3", "writing", "en")).toBe(
      "Segment 2/3",
    );
    expect(
      localizeOperationText(
        "Запись 4096 байт по адресу 0x00010000",
        "writing",
        "en",
      ),
    ).toBe("Writing 4096 bytes at 0x00010000");
  });
});
