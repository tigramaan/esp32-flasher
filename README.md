# ESP32 Flasher

[![Windows build](https://github.com/tigramaan/esp32-flasher/actions/workflows/build.yml/badge.svg)](https://github.com/tigramaan/esp32-flasher/actions/workflows/build.yml)
[![Latest release](https://img.shields.io/github/v/release/tigramaan/esp32-flasher?display_name=tag)](https://github.com/tigramaan/esp32-flasher/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

ESP32 Flasher is a free, open-source, portable Windows application for flashing
ESP32 firmware over UART and monitoring serial output. It is designed for two
jobs: simple firmware updates by end users and fast, repeatable factory
programming.

Download the latest portable build:
[ESP32-Flasher-Windows-x64.exe](https://github.com/tigramaan/esp32-flasher/releases/latest/download/ESP32-Flasher-Windows-x64.exe).
There is no installer and no Python dependency.

Project website:
[tigramaan.github.io/esp32-flasher](https://tigramaan.github.io/esp32-flasher/).

## Features

- One portable Windows x64 EXE.
- Automatic COM port selection when exactly one serial device is present.
- ESP32 chip, MAC address, flash size, image, partition, and range validation.
- Single-file application update: select any valid application `.bin` file.
- Safe OTA updates: writes and verifies an inactive slot before switching boot
  metadata.
- Factory programming from a standard PlatformIO folder containing
  `bootloader.bin`, `partitions.bin`, and `firmware.bin`.
- Computed flash map shown before factory programming.
- Fixed 921600 baud flashing with flash verification.
- Live progress, structured diagnostics, and automatic UART monitoring.
- Standalone UART monitor with connect, disconnect, baud-rate selection, and
  normal board restart.
- Production sessions with passed/failed counters and UTF-8 CSV reports.
- Optional UART ready marker and guarded full-flash erase.
- Automatic interface language: Russian for a Russian Windows locale, English
  for every other locale.
- Local-only operation with no telemetry and no firmware upload.

Supported image families are ESP32, ESP32-C2, ESP32-C3, ESP32-C5, ESP32-C6,
ESP32-H2, ESP32-P4, ESP32-S2, and ESP32-S3.

## Quick start

1. Download `ESP32-Flasher-Windows-x64.exe` from the
   [latest release](https://github.com/tigramaan/esp32-flasher/releases/latest).
2. Put the EXE in a writable folder and run it.
3. Connect one ESP32 board over a data-capable USB cable.
4. Choose an application BIN for an update, or switch to Production and choose
   a PlatformIO build folder.
5. Review the selected port and firmware, then start flashing.
6. After verification, ESP32 Flasher restarts the board and opens the UART
   monitor.

The first run creates a local `data` folder next to the EXE for settings,
operation logs, and production CSV reports. If that folder is read-only, the
application asks you to choose another working folder.

## Firmware inputs

For a normal application update, select one application `.bin` file. The file
may have any name. ESP32 Flasher reads the partition table and OTA metadata from
the connected device before writing and chooses a safe target when one exists.

For factory programming, select a PlatformIO folder:

```text
bootloader.bin
partitions.bin
firmware.bin
boot_app0.bin    # optional
```

ESP32 Flasher parses `partitions.bin`, checks the image chip, computes segment
addresses, validates flash capacity, and displays the write map before the
operation starts.

An optional advanced `programmer-pack` CLI remains available for teams that
need reproducible, versioned legacy packages. It is not required for the normal
desktop workflows.

## UART monitor

Open the UART Monitor tab to monitor a board without flashing it. The selected
port opens passively: the application does not reset the board just to start
monitoring. You can disconnect and reconnect explicitly, choose a supported
baud rate, or restart the board in normal boot mode.

When flashing starts, ESP32 Flasher closes the monitor, waits for the COM port
to be released, locks monitor controls, and then starts the bootloader
connection. The monitor opens again after flashing. Long UART lines are not
wrapped; use horizontal scrolling to inspect them.

## Safety model

- Firmware and device layout are validated before the first flash write.
- OTA metadata is switched only after the new application has been written and
  verified.
- Updating a one-slot device requires explicit confirmation because it is not
  power-loss safe.
- Full-flash erase is off by default, limited to Production mode, and requires
  confirmation.
- The application does not silently reduce the flashing baud rate.
- Firmware signatures are not verified; obtain BIN files from a source you
  trust.
- Public releases may be unsigned until Authenticode signing is configured, so
  Windows SmartScreen may display a warning.

## Build from source

Requirements:

- Windows 10 or 11 x64;
- Node.js 22 or newer;
- Rust stable with the `x86_64-pc-windows-msvc` target;
- Visual Studio 2022 Build Tools with **Desktop development with C++** and a
  Windows SDK;
- system WebView2.

The full Visual Studio IDE is not required.

```powershell
npm ci
npm test
npm run build
cargo test --workspace
npm run build:portable
```

The portable application is created at:

```text
target\release\ESP32 Flasher.exe
```

Optional package tool:

```powershell
cargo build --release -p programmer-pack
```

Full verification commands and hardware procedures are documented in
[specs/VERIFICATION_RUNBOOK.md](specs/VERIFICATION_RUNBOOK.md).

## Project structure

- `src/` — TypeScript UI, state, and RU/EN localization.
- `src-tauri/` — Tauri IPC and Windows serial/flash/filesystem adapters.
- `crates/programmer-core/` — pure ESP image, partition, OTA, and guard logic.
- `tools/firmware-packager/` — optional legacy package CLI.
- `site/` — static GitHub Pages source.
- `specs/` and `docs/` — requirements, contracts, ADRs, traceability, and
  verification runbooks.

## Privacy

ESP32 Flasher has no telemetry, analytics, account, cloud backend, or network
firmware catalog. Firmware, serial output, settings, logs, and reports remain on
the local computer.

## Origin

ESP32 Flasher is based on
[soofdev/serial-monitor](https://github.com/soofdev/serial-monitor), revision
`03aa2879b226af831f7b11c1aea2139c6b3f6d79`. The upstream remote and attribution
are retained.

## License

[MIT](LICENSE). Copyright notices from the original project are preserved.
