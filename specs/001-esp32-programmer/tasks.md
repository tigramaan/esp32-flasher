# Tasks: ESP32 Flasher

## Phase 1 — Setup

- [x] T001 Preserve upstream revision and product provenance in README.md and LICENSE
- [x] T002 Rename package, executable and Tauri identity in package.json and src-tauri/tauri.conf.json
- [x] T003 Create Rust workspace members in Cargo.toml, crates/programmer-core/Cargo.toml and tools/firmware-packager/Cargo.toml
- [x] T004 Create required project documentation structure in specs/ and docs/
- [x] T005 Add Windows portable validation workflow in .github/workflows/build.yml

## Phase 2 — Foundational

- [x] T006 [P] Add manifest fixtures in tests/fixtures/packages/
- [x] T007 [P] Add ESP-IDF build fixtures in tests/fixtures/idf-build/
- [x] T008 Implement typed error codes in crates/programmer-core/src/error.rs
- [x] T009 Implement manifest types and strict deserialization in crates/programmer-core/src/manifest.rs
- [x] T010 Write manifest validation tests in crates/programmer-core/tests/manifest_validation.rs
- [x] T011 Implement path/hash/range package validation in crates/programmer-core/src/package.rs
- [x] T012 Write operation state tests in crates/programmer-core/tests/operation_state.rs
- [x] T013 Implement operation state machine in crates/programmer-core/src/operation.rs
- [x] T014 Implement shared DTO exports in crates/programmer-core/src/lib.rs

## Phase 3 — US-003 Developer prepares packages

**Goal**: reproducibly generate and validate simple factory/update folders.

**Independent test**: generate both package kinds from fixtures, validate them, corrupt one file and verify rejection.

- [x] T015 [P] [US3] Write ESP-IDF parser tests in tools/firmware-packager/tests/packager.rs
- [x] T016 [US3] Implement flasher_args parser in tools/firmware-packager/src/idf.rs
- [x] T017 [US3] Implement atomic package builder in tools/firmware-packager/src/builder.rs
- [x] T018 [US3] Implement factory/update/validate CLI in tools/firmware-packager/src/main.rs
- [x] T019 [US3] Document CLI, exits and dry-run in tools/firmware-packager/README.md

## Phase 4 — US-001 Client updates a device

**Goal**: update one application image on factory-only or safe OTA layout and open the monitor.

**Independent test**: run use case with fake adapters for both layouts, then execute HIL factory-only and OTA scenarios.

- [x] T020 [P] [US1] Write partition/otadata tests in crates/programmer-core/tests/ota.rs
- [x] T021 [US1] Implement partition and OTA selection in crates/programmer-core/src/ota.rs
- [x] T022 [P] [US1] Write marker detector tests in crates/programmer-core/tests/marker.rs
- [x] T023 [US1] Implement bounded UART marker detector in crates/programmer-core/src/marker.rs
- [x] T024 [US1] Define narrow flash/serial/storage module contracts in src-tauri/src/platform/
- [x] T025 [US1] Implement espflash adapter in src-tauri/src/platform/esp.rs
- [x] T026 [US1] Implement serial discovery and monitor adapter in src-tauri/src/platform/serial.rs
- [x] T027 [US1] Write update planning regressions in crates/programmer-core/tests/ota.rs
- [x] T028 [US1] Implement update workflow in src-tauri/src/application/update.rs
- [x] T029 [US1] Implement typed Tauri commands/events in src-tauri/src/ipc/
- [x] T030 [P] [US1] Implement TypeScript IPC client and types in src/api.ts and src/types.ts
- [x] T031 [US1] Implement client update screen in src/main.ts and src/styles.css

## Phase 5 — US-002 Operator flashes production boards

**Goal**: manually run repeatable multi-segment cycles with boot marker, counters and one CSV row per board.

**Independent test**: execute ten fake cycles and HIL cycles, including timeout and disconnect; compare counters with CSV.

- [x] T032 [P] [US2] Write CSV/session tests in src-tauri/src/platform/reports.rs
- [x] T033 [US2] Implement portable CSV factory sessions in src-tauri/src/platform/reports.rs
- [x] T034 [US2] Cover factory manifest/ranges/session outcomes with core and report tests
- [x] T035 [US2] Implement multi-segment factory workflow in src-tauri/src/application/factory.rs
- [x] T036 [US2] Add guarded full-chip erase in src-tauri/src/application/factory.rs
- [x] T037 [US2] Implement factory session/counter IPC in src-tauri/src/ipc/
- [x] T038 [US2] Implement factory screen, counters and erase confirmation in src/main.ts and src/styles.css

## Phase 6 — Polish and cross-cutting concerns

- [x] T039 Implement portable settings and atomic filesystem writes in src-tauri/src/platform/storage.rs
- [x] T040 Implement bounded operation log rotation in src-tauri/src/platform/logging.rs
- [x] T041 Implement device polling and one-operation coordinator in src-tauri/src/application/coordinator.rs
- [x] T042 Add frontend state and rendering tests in src/state.test.ts and src/layout.test.ts
- [x] T043 Add Rust contract/regression tests for all stable error codes in crates/programmer-core/tests/contracts.rs
- [x] T044 Configure raw portable build and optional Authenticode CI in .github/workflows/build.yml
- [x] T045 Add security/regression tests for no baud fallback, no network access and package traversal in crates/programmer-core/tests/security.rs
- [x] T046 Add sustained UART buffer responsiveness test in src/state.test.ts
- [x] T047 Update architecture, catalogs, integration guide and ADR in docs/
- [x] T048 Complete traceability and contract matrices in specs/
- [x] T049 Complete verification and HIL runbooks in specs/VERIFICATION_RUNBOOK.md and tests/hil/README.md
- [x] T050 Run npm test/build, cargo fmt/test/clippy, Tauri build and address all failures

## Phase 7 — Direct PlatformIO folders

- [x] T051 Update direct-folder requirements, contracts, ADR and traceability in specs/ and docs/adr/
- [x] T052 [P] [US2] Add partition-table checksum/selection and chip boot-address tests in crates/programmer-core/tests/
- [x] T053 [US2] Implement PlatformIO trio inspection without JSON in src-tauri/src/platform/package.rs
- [x] T054 [US2] Add incomplete/ambiguous folder, symlink, size and chip guards in src-tauri/src/platform/package.rs
- [x] T055 [US1] Implement bounded device partition-table discovery in src-tauri/src/platform/esp.rs
- [x] T056 [US1] Select inactive OTA slot including factory-to-single-OTA transition in crates/programmer-core/src/ota.rs
- [x] T057 [US1] Make UART marker optional while always opening monitor in src-tauri/src/platform/serial.rs and application/
- [x] T058 [US3] Extend IPC summary with source and computed map in crates/programmer-core/src/package.rs and src/types.ts
- [x] T059 [US3] Render direct-folder source/map without layout regressions in src/view.ts and src/styles/
- [x] T060 [P] Add backend/frontend direct-folder regressions in src-tauri/src/platform/package.rs and src/*.test.ts
- [x] T061 Run automated verification, responsive layout contract and portable release build per specs/VERIFICATION_RUNBOOK.md

## Phase 8 — Update file picker and UART line fidelity

- [x] T062 Update update-input and UART line contracts in specs/ and docs/
- [x] T063 [US1] Accept a direct arbitrary-name BIN path with file/symlink/extension guards in src-tauri/src/platform/package.rs
- [x] T064 [US1] Use a filtered single-file picker and mode-specific copy in src/main.ts and src/view.ts
- [x] T065 [US1] Preserve UART line boundaries across read chunks, bound the stream and disable UART-only soft wrapping
- [x] T066 Add Rust/Vitest regressions, run layout QA and rebuild the portable release

## Phase 9 — Safe COM discovery and reset recovery

- [x] T067 [US1] Replace startup/manual active probe with non-invasive COM enumeration and selection
- [x] T068 [US1] Use hard reset after completed ESP sessions and best-effort normal-boot recovery after failed connect
- [x] T069 Add TypeScript/Rust regressions and update IPC, traceability and HIL contracts
- [x] T070 Run automated verification and rebuild the portable release

## Phase 10 — Standalone UART monitor

- [x] T071 [US4] Define direct-monitor lifecycle, IPC, safety and UI contracts
- [x] T072 [US4] Implement passive monitor, bounded stop-and-wait and in-port normal reset
- [x] T073 [US4] Implement auto-connect tab, explicit reconnect, baud persistence and flash lock UI
- [x] T074 Add state/layout/storage/baud regressions and HIL-06 procedure
- [ ] T075 Run full verification and rebuild the portable release

## Phase 11 — Localization, public identity and release

**Goal**: publish ESP32 Flasher as a bilingual portable Windows utility with an
English open-source landing page and downloadable release.

**Independent test**: run locale tests for `ru-RU`, `en-US`, and `de-DE`; verify
both app languages, site smoke matrix, release EXE/checksum, README links, Pages
metadata, and public repository settings.

- [x] T076 [US5] Define locale, branding, public documentation, Pages and release requirements in specs/ and docs/
- [x] T077 [P] [US5] Implement native Windows locale IPC and RU/EN catalogs in src-tauri/src/ipc/mod.rs and src/i18n.ts
- [x] T078 [US5] Localize UI, dialogs, operation stages and stable backend errors in src/main.ts, src/state.ts and src/view.ts
- [x] T079 [P] [US5] Add locale, fallback, English-leak and bilingual layout regressions in src/*.test.ts
- [x] T080 [US5] Rename public product identity and portable binary metadata in package.json, Cargo.toml and src-tauri/
- [ ] T081 [P] [US5] Write fully English README, release notes and repository metadata
- [ ] T082 [P] [US5] Build responsive English GitHub Pages with SEO/GEO artifacts and validation
- [ ] T083 [US5] Run automated verification, layout QA and portable release build
- [ ] T084 [US5] Publish source, Pages configuration and tagged GitHub Release to tigramaan/esp32-flasher

## Dependencies

```text
Setup -> Foundational -> US3 package tool
                    \-> US1 update workflow -> US2 factory workflow
All stories -> Polish -> Verification
```

US3 and UI fixture preparation may progress independently after the shared manifest contract. US2 reuses detection, monitor and operation coordination established by US1.

## Delivery Strategy

1. Core manifest and tests.
2. Package generator as the first independently usable artifact.
3. Client update vertical slice.
4. Factory workflow and reporting.
5. Portable release hardening and full verification.
