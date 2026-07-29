# Verification Runbook

## Требования к машине разработки

- Windows 10/11 x64;
- Node.js 22+;
- Rust stable `x86_64-pc-windows-msvc`, rustfmt, clippy;
- Visual Studio 2022 Build Tools: C++ MSVC + Windows SDK;
- для HIL: ESP32, data-capable USB cable и совместимые PlatformIO BIN.

Полная Visual Studio IDE не требуется.

## Установка

```powershell
npm ci
rustup component add rustfmt clippy
```

## Автоматические проверки

Из Developer PowerShell for VS:

```powershell
npm audit
npm test
npm run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
tools\traceability\verify.ps1
npm run build:portable
cargo build --release -p programmer-pack
```

Ожидание: exit code `0`, audit без уязвимостей, все тесты проходят, файл
`target/release/ESP32 Flasher.exe` и `target/release/programmer-pack.exe` существуют.

## Проверка direct BIN folders

1. Factory: папка с `bootloader.bin`, `partitions.bin`, `firmware.bin` и optional
   `boot_app0.bin` должна показать вычисленную карту напрямую.
2. Update: выбранный application BIN с произвольным именем должен показать
   `адрес с платы`; non-BIN и symlink должны блокироваться.
3. Неполная factory-тройка, два неизвестных BIN и chip mismatch должны
   блокироваться до flash.
4. Legacy manifest fixture должен по-прежнему проходить проверку.

## Проверка optional package tool

```powershell
cargo run -p programmer-pack -- factory `
  --build-dir D:\build `
  --out D:\packages\nova-factory `
  --package-id nova `
  --display-name NOVA `
  --version 1.2.3 `
  --success-marker NOVA_READY `
  --dry-run

cargo run -p programmer-pack -- validate D:\packages\nova-factory
```

Сначала использовать `--dry-run`. Без `--force` существующий output не
перезаписывается.

## Portable smoke

1. Скопировать только `ESP32 Flasher.exe` в новую папку.
2. Запустить обычным пользователем без Python/Rust/Node в `PATH`.
3. Убедиться, что рядом создаётся `data/settings.json`.
4. Повторить в read-only папке: UI должен потребовать рабочую папку.
5. Выбрать standalone update BIN и PlatformIO factory folder; неверный файл и
   неоднозначный factory-набор должны блокироваться.
6. Подать UART-строку несколькими чанками: искусственные переводы не появляются,
   длинная строка не переносится и прокручивается внутри terminal.
7. Проверить системную светлую и тёмную темы, keyboard focus и отсутствие
   горизонтального overflow на окнах 320, 768, 1440 и 3840 CSS px.
8. Запустить EXE с работающей ESP32: автоselection и refresh не должны открывать
   COM-порт, переключать DTR/RTS, сбрасывать плату или останавливать её UART.

## HIL

Подробности: [tests/hil/README.md](../tests/hil/README.md).

- HIL-01: non-invasive auto-selection, multiple ports, reset recovery, busy/disconnect;
- HIL-02: factory-only update with explicit warning;
- HIL-03: OTA inactive slot and power interruption before switch;
- HIL-04: ten factory cycles, marker timeout and CSV reconciliation;
- HIL-05: sustained UART stream and UI responsiveness.
- HIL-06: passive standalone UART, reconnect/reset/baud and flash ownership handoff.

## Traceability

Проверить, что каждая строка `REQ-001`—`REQ-027` существует в
`TRACEABILITY_MATRIX.md`, а direct-folder/legacy manifest/IPC/CLI совпадают с кодом.

## Known limitations

- HIL требует физической платы и не считается пройденным автоматическими тестами.
- Production Pages viewport matrix ожидает ручного включения источника
  `Settings → Pages → GitHub Actions`: доступ агента к GitHub Settings заблокирован
  enterprise browser policy. До включения responsive/layout contract покрыт
  Vitest и `npm run check:site`.
- WebView2 является системной Windows dependency.
- Неподписанный EXE может вызвать SmartScreen; CI signing опционален.
- Direct-folder SHA-256 не подтверждает автора firmware.

## Checklist приёмки

- [x] npm audit/test/build — PASS, 0 vulnerabilities, 26 Vitest (2026-07-29)
- [x] cargo fmt/clippy/test — PASS, 51 tests (2026-07-29)
- [x] `npm run check:site` — PASS, English-only content and SEO/GEO contracts verified (2026-07-29)
- [x] portable release build — PASS, 12,022,784 bytes, SHA-256 `542A9D02071EC787508CD012133E59EA61EA266B41D0687AB8330C9008240CB8` (2026-07-29)
- [x] EXE launch smoke — PASS; title `ESP32 Flasher`, process responsive and cleanly stopped (2026-07-29)
- [x] app visual smoke — PASS на реальной ESP32/COM4: passive monitor, data stream, no line wrap, disconnect and production view (2026-07-29)
- [ ] production Pages viewport matrix 320/390/768/1024/1440/1920/2560/3840 CSS px
- [ ] HIL-01—HIL-06 — PASS на целевом оборудовании
- [x] CSV и logs — автоматические проверки записи, счётчиков, sanitization и лимита PASS (2026-07-29)
- [x] standalone UART automated verification и portable rebuild — PASS (2026-07-29)
- [x] direct-folder fixtures, release verification и traceability — PASS, 27/27 REQ (2026-07-29)
