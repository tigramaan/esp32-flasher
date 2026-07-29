# Task / Contract Matrix

| Контракт | Реализация | Задачи | Проверка |
|---|---|---|---|
| Firmware input inspection | `platform/package.rs`, `programmer-core/ota.rs` | T051–T054, T062–T064 | direct file/folder tests |
| Firmware package v1 (legacy) | `programmer-core/manifest.rs`, `package.rs` | T008–T011, T053 | `manifest_validation`, legacy regression |
| ESP image chip header | `programmer-core/image.rs` | T010–T011 | image unit tests |
| OTA selection/otadata/device table | `programmer-core/ota.rs`, `platform/esp.rs` | T020–T021, T055–T056 | `tests/ota.rs`, scan tests |
| Operation state/errors | `operation.rs`, `error.rs` | T008, T012–T014 | state + contract tests |
| Optional UART marker and line fidelity | `marker.rs`, `platform/serial.rs`, `src/state.ts` | T022–T023, T026, T057, T065 | marker/stream/layout tests, HIL |
| Tauri IPC v1 | `src-tauri/src/ipc`, `src/api.ts` | T029–T030, T037 | TypeScript build, Rust check |
| CLI v1 | `tools/firmware-packager` | T015–T019 | `tests/packager.rs` |
| Factory CSV | `platform/reports.rs` | T032–T033 | report unit test, HIL |
| Portable storage | `platform/storage.rs` | T039 | replacement unit test |
| Resource bounds | logging/terminal/core constants | T040, T042, T046 | Rust + Vitest |
| Portable executable | Cargo/Tauri/CI config | T044, T050 | release build smoke |
| Direct-folder summary/map | `PackageSummary`, IPC, TypeScript UI | T058–T059 | Rust contract + Vitest/layout |
| Direct update BIN selection | `platform/package.rs`, dialog/UI | T062–T064 | Rust path guards + Vitest copy/layout |
| Safe COM discovery/reset | `src/main.ts`, `src/state.ts`, `platform/esp.rs`, `platform/serial.rs` | T067–T069 | enumeration/reset-policy regressions + HIL-01 |
| Standalone UART lifecycle | `platform/serial.rs`, `application/coordinator.rs`, IPC, TypeScript UI | T071–T074 | state/layout/storage/baud tests + HIL-06 |
| Windows locale + bilingual UI | `ipc::system_locale`, `src/i18n.ts`, state/view | T076–T079 | locale/error/English DOM tests |
| Public product identity and portable release | Cargo/Tauri/npm/workflows/README | T080–T084 | source audit, release build and download smoke |
| GitHub Pages/SEO/GEO | `site/`, Pages workflow | T081–T084 | site validator + browser viewport matrix |
