# Catalog: Modules

| Модуль | Тип | Зависимости | Состояние |
|---|---|---|---|
| `core::manifest` | pure | serde | stable v1 |
| `core::package` | pure over `PackageReader` | sha2, manifest/image | stable v1 |
| `core::ota` | pure | md5/manifest/errors | stable v1 |
| `core::marker` | pure bounded stream | none | stable v1 |
| `application::coordinator` | application | storage/report/monitor | internal |
| `application::update` | application | core + platform contracts | internal |
| `application::factory` | application | core + platform contracts | internal |
| `platform::esp` | I/O | espflash | internal replaceable |
| `platform::serial` | I/O | serialport | passive/reset modes, bounded ownership handoff |
| `platform::package` | I/O | filesystem + core | direct PlatformIO/standalone/legacy |
| `platform::storage/logging/reports` | I/O | filesystem/csv | internal |
| `ipc` | public desktop | Tauri/application | contract v1 |
| `src/state` | pure UI | DTO | internal/tested |
| `src/view` | UI | DOM/state | internal |
| `src/i18n` | pure UI boundary | locale string + stable error/stage contracts | RU/EN, English fallback |
| `site` | static public view | release/source links only | English Pages source |
