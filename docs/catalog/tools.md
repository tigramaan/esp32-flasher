# Catalog: Tools

| Tool | Назначение | Input | Output | Проверка |
|---|---|---|---|---|
| `programmer-pack` | optional legacy package generation/validation | ESP-IDF build folder + metadata | atomic package folder / summary | Rust integration tests |
| `tools/test-fixtures/generate.ps1` | deterministic direct/legacy BIN fixtures | fixed repository paths | image headers + MD5 partition table | `-DryRun`, `-Verify` |
| `tools/traceability/verify.ps1` | проверка покрытия требований матрицей | project root | text/JSON result | exit code + counts |
| Cargo | build/test/lint Rust workspace | workspace | binaries and reports | fmt/clippy/test |
| npm/Vite/Vitest | frontend deps/build/tests | package-lock + TypeScript | `dist/` | audit/test/build |
| Tauri CLI | portable Windows release | dist + Rust backend | raw EXE | build smoke |
| GitHub Actions | reproducible Windows artifact/signing | source commit | CI artifact | workflow run |
| `tools/site/verify.mjs` | validate public Pages metadata/contracts | `site/` | deterministic PASS/errors | Node smoke |
| GitHub Pages Actions | publish static site | `site/` | deployed Pages URL | deployment smoke |

`programmer-pack` поддерживает `--dry-run`, детерминированные ошибки и не
перезаписывает output без `--force`. Полное описание находится в
`tools/firmware-packager/README.md`.
