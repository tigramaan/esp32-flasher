# Catalog: Components

| Компонент | Назначение | Public interface | Ошибки | REQ / тесты |
|---|---|---|---|---|
| ESP32 Flasher UI | RU/EN update/factory UX | user actions + rendered state | локализует `OperationError` по коду | REQ-002/008/011/013/023–025; Vitest |
| Tauri IPC | desktop boundary | commands, DTO, Channel | serialized stable codes | REQ-003/008/015; build/contracts |
| programmer-core | image/partition/OTA rules | Rust exports | fail-fast guards | REQ-004/007/009/010/017–019; core tests |
| Firmware folder adapter | direct/legacy inspection | internal `LoadedPackage` | incomplete/ambiguous/path errors | REQ-004/017/020; unit tests |
| ESP adapter | explicit-operation ROM bootloader | connect/read/write/erase/reset/discover table | connect/write/verify/partition codes; normal-boot recovery | REQ-003/005/006/018; reset-policy test/HIL |
| UART adapter | standalone/flash monitor, marker, normal reset | passive/reset start, send/reset/stop-and-wait | busy/disconnect/timeout | REQ-008/009/022; marker/state/HIL |
| Portable store | settings/logs/reports | local filesystem | data/io codes | REQ-011/012; storage/report tests |
| programmer-pack | optional legacy package generation | CLI v1 | deterministic exit != 0 | REQ-014; integration tests |
| Locale boundary | Windows locale + catalogs | `system_locale`, `UiLanguage`, `Translator` | English fallback | REQ-023/024; i18n tests |
| Public Pages | download/features/FAQ | static HTML + latest release link | no runtime state | REQ-026/027; site/layout checks |
