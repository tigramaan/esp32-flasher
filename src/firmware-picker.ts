import type { AppMode } from "./types";
import {
  createTranslator,
  type UiLanguage,
} from "./i18n";

export interface FirmwarePickerOptions {
  directory: boolean;
  multiple: false;
  title: string;
  filters?: Array<{
    name: string;
    extensions: string[];
  }>;
}

export function firmwarePickerOptions(
  mode: AppMode,
  language: UiLanguage = "ru",
): FirmwarePickerOptions {
  const t = createTranslator(language);
  if (mode === "factory") {
    return {
      directory: true,
      multiple: false,
      title: t("package.picker_factory"),
    };
  }
  return {
    directory: false,
    multiple: false,
    title: t("package.picker_update"),
    filters: [{ name: t("package.picker_filter"), extensions: ["bin"] }],
  };
}
