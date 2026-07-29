# Implementation Plan: ESP32 Flasher

**Branch**: `001-esp32-programmer` | **Date**: 2026-07-29 | **Spec**: [spec.md](spec.md)

## Summary

Переработать `soofdev/serial-monitor` в один portable Windows EXE с двумя сценариями: безопасное клиентское обновление выбранным application BIN и быстрая первичная прошивка стандартной папки PlatformIO. Общая Rust domain-библиотека отвечает за разбор ESP image/partition table, guards, OTA и результаты; Tauri-слой распознаёт файл или папку и изолирует serial/flash/filesystem I/O; TypeScript UI отображает вычисленную карту. Legacy manifest и отдельный CLI остаются необязательными.

## Technical Context

- **Language/Version**: Rust stable edition 2021; TypeScript 5.9.
- **Primary Dependencies**: Tauri 2, `espflash` 4, `serialport` 4, Tokio, Serde, Vite 6, Vitest.
- **Storage**: JSON settings, UTF-8 operation logs, UTF-8 BOM CSV reports рядом с EXE.
- **Testing**: `cargo test`, `cargo clippy`, Vitest, Tauri build smoke, ручной HIL runbook.
- **Target Platform**: Windows 10/11 x64, системный WebView2.
- **Distribution**: один `ESP32 Flasher.exe`, без installer и Python; GitHub
  Release с SHA-256 и англоязычный GitHub Pages.
- **Performance**: flash baud 921600 без fallback; UI получает ограниченный поток прогресс-событий; не более 10 000 UART-строк в DOM.
- **Constraints**: одна плата и одна операция одновременно; package input считается недоверенным.

## Architecture Gates

- Domain не зависит от Tauri, UI, serialport или файловой системы: **PASS**.
- Все I/O вызываются через интерфейсы application/platform: **PASS**.
- Guards выполняются до flash mutation: **PASS**.
- Публичные manifest/IPC/CLI контракты версионированы и тестируются: **PASS**.
- Ошибки классифицированы, retry ограничен, silent failures запрещены: **PASS**.
- Логи, security boundaries и resource limits определены до реализации: **PASS**.
- Каждое REQ связано с задачей и проверкой: **PASS**.

## Design

### Domain

`programmer-core` содержит:

- `FirmwareManifestV1` для legacy, `FirmwareSegment`, `PackageKind`, `ChipFamily`, `FirmwareSource`;
- строгую проверку структуры, пути, размера, SHA-256 и диапазонов;
- разбор и проверку binary partition table, выбор factory/OTA-раздела и chip-specific bootloader offset;
- `FlashPlan` для factory и update;
- ESP-IDF partition table/OTA модели и расчёт следующей `otadata`;
- конечную машину `OperationState`;
- `OperationError` со стабильным `ErrorCode`;
- детектор UART-маркера, устойчивый к границам чанков;
- модели factory session/report.

### Application

Use cases:

- inspect selected firmware file or folder;
- detect/list device;
- factory flash;
- application update;
- start/stop UART monitor;
- start factory session;
- persist settings and append result.

Все use cases получают platform interfaces и не обращаются напрямую к UI. Operation lock отклоняет конкурентный запуск.

### Platform

- `EspflashAdapter`: ROM handshake, chip/MAC, flash size, erase, read/write/verify/reset.
- `SerialAdapter`: enumerate/open/read/write/disconnect.
- `FirmwareFolderStore`: canonical paths, bounded reads, распознавание PlatformIO/standalone/legacy.
- `PortableDataStore`: atomic settings, log rotation and CSV flush.
- `TauriEventSink`: typed progress/log/serial events.

### UI

Один responsive shell:

- header с режимом и status pill;
- device card;
- package card;
- компактную карту обнаруженных BIN и адресов;
- primary action/progress;
- factory counters и advanced erase;
- process log и UART terminal;
- modal выбора при нескольких устройствах и подтверждение опасных действий.

Клиентский экран загружается первым. Factory mode запоминается только после явного переключения.

Locale определяется отдельной Windows IPC-командой через user default locale.
Каталоги RU/EN находятся на UI-границе. Backend сохраняет стабильные error codes;
английский UI переводит сообщения по коду/стадии и скрывает непереведённые
кириллические details, не меняя диагностические локальные логи.

### Publication

- public identity: `ESP32 Flasher`, repository `tigramaan/esp32-flasher`;
- GitHub Actions собирает portable x64 EXE, checksum и tagged release;
- статический `site/` публикуется через Pages workflow;
- canonical, Open Graph, SoftwareApplication/FAQ JSON-LD, sitemap, robots и
  `llms.txt` описывают только проверяемые возможности;
- README, Pages и release notes полностью английские; внутренняя инженерная
  документация остаётся источником истины на русском.

## Project Structure

```text
src/                            TypeScript UI
src-tauri/src/
  application/                  use cases and operation coordinator
  domain/                       app-facing types re-exported from core
  platform/                     espflash, serial, filesystem, reports
  ipc/                          Tauri commands and DTO mapping
crates/programmer-core/         pure contracts, validation, OTA, state
tools/firmware-packager/        programmer-pack CLI
specs/                          requirements, plan, tasks, traceability
docs/                           ADR, architecture and catalogs
tests/fixtures/                 valid and invalid package fixtures
tests/hil/                      hardware procedures
```

## Implementation Phases

1. Preserve upstream baseline and establish project identity.
2. Implement core types/guards with tests before platform integration.
3. Implement optional packager CLI using the shared legacy contract.
4. Add direct PlatformIO folder/standalone BIN inspection and device partition-table discovery.
5. Replace upstream flash functions with adapters and stateful workflows.
6. Replace multi-tab UI with the two-mode interface and detected flash map.
7. Add portable storage, reports, release pipeline and docs.
8. Add native locale detection, RU/EN UI boundary and product rebranding.
9. Add English public documentation, Pages, SEO/GEO metadata and release.
10. Run static, unit, integration, UI, site, build and HIL-ready verification.

## Risks and Mitigations

- **921600 is cable-sensitive**: no fallback by explicit decision; emit `FLASH_CONNECT_FAILED` or `FLASH_WRITE_FAILED`.
- **OTA metadata corruption**: parse and validate before writing; fail closed.
- **Single factory partition is not power-loss-safe**: show an explicit warning before in-place update.
- **Partition table address is not stored in its own BIN**: direct factory uses the PlatformIO/ESP-IDF standard `0x8000`; device update uses bounded aligned scanning and fails closed when no valid table is found.
- **Rollback requires firmware cooperation**: direct BIN defaults to rollback disabled; legacy manifest may opt in.
- **Opening COM toggles DTR/RTS**: polling and selection use enumeration metadata
  only; active detection starts inside an explicit operation and all error paths
  attempt a normal-boot reset.
- **Unsigned build triggers SmartScreen**: CI supports optional Authenticode; unsigned artifact is labelled.
- **No package signature**: direct-folder SHA-256 is diagnostic/TOCTOU protection only; document that it does not authenticate a release.
- **Locale API failure**: fail deterministically to English; never infer Russian
  from timezone, region, keyboard or firmware data.
- **Search metadata drift**: version-independent latest-release URLs and
  automated site checks keep README/Pages aligned with published artifacts.
- **Unsigned build triggers SmartScreen**: release notes state this limitation;
  optional Authenticode remains available through repository secrets.

## Post-design Gate

Контракты below eliminate all technical ambiguity. No architecture gate is violated. Implementation may proceed after tasks/analyze.
