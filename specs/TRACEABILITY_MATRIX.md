# Requirements Traceability Matrix

| REQ | Наблюдаемое поведение / контракт | Задачи | Тест или процедура |
|---|---|---|---|
| REQ-001 | raw x64 `ESP32 Flasher.exe`, static CRT, embedded backend | T002–T005, T044, T050, T080 | portable build; clean-PC smoke |
| REQ-002 | update default, explicit factory tab | T030–T031, T038 | `state.test.ts`; UI smoke |
| REQ-003 | non-invasive enumerate/rank/select; active detection only inside explicit operation; normal reset on failure | T025–T026, T041, T067–T069 | `state.test.ts`; esp reset-policy test; HIL-01 |
| REQ-004 | direct PlatformIO folder или выбранный standalone BIN; manifest optional | T051, T053–T054, T062–T064 | file/folder backend tests; dialog UI test |
| REQ-005 | multi-segment, offsets, 921600, manual cycle | T035–T038 | security test; HIL-04 |
| REQ-006 | one app BIN, factory-only and OTA | T020–T031 | OTA tests; HIL-02/HIL-03 |
| REQ-007 | inactive slot then otadata; guarded in-place fallback | T020–T021, T028, T056 | OTA CRC/selection tests; HIL-03 |
| REQ-008 | states, progress, logs, auto UART; chunk-safe lines without wrapping | T026, T029–T031, T065 | stream reducer/layout Vitest; HIL |
| REQ-009 | optional bounded marker; production setting; monitor always opens | T022–T023, T057 | marker/factory/storage tests; timeout HIL |
| REQ-010 | image/chip/partition/range/capacity guards | T008–T011, T021, T052–T054 | image, direct-folder, OTA, security tests |
| REQ-011 | session counters + one flushed CSV row | T032–T038 | report test; HIL-04 reconciliation |
| REQ-012 | data beside EXE or explicit folder | T039–T040 | storage tests; read-only-folder smoke |
| REQ-013 | factory-only confirmed full erase | T036, T038 | UI smoke; HIL destructive case |
| REQ-014 | optional `programmer-pack` factory/update/validate | T015–T019 | CLI integration tests |
| REQ-015 | stable codes, log, fail closed | T008, T027, T034, T040, T043 | contract/workflow tests; logs |
| REQ-016 | no fallback/network/telemetry | T045 | `security.rs`; source/config audit |
| REQ-017 | parse bundled partitions and chip-specific boot address | T052–T054 | direct-folder plan tests |
| REQ-018 | discover partition table and otadata on device | T055 | scan tests; HIL-02/HIL-03 |
| REQ-019 | choose actual inactive OTA slot | T056 | OTA selection tests |
| REQ-020 | manifest remains optional legacy input | T051, T053 | legacy regression test |
| REQ-021 | source/chip/segment flash map visible before factory write | T058–T059 | TypeScript render/layout test |
| REQ-022 | direct passive UART, explicit reconnect/reset/baud, flash ownership lock | T071–T075 | monitor state/layout/storage/baud tests; HIL-06 |
| REQ-023 | Windows user locale; `ru-*` Russian, every other/failure case English | T076–T079 | `i18n.test.ts`; native IPC build; locale smoke |
| REQ-024 | bilingual labels/dialogs/stages/errors without Russian leakage in English mode | T077–T079 | English DOM and backend-error localization tests |
| REQ-025 | ESP32 Flasher identity across app, EXE, metadata and public pages | T080–T082 | config/source audit; release smoke |
| REQ-026 | English public repository, MIT, Windows workflow, portable EXE + SHA-256 | T081, T083–T084 | repository/release inspection; workflow |
| REQ-027 | responsive Pages with factual SEO/GEO metadata and latest-release link | T082–T084 | site validator; viewport matrix; deployed Pages smoke |

После выполнения T076–T084 все требования имеют реализацию и процедуру. Аппаратные строки требуют отдельного
подписанного протокола выполнения на реальном стенде.
