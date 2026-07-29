# Data Model

## FirmwareSelection

- `source: platformio | standalone | legacy_manifest`
- `selected_path: canonical file or directory path`
- `root: canonical containing directory`
- `kind: factory | update`
- `target_chip: ChipFamily`
- `segments: FirmwareSegment[]`
- `partition_table_source: bundled | device`
- `monitor: MonitorPolicy`

Распознавание:

- `platformio`: обязательны `bootloader.bin`, `partitions.bin`, `firmware.bin`; `boot_app0.bin` необязателен;
- `standalone`: выбран один обычный `.bin`-файл с любым именем и валидным
  application image; папка с одним BIN поддерживается только для обратной
  совместимости IPC;
- `legacy_manifest`: присутствует валидный `firmware.json`.

Неизвестные дополнительные BIN в direct factory-папке считаются неоднозначностью и блокируют прошивку.

## FirmwareManifestV1 (legacy)

- `schema_version: 1`
- `package_id: string` — 1..64 ASCII letters, digits, `.`, `_`, `-`
- `display_name: string` — 1..120 UTF-8 characters
- `firmware_version: string` — 1..64 printable characters
- `kind: factory | update`
- `target_chips: ChipFamily[]` — non-empty unique list
- `partition_table_offset: HexAddress`
- `monitor: MonitorPolicy`
- `ota: OtaPolicy`
- `segments: FirmwareSegment[]`

Validation:

- no unknown fields;
- factory has at least one segment and every segment has an offset;
- update has exactly one application segment without an offset;
- segment ranges do not overflow `u32` or overlap;
- partition table role and declared offset agree for factory;
- all referenced paths are relative regular files within the package directory.

## FirmwareSegment

- `role: bootloader | partition_table | application | ota_data | data`
- `file: string`
- `offset: HexAddress?`
- `size: integer` — 1..64 MiB
- `sha256: string` — 64 lowercase hexadecimal characters

## MonitorPolicy

- `baud: integer` — one of 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600
- `success_marker: string` — 0..256 UTF-8 bytes; пустое значение отключает boot confirmation
- `success_timeout_ms: integer` — 1000..120000

Маркер сравнивается как UTF-8 byte sequence с перекрытием между чанками.

## OtaPolicy

- `rollback_enabled: boolean`

Поле относится только к legacy manifest. Direct BIN использует `false`, потому что настройку bootloader rollback нельзя надёжно вывести из application image.

## DetectedDevice

- `port: string`
- `description: string`
- `vid: u16?`
- `pid: u16?`
- `serial_number: string?`
- `status: candidate | selected | ready | busy | unsupported`
- `chip: ChipFamily?`
- `mac: string?`
- `flash_size_bytes: u64?`
- `error_code: ErrorCode?`

`candidate/selected` формируются только по метаданным Windows и не открывают
COM-порт. `ready`, chip, MAC и flash size появляются после явного запуска операции.

## FlashPlan

- `kind: factory | update_factory | update_ota`
- `chip`
- `segments: PlannedSegment[]`
- `full_erase`
- `ota_switch: OtaSwitch?`
- `partition_table_offset?`
- `monitor`

Создаётся только после package и device validation.

## FlashOperation

- `operation_id: UUID`
- `state: OperationState`
- `package`
- `device`
- `started_at`
- `finished_at?`
- `result?`

State transitions:

```text
idle -> validating -> detecting -> connecting
connecting -> erasing? -> writing -> verifying -> resetting -> monitoring
monitoring -> passed | failed
any active state -> failed
passed | failed -> disconnected
```

При пустом UART-маркере `passed` означает успешный flash verify и запуск монитора, а `boot_confirmed=false`. Переходы назад и параллельная active operation запрещены.

## FactorySession

- `session_id`
- `started_at`
- `package_id`
- `firmware_version`
- `total`
- `passed`
- `failed`
- `report_path`

Каждая завершённая операция добавляет ровно один `FactoryResult`.

## FactoryResult

- `timestamp`
- `session_id`
- `package_id`
- `firmware_version`
- `port`
- `mac`
- `chip`
- `duration_ms`
- `full_erase`
- `result: OK | ERROR`
- `error_code`

## PortableSettings

- `schema_version`
- `mode: update | factory`
- `last_update_package?`
- `last_factory_package?`
- `factory_success_marker: string` — 0..256 UTF-8 bytes, используется direct factory workflow
- `monitor_baud: 9600 | 57600 | 115200 | 230400 | 460800 | 921600`

## UiLocale

- `windows_locale: string` — значение Windows user default locale;
- `language: ru | en`;
- `source: windows | webview_fallback | english_fallback`.

Правило выбора: `ru-* -> ru`, любое другое, пустое или ошибочное значение
`-> en`. Locale определяется до первого render и не хранится в portable
settings. Каталоги содержат все пользовательские labels/dialogs/statuses;
backend errors локализуются по стабильному `ErrorCode`, operation progress — по
стабильной стадии.

## MonitorState

- `status: disconnected | connecting | connected | error | busy`
- `manually_disconnected: boolean`
- `resetting: boolean`
- `baud`
- `error?`

Ручное отключение запрещает автоподключение до явной команды пользователя.
`busy` означает, что COM-порт принадлежит flash operation.

Запись выполняется через temporary file + atomic rename.
