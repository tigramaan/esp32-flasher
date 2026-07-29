# Quickstart

## Prerequisites

- Windows 10/11 x64;
- Node.js 22+;
- Rust stable MSVC toolchain;
- Visual Studio Build Tools with Desktop development with C++;
- WebView2 Runtime for launching the application.

## Install and validate

```powershell
npm ci
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Development

```powershell
npm run tauri dev
```

## Package tool

Для обычной работы package tool не нужен. Programmer принимает напрямую:

```text
factory/
  bootloader.bin
  partitions.bin
  firmware.bin
  boot_app0.bin       # optional

update/
  firmware.bin
```

`firmware.json` — только backward-compatible/advanced формат. Необязательный CLI:

```powershell
cargo run --manifest-path tools/firmware-packager/Cargo.toml -- `
  factory `
  --build-dir D:\build\nova `
  --out D:\releases\nova-factory `
  --package-id nova `
  --version 1.4.2 `
  --success-marker APP_READY
```

## Portable build

```powershell
$env:RUSTFLAGS='-C target-feature=+crt-static'
npm run tauri build -- --no-bundle
```

Expected artifact:

```text
target\release\ESP32 Flasher.exe
```

## First run

Place `ESP32 Flasher.exe` in a writable folder. The application creates
`data\settings.json`, `data\logs` and `data\reports`. If the folder is
read-only, select a work directory when prompted.
