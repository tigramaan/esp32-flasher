# Архитектура ESP32 Flasher

```text
TypeScript view/state
        |
        v
Tauri IPC commands + typed Channel
        |
        v
application coordinator/update/factory
        |                    |
        v                    v
programmer-core          platform adapters
(pure domain)            (espflash/serial/fs/csv)
```

## Границы

- `programmer-core` — pure/domain: ESP image and partition models, legacy JSON,
  guards, hash/range validation, OTA calculations, operation states, marker detector.
- `application` — smart orchestration: single-operation lease, update/factory
  workflows, typed events and final result.
- `platform` — I/O: ROM bootloader, COM monitor, canonical filesystem, log/CSV.
- `ipc` — public desktop boundary: strict requests and serializable errors.
- `src` — UI-only state/rendering; flash decisions здесь отсутствуют.
- `src/i18n` — pure locale boundary: RU/EN catalogs, stable error/stage
  localization and English fallback.

COM polling uses only Windows enumeration metadata and never opens a port.
ROM bootloader detection and DTR/RTS switching belong exclusively to an explicit
update/factory operation. Every completed/error ESP session returns the board to
normal boot before the UART monitor opens; the monitor performs its own controlled
normal-boot reset after acquiring the port.

## Состояния операции

`idle -> validating -> detecting -> connecting -> [erasing] -> writing ->
verifying -> resetting -> monitoring -> passed|failed -> disconnected`.

Недопустимый переход отклоняется. UI получает те же стадии, что operation log.

## Потоки

Tauri async command переносит blocking flash workflow в `spawn_blocking`. UART
читается отдельным bounded thread. UI events проходят через Tauri `Channel`;
рендер объединяется через `requestAnimationFrame`, terminal хранит не более
10 000 строк.

Единственный `AppState` владеет `MonitorHandle`. Direct monitor и flash operation
сериализуются одним monitor mutex и operation lease. Flash acquisition сначала
блокирует новые monitor/reset/disconnect команды, затем отправляет shutdown и
ждёт завершения worker до двух секунд. Поэтому один COM-порт не используется
одновременно двумя потоками.

Windows locale читается до первого render через `system_locale`. Только `ru-*`
выбирает русский каталог; любой другой locale или ошибка IPC выбирает English.
Raw UART не переводится. Backend operation logs сохраняют исходную диагностику,
а UI не показывает непереведённые кириллические details в English mode.

## Storage

`data/` рядом с EXE:

```text
data/
  settings.json
  logs/<operation-id>.log
  reports/factory-YYYYMMDD-HHMMSS.csv
```

Если папка недоступна, запись запрещена до явного выбора другой data directory.

## Замена адаптеров

Core можно использовать без Tauri. Platform modules имеют узкие внутренние API:
connect/read/write/verify/reset, list/open/read/write/stop, inspect firmware folder и
append reports. При замене hardware backend public manifest/IPC не меняются.

Public `site/` изолирован от desktop runtime: это статическая англоязычная
страница без analytics и пользовательских данных. Download URL указывает на
GitHub Releases latest asset, публикация выполняется отдельным Pages workflow.
