# ESP32 Flasher v0.1.0

The first public release of ESP32 Flasher: a portable Windows application for
flashing ESP32 firmware over UART and monitoring serial output.

## Highlights

- One portable Windows x64 EXE with no installer or Python dependency.
- Single-file application updates with device partition-table discovery.
- Safe inactive-slot OTA updates with verification before the boot switch.
- Factory flashing from standard PlatformIO build folders.
- Automatic COM port selection and ESP32 chip validation.
- Standalone UART monitor with baud selection, reconnect, and normal restart.
- Production counters, optional ready marker, and UTF-8 CSV reports.
- Russian interface for a Russian Windows locale; English for every other
  locale.
- Local-only operation with no telemetry or firmware upload.

## Download

Download `ESP32-Flasher-Windows-x64.exe` and optionally verify it with the
adjacent `.sha256` file.

## Important notes

- The Windows release may be unsigned until Authenticode signing is configured,
  so SmartScreen can display a warning.
- Firmware signatures are not verified. Use firmware obtained from a trusted
  source.
- A one-slot in-place update is not power-loss safe and requires explicit
  confirmation.
- WebView2 is a Windows system dependency.

## Acknowledgements

ESP32 Flasher is based on
[soofdev/serial-monitor](https://github.com/soofdev/serial-monitor) and is
released under the MIT License.
