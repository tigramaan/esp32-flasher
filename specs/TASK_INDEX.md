# Task Index

Детальная декомпозиция: [001-esp32-programmer/tasks.md](001-esp32-programmer/tasks.md).

| Диапазон | Область | Основные артефакты |
|---|---|---|
| T001–T005 | setup/release | README, Cargo workspace, Tauri, CI |
| T006–T014 | contracts/core | fixtures, errors, manifest, package, state |
| T015–T019 | US-003 package tool | `tools/firmware-packager` |
| T020–T031 | US-001 update | OTA, espflash, serial, IPC, update UI |
| T032–T038 | US-002 factory | CSV, workflow, erase, counters |
| T039–T050 | cross-cutting | storage, logs, tests, security, docs, build |
| T051–T061 | direct folders | partition parsing, PlatformIO/standalone, map, verification |
| T062–T066 | update file/UART fidelity | direct BIN picker, chunk-safe lines, no-wrap monitor, release |
| T067–T070 | safe COM lifecycle | non-invasive selection, normal-boot recovery, regressions, release |
| T071–T075 | standalone UART | passive open, reconnect/reset/baud, flash lock, verification |
| T076–T084 | localization/public release | RU/EN locale, ESP32 Flasher identity, README, Pages, SEO/GEO, GitHub Release |

Статус каждой задачи хранится только в feature `tasks.md`. HIL runbook может быть
готов, даже если аппаратный прогон ещё не выполнен.
