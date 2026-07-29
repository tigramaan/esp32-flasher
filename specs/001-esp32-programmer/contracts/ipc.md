# Tauri IPC Contract v1

Все DTO используют `snake_case`. Неизвестные поля request/settings отклоняются.
Командная ошибка:

```json
{
  "code": "PACKAGE_INVALID",
  "message": "Пакет прошивки не прошёл проверку",
  "detail": "segments[0].sha256 does not match file",
  "retryable": false
}
```

## Commands

| Command | Input | Output |
|---|---|---|
| `system_locale` | — | Windows user default locale name; fallback `en` |
| `list_devices` | — | `PortCandidate[]` без reset/probe |
| `start_serial_monitor` | `{port, baud, on_event: Channel}` | `null`; passive open без reset |
| `validate_package` | `{path}` | `PackageSummary` с source/layout/segments |
| `data_directory` | — | `{path?, writable}` |
| `set_data_directory` | `{path}` | `{path, writable:true}` |
| `get_settings` | — | `PortableSettingsV1` |
| `update_settings` | `{settings}` | сохранённые settings |
| `start_factory_session` | `{package_path}` | `FactorySessionSummary` |
| `factory_session` | — | `FactorySessionSummary?` |
| `start_update` | `{request, on_event: Channel}` | final `OperationResult` |
| `start_factory_flash` | `{request, on_event: Channel}` | final `OperationResult` |
| `send_serial` | `{data}` | `null`; только при активном monitor |
| `reset_monitor` | — | `null`; normal boot reset через открытый monitor |
| `disconnect_monitor` | — | `null`; ждёт освобождения COM до 2 секунд |

`UpdateRequest`:

```json
{
  "package_path": "D:\\firmware\\nova-update",
  "port": "COM5",
  "confirm_in_place": false
}
```

При factory-only layout без подтверждения возвращается
`IN_PLACE_CONFIRMATION_REQUIRED`; запись не начинается.

`FactoryFlashRequest`:

```json
{
  "package_path": "D:\\firmware\\nova-factory",
  "port": "COM5",
  "full_erase": false,
  "success_marker": "NOVA_READY"
}
```

`success_marker` содержит 0..256 UTF-8 bytes. Для direct PlatformIO-папки
пустая строка отключает подтверждение запуска; непустая требует совпадения до
таймаута. Legacy manifest продолжает задавать собственную monitor policy.

Обе flash-команды отклоняют concurrent operation. Команда выполняется вне UI
thread и возвращает result после flash verify и, если marker настроен, после
marker/timeout.

`list_devices` никогда не открывает COM-порт. Chip/MAC/flash size определяются
flash-командой после явного действия пользователя. Ошибка подключения должна
освободить DTR/RTS и выполнить best-effort reset в normal boot.

`start_serial_monitor` принимает только `9600`, `57600`, `115200`, `230400`,
`460800` или `921600`. Direct monitor освобождает DTR/RTS без reset и передаёт
`MonitorEvent`: `data {data}` или `disconnected {port,message}`. Повторные
подключения не выполняются скрыто. Flash-команда останавливает существующий
monitor внутри backend и не начинает ROM handshake, пока COM не освобождён.

`PortableSettingsV1`:

```json
{
  "schema_version": 1,
  "mode": "factory",
  "last_update_package": "D:\\firmware\\update",
  "last_factory_package": "D:\\firmware\\factory",
  "factory_success_marker": "NOVA_READY",
  "monitor_baud": 115200
}
```

`factory_success_marker` обязателен в ответе и сохраняется атомарно; при чтении
старого файла отсутствующее поле трактуется как пустая строка.
Отсутствующий `monitor_baud` трактуется как `115200`.

## Operation Channel

Channel передаёт tagged union с полем `type`:

- `state`: `operation_id`, `state`, `message`, optional `error`;
- `progress`: `operation_id`, вложенный `progress` с current/total/percentage,
  segment index/count и message;
- `log`: `operation_id`, timestamped `entry`;
- `serial`: `operation_id`, `{text, base64}`;
- `monitor_disconnected`: `operation_id`, `port`, `message`.

Raw bytes кодируются base64; lossy UTF-8 используется только для показа. Marker
detector получает raw bytes. UI объединяет render через animation frame и хранит
не более 10 000 terminal lines.

## Final Result

```json
{
  "operation_id": "uuid",
  "success": true,
  "boot_confirmed": true,
  "duration_ms": 4812,
  "device": {},
  "package": {},
  "error": null,
  "report_path": null
}
```

`success=true` допускается после verify. При настроенном marker дополнительно
требуется marker match. Без marker монитор открывается, `success=true`, а
`boot_confirmed=false`. Factory result содержит CSV report path.

## Firmware input inspection

`validate_package` сохраняет имя IPC для обратной совместимости, но принимает:

- `platformio`: standard factory trio/quartet без дополнительного manifest;
- `standalone`: прямой путь к одному application `.bin` с любым именем; для
  обратной совместимости также принимается папка ровно с одним BIN;
- `legacy_manifest`: `firmware.json` v1.

`PackageSummary.segments[].offset` присутствует для вычисленной factory-карты и
отсутствует для standalone update. Поля `source`,
`requires_device_layout` и `success_marker_configured` обязательны.
