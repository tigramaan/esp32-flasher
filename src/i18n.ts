import type {
  BackendError,
  OperationStage,
} from "./types";

export type UiLanguage = "ru" | "en";
export type TranslationParams = Record<string, string | number>;

const EN = {
  "app.title": "ESP32 Flasher",
  "app.subtitle": "ESP32 Flash & Monitor",
  "app.footer": "Flash 921600 baud • Verify enabled",
  "mode.aria": "Operating mode",
  "mode.update": "Update",
  "mode.factory": "Production",
  "data.required": "Working folder required",
  "data.description": "Stores settings, logs, and CSV reports",
  "data.choose": "Choose",
  "data.choose_title": "Choose the ESP32 Flasher working folder",
  "data.not_selected": "Working folder is not selected",
  "data.saved": "Working folder saved",
  "intro.update": "Safe manual firmware update",
  "status.initial": "Connect a board and choose firmware",
  "status.factory_choose": "Choose a PlatformIO folder for the production session",
  "status.update_choose": "Choose an application BIN",
  "status.board_disconnected": "Board disconnected",
  "status.next_board": "Board disconnected — connect the next board",
  "status.port_selected": "{port} selected — ESP32 will be detected when flashing starts",
  "status.select_port": "Choose a COM port",
  "status.package_ready": "{name} firmware is ready",
  "status.disconnect_for_next": "Disconnect the flashed board for the next cycle",
  "status.monitor_disconnected": "UART monitor disconnected",
  "device.title": "Device",
  "device.subtitle": "Automatic ESP32 detection",
  "device.detecting": "Detecting device…",
  "device.selected": "{port} selected",
  "device.not_connected": "ESP32 is not connected",
  "device.com_port": "COM port",
  "device.model_on_start": "ESP32 model will be detected when flashing starts",
  "device.multiple": "Multiple ports found — choose the correct one",
  "device.connect_usb": "Connect the board over USB",
  "port.choose": "Choose a COM port",
  "port.refresh": "Refresh COM ports",
  "package.title": "Firmware",
  "package.subtitle": "Single BIN or PlatformIO folder",
  "package.validating": "Checking firmware…",
  "package.folder_not_selected": "Folder is not selected",
  "package.file_not_selected": "BIN file is not selected",
  "package.update_hint": "Application BIN with any file name",
  "package.address_on_device": "address from device",
  "package.choose_folder": "Choose folder",
  "package.choose_file": "Choose BIN file",
  "package.picker_factory": "Choose a PlatformIO folder",
  "package.picker_update": "Choose an application BIN",
  "package.picker_filter": "ESP32 firmware",
  "package.invalid_factory": "Production mode requires bootloader.bin, partitions.bin, and firmware.bin",
  "package.invalid_update": "Choose an application BIN for update mode",
  "package.checked_factory": "Firmware folder checked",
  "package.checked_update": "BIN file checked",
  "package.source_standalone": "Single BIN",
  "factory.total": "Total",
  "factory.error": "Failed",
  "factory.marker": "UART ready marker",
  "factory.optional": "optional",
  "factory.marker_placeholder": "For example, NOVA_READY",
  "factory.full_erase": "Erase entire flash",
  "factory.new_session": "New session",
  "factory.report_pending": "The report will appear after the session starts",
  "factory.session_started": "New production session started",
  "operation.waiting": "Waiting to start",
  "operation.running": "Working…",
  "operation.disconnect": "Disconnect the board",
  "operation.flash": "Flash board",
  "operation.update": "Update firmware",
  "terminal.process": "Process",
  "terminal.uart": "UART monitor",
  "terminal.clear": "Clear",
  "terminal.baud": "UART baud rate",
  "terminal.reset": "Restart",
  "terminal.resetting": "Restarting…",
  "terminal.connect": "Connect",
  "terminal.disconnect": "Disconnect",
  "terminal.retry": "Retry",
  "terminal.empty_process": "Operation log is empty",
  "terminal.choose_port": "Choose a COM port for the UART monitor",
  "terminal.busy": "UART monitor is temporarily busy flashing",
  "terminal.connecting_to": "Connecting to {port}…",
  "terminal.open_failed": "Could not open the UART monitor",
  "terminal.connected_quiet": "Monitor connected. UART is quiet",
  "terminal.manual_disconnect": "UART monitor disconnected. Click “Connect”",
  "terminal.ready": "UART monitor is ready to connect",
  "terminal.locked_title": "UART is busy flashing",
  "monitor.disconnected": "Monitor disconnected",
  "monitor.connecting": "Connecting…",
  "monitor.connected": "Monitor connected",
  "monitor.error": "Monitor error",
  "monitor.busy": "Busy flashing",
  "monitor.board_reset": "Restarting board",
  "monitor.disconnected_error": "UART monitor disconnected",
  "monitor.board_restarted": "Board restarted",
  "confirm.in_place": "This device has no inactive OTA slot. The update will overwrite its only application partition. Do not disconnect power. Continue?",
  "confirm.full_erase": "Erasing the entire flash removes all data, including NVS. Enable it for this session?",
  "stage.idle": "Waiting",
  "stage.validating": "Validation",
  "stage.detecting": "Device",
  "stage.connecting": "Connecting",
  "stage.erasing": "Erasing",
  "stage.writing": "Writing",
  "stage.verifying": "Flash verify",
  "stage.resetting": "Restarting",
  "stage.monitoring": "UART",
  "stage.passed": "Complete",
  "stage.failed": "Error",
  "stage.disconnected": "Disconnected",
  "progress.validating": "Checking firmware",
  "progress.detecting": "Detecting the connected ESP32",
  "progress.connecting": "Connecting to the ROM bootloader",
  "progress.erasing": "Erasing flash — do not disconnect power",
  "progress.writing": "Writing firmware at 921600 baud",
  "progress.verifying": "Verifying written data",
  "progress.resetting": "Starting the application and UART monitor",
  "progress.monitoring": "UART monitor is open",
  "progress.passed": "Firmware written and verified",
  "progress.failed": "Operation failed",
  "progress.disconnected": "Device disconnected",
  "progress.segment": "Segment {current}/{total}",
  "progress.write_address": "Writing {bytes} bytes at {address}",
  "progress.segment_verified": "Verifying written segment",
  "progress.segment_written": "Segment {current} written",
  "unit.bytes": "{value} B",
  "unit.kib": "{value} KiB",
  "unit.mib": "{value} MiB",
  "error.unknown": "Unknown error",
  "error.PACKAGE_INVALID": "The selected firmware is invalid",
  "error.PACKAGE_UNSUPPORTED": "This firmware format or ESP32 chip is not supported",
  "error.PACKAGE_PATH_INVALID": "The selected firmware path is invalid or unavailable",
  "error.PACKAGE_FILE_MISSING": "A required firmware file is missing",
  "error.HASH_MISMATCH": "A firmware file changed after validation",
  "error.PORT_BUSY": "The COM port is busy or cannot be opened",
  "error.DEVICE_NOT_FOUND": "The ESP32 device or COM port was not found",
  "error.CHIP_MISMATCH": "The firmware is intended for a different ESP32 chip",
  "error.FLASH_CONNECT_FAILED": "Could not connect to the ESP32 bootloader",
  "error.FLASH_ERASE_FAILED": "Could not erase the ESP32 flash",
  "error.FLASH_WRITE_FAILED": "Could not write firmware to flash",
  "error.FLASH_VERIFY_FAILED": "Flash verification failed",
  "error.PARTITION_INVALID": "The ESP32 partition table is invalid or could not be found",
  "error.OTA_STATE_INVALID": "The OTA partition state is invalid",
  "error.BOOT_MARKER_TIMEOUT": "The UART ready marker was not received in time",
  "error.DEVICE_DISCONNECTED": "The ESP32 device was disconnected",
  "error.DATA_DIRECTORY_UNWRITABLE": "The working folder is not writable",
  "error.OPERATION_IN_PROGRESS": "Another operation is already running",
  "error.INVALID_STATE": "This action is not available in the current state",
  "error.IN_PLACE_CONFIRMATION_REQUIRED": "This update requires confirmation because no inactive OTA slot is available",
  "error.IO_ERROR": "A file or device input/output error occurred",
  "error.INTERNAL_ERROR": "An internal error occurred",
} as const;

export type TranslationKey = keyof typeof EN;

const RU: Record<TranslationKey, string> = {
  "app.title": "ESP32 Flasher",
  "app.subtitle": "Прошивка и UART-монитор ESP32",
  "app.footer": "Flash 921600 baud • Проверка включена",
  "mode.aria": "Режим работы",
  "mode.update": "Обновление",
  "mode.factory": "Производство",
  "data.required": "Нужна рабочая папка",
  "data.description": "Для настроек, логов и CSV-отчётов",
  "data.choose": "Выбрать",
  "data.choose_title": "Выберите рабочую папку ESP32 Flasher",
  "data.not_selected": "Рабочая папка не выбрана",
  "data.saved": "Рабочая папка сохранена",
  "intro.update": "Безопасное ручное обновление",
  "status.initial": "Подключите плату и выберите файл прошивки",
  "status.factory_choose": "Выберите папку PlatformIO для производственной сессии",
  "status.update_choose": "Выберите application BIN",
  "status.board_disconnected": "Плата отключена",
  "status.next_board": "Плата отключена — можно подключить следующую",
  "status.port_selected": "{port} выбран — устройство определится при запуске",
  "status.select_port": "Выберите COM-порт",
  "status.package_ready": "Прошивка {name} готова",
  "status.disconnect_for_next": "Отключите прошитую плату для следующего цикла",
  "status.monitor_disconnected": "UART-монитор отключён",
  "device.title": "Устройство",
  "device.subtitle": "Автоопределение ESP32",
  "device.detecting": "Определяем устройство…",
  "device.selected": "{port} выбран",
  "device.not_connected": "ESP32 не подключена",
  "device.com_port": "COM-порт",
  "device.model_on_start": "модель ESP32 определится при запуске",
  "device.multiple": "Найдено несколько портов — выберите нужный",
  "device.connect_usb": "Подключите плату по USB",
  "port.choose": "Выберите COM-порт",
  "port.refresh": "Обновить COM-порты",
  "package.title": "Прошивка",
  "package.subtitle": "Один BIN-файл или папка PlatformIO",
  "package.validating": "Проверяем прошивку…",
  "package.folder_not_selected": "Папка не выбрана",
  "package.file_not_selected": "BIN-файл не выбран",
  "package.update_hint": "Application BIN с любым именем",
  "package.address_on_device": "адрес с платы",
  "package.choose_folder": "Выбрать папку",
  "package.choose_file": "Выбрать BIN-файл",
  "package.picker_factory": "Выберите папку PlatformIO",
  "package.picker_update": "Выберите application BIN",
  "package.picker_filter": "Прошивка ESP32",
  "package.invalid_factory": "Для производства нужны bootloader.bin, partitions.bin и firmware.bin",
  "package.invalid_update": "Для обновления выберите application BIN",
  "package.checked_factory": "Папка прошивки проверена",
  "package.checked_update": "BIN-файл проверен",
  "package.source_standalone": "Один BIN",
  "factory.total": "Всего",
  "factory.error": "Ошибка",
  "factory.marker": "Маркер готовности UART",
  "factory.optional": "необязательно",
  "factory.marker_placeholder": "Например, NOVA_READY",
  "factory.full_erase": "Полное стирание flash",
  "factory.new_session": "Новая сессия",
  "factory.report_pending": "Отчёт появится после начала сессии",
  "factory.session_started": "Новая производственная сессия начата",
  "operation.waiting": "Ожидание запуска",
  "operation.running": "Выполняется…",
  "operation.disconnect": "Отключите плату",
  "operation.flash": "Прошить плату",
  "operation.update": "Обновить прошивку",
  "terminal.process": "Процесс",
  "terminal.uart": "UART монитор",
  "terminal.clear": "Очистить",
  "terminal.baud": "Скорость UART",
  "terminal.reset": "Перезапустить",
  "terminal.resetting": "Перезапуск…",
  "terminal.connect": "Подключить",
  "terminal.disconnect": "Отключить",
  "terminal.retry": "Повторить",
  "terminal.empty_process": "Лог операции пуст",
  "terminal.choose_port": "Выберите COM-порт для UART-монитора",
  "terminal.busy": "UART-монитор временно занят прошивкой",
  "terminal.connecting_to": "Подключение к {port}…",
  "terminal.open_failed": "Не удалось открыть UART-монитор",
  "terminal.connected_quiet": "Монитор подключён. UART пока молчит",
  "terminal.manual_disconnect": "UART-монитор отключён. Нажмите «Подключить»",
  "terminal.ready": "UART-монитор готов к подключению",
  "terminal.locked_title": "UART занят прошивкой",
  "monitor.disconnected": "Монитор отключён",
  "monitor.connecting": "Подключение…",
  "monitor.connected": "Монитор подключён",
  "monitor.error": "Ошибка монитора",
  "monitor.busy": "Занят прошивкой",
  "monitor.board_reset": "Перезапуск платы",
  "monitor.disconnected_error": "UART-монитор отключён",
  "monitor.board_restarted": "Плата перезапущена",
  "confirm.in_place": "На устройстве нет свободного неактивного OTA-слота. Обновление будет записано поверх единственного application-раздела. Не отключайте питание. Продолжить?",
  "confirm.full_erase": "Полное стирание удалит всё содержимое flash, включая NVS. Включить для текущей сессии?",
  "stage.idle": "Ожидание",
  "stage.validating": "Проверка",
  "stage.detecting": "Устройство",
  "stage.connecting": "Подключение",
  "stage.erasing": "Стирание",
  "stage.writing": "Запись",
  "stage.verifying": "Проверка flash",
  "stage.resetting": "Перезапуск",
  "stage.monitoring": "UART",
  "stage.passed": "Готово",
  "stage.failed": "Ошибка",
  "stage.disconnected": "Отключено",
  "progress.validating": "Проверка прошивки",
  "progress.detecting": "Определение подключённой ESP32",
  "progress.connecting": "Подключение к ROM bootloader",
  "progress.erasing": "Стирание flash — не отключайте питание",
  "progress.writing": "Запись прошивки на скорости 921600",
  "progress.verifying": "Проверка записанных данных",
  "progress.resetting": "Запуск приложения и UART-монитора",
  "progress.monitoring": "UART-монитор открыт",
  "progress.passed": "Прошивка записана и проверена",
  "progress.failed": "Операция завершилась ошибкой",
  "progress.disconnected": "Устройство отключено",
  "progress.segment": "Сегмент {current}/{total}",
  "progress.write_address": "Запись {bytes} байт по адресу {address}",
  "progress.segment_verified": "Проверка записанного сегмента",
  "progress.segment_written": "Сегмент {current} записан",
  "unit.bytes": "{value} Б",
  "unit.kib": "{value} КиБ",
  "unit.mib": "{value} МиБ",
  "error.unknown": "Неизвестная ошибка",
  "error.PACKAGE_INVALID": "Выбранная прошивка некорректна",
  "error.PACKAGE_UNSUPPORTED": "Формат прошивки или ESP32-чип не поддерживается",
  "error.PACKAGE_PATH_INVALID": "Путь к прошивке недоступен или некорректен",
  "error.PACKAGE_FILE_MISSING": "Не найден обязательный файл прошивки",
  "error.HASH_MISMATCH": "Файл прошивки изменился после проверки",
  "error.PORT_BUSY": "COM-порт занят или не открывается",
  "error.DEVICE_NOT_FOUND": "ESP32 или COM-порт не найдены",
  "error.CHIP_MISMATCH": "Прошивка предназначена для другого ESP32-чипа",
  "error.FLASH_CONNECT_FAILED": "Не удалось подключиться к загрузчику ESP32",
  "error.FLASH_ERASE_FAILED": "Не удалось стереть flash ESP32",
  "error.FLASH_WRITE_FAILED": "Не удалось записать прошивку во flash",
  "error.FLASH_VERIFY_FAILED": "Проверка записанных данных завершилась ошибкой",
  "error.PARTITION_INVALID": "Таблица разделов ESP32 некорректна или не найдена",
  "error.OTA_STATE_INVALID": "Состояние OTA-разделов некорректно",
  "error.BOOT_MARKER_TIMEOUT": "UART-маркер не получен до истечения таймаута",
  "error.DEVICE_DISCONNECTED": "ESP32 отключена",
  "error.DATA_DIRECTORY_UNWRITABLE": "Рабочая папка недоступна для записи",
  "error.OPERATION_IN_PROGRESS": "Другая операция уже выполняется",
  "error.INVALID_STATE": "Действие недоступно в текущем состоянии",
  "error.IN_PLACE_CONFIRMATION_REQUIRED": "Обновление требует подтверждения: свободного OTA-слота нет",
  "error.IO_ERROR": "Ошибка чтения или записи файла либо устройства",
  "error.INTERNAL_ERROR": "Внутренняя ошибка приложения",
};

export type Translator = (
  key: TranslationKey,
  params?: TranslationParams,
) => string;

export function languageFromLocale(locale: string | undefined): UiLanguage {
  return locale?.trim().toLowerCase().startsWith("ru") ? "ru" : "en";
}

export function detectUiLanguage(
  locales: readonly string[] = globalThis.navigator?.languages ?? [],
): UiLanguage {
  const first = locales[0] ?? globalThis.navigator?.language;
  return languageFromLocale(first);
}

export function createTranslator(language: UiLanguage): Translator {
  const catalog = language === "ru" ? RU : EN;
  return (key, params = {}) =>
    Object.entries(params).reduce(
      (value, [name, replacement]) =>
        value.replaceAll(`{${name}}`, String(replacement)),
      catalog[key],
    );
}

export function stageText(
  stage: OperationStage,
  language: UiLanguage,
): string {
  return createTranslator(language)(`stage.${stage}` as TranslationKey);
}

export function localizeBackendError(
  error: BackendError,
  language: UiLanguage,
): BackendError {
  const t = createTranslator(language);
  const key = `error.${error.code}` as TranslationKey;
  const message = key in EN ? t(key) : t("error.unknown");
  return {
    ...error,
    message,
    detail: localizeDiagnosticDetail(error.detail, language),
  };
}

export function localizeOperationText(
  value: string,
  stage: OperationStage,
  language: UiLanguage,
): string {
  if (language === "ru" || !containsCyrillic(value)) return value;
  const t = createTranslator(language);
  const segment = /^Сегмент (\d+)\/(\d+)$/.exec(value);
  if (segment) {
    return t("progress.segment", { current: segment[1], total: segment[2] });
  }
  const write = /^Запись (\d+) байт по адресу (0x[0-9A-F]+)$/i.exec(value);
  if (write) {
    return t("progress.write_address", {
      bytes: write[1],
      address: write[2],
    });
  }
  const written = /^Сегмент (\d+) записан$/.exec(value);
  if (written) return t("progress.segment_written", { current: written[1] });
  if (value === "Проверка записанного сегмента") {
    return t("progress.segment_verified");
  }
  return t(`progress.${stage}` as TranslationKey);
}

export function localizeDiagnosticDetail(
  detail: string | undefined,
  language: UiLanguage,
): string | undefined {
  if (!detail || /json/i.test(detail)) return undefined;
  if (language === "ru" || !containsCyrillic(detail)) return detail;
  return undefined;
}

function containsCyrillic(value: string): boolean {
  return /[А-Яа-яЁё]/.test(value);
}
