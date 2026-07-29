# Execution Plan

Полный архитектурный план: [001-esp32-programmer/plan.md](001-esp32-programmer/plan.md).

## Этапы и состояние

| Этап | Результат | Состояние |
|---|---|---|
| Spec | REQ-001—REQ-027, US-001—US-005 | готово |
| Plan | архитектура, риски, storage/security/observability | готово |
| Contracts | direct folders, legacy schema, IPC v1, CLI v1 | готово |
| Tasks | T001—T084 | выполняется |
| Core | folder/image/partition guards, OTA, marker, state, errors | реализовано |
| Package tool | factory/update/validate/dry-run | реализовано |
| Windows app | direct update/factory, monitor, report, portable storage | реализовано |
| UI | два режима, computed map, progress, terminal, counters | реализовано |
| Localization | Windows locale, RU/EN UI/errors/stages | реализовано |
| Public release | English README, Pages, SEO/GEO, GitHub Release | выполняется |
| Verification | unit, integration, clippy, npm, portable build | автоматические проверки готовы; manual visual/HIL ожидаются |
| HIL | реальные factory-only/OTA/production платы | требуется стенд |

## Инструменты и plugins

- `speckit-workflow` — Spec/Plan/Contracts/Tasks/Analyze/Implementation;
- `frontend-layout-qa` — responsive criteria and visual QA;
- Cargo/rustfmt/clippy/test — Rust checks;
- npm/TypeScript/Vite/Vitest — frontend checks;
- Tauri CLI — raw portable EXE;
- `programmer-pack` — optional legacy package preparation;
- `tools/test-fixtures/generate.ps1` — direct/legacy binary fixtures;
- Git/GitHub Actions — provenance and release verification.
- Windows user-locale API — deterministic language selection;
- GitHub Pages actions — static site publication;
- static site validator — metadata/link/structured-data checks.

Новые внешние plugins не требуются. Единственный созданный project tool —
`tools/firmware-packager`; для Pages добавляется `tools/site/verify.mjs`.

## Ограничения выполнения

HIL нельзя эмулировать как пройденный без физической платы. Authenticode требует
сертификат владельца и выполняется только при наличии CI secrets.
