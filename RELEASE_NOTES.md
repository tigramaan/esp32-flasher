# ESP32 Flasher v0.1.1

This maintenance release improves recovery and automatic bootloader entry for
ESP32 boards connected through Windows USB-UART adapters.

## Highlights

- Adds a visible **STOP** action for active flash operations.
- Runs flash I/O in an isolated worker so cancellation and timeouts release the
  COM port without freezing the main window.
- Limits ROM bootloader connection to three diagnostic attempts and prevents
  automatic retries after erase or write has started.
- Uses two esptool-compatible Windows DTR/RTS reset profiles before the espflash
  fallback, improving CH340 auto-reset compatibility.
- Adds explicit connection and five-minute flash-stage timeouts.
- Keeps the UART monitor locked during flashing and restores normal operation
  after completion, cancellation, or failure.

## Validation

- Both esptool-compatible reset profiles completed ROM handshake, chip
  detection, flash parameter reads, and RAM stub loading on an ESP32-D0WD-V3
  connected through CH340.
- Rust, TypeScript, layout, traceability, and portable release checks pass.

## Download

Download `ESP32-Flasher-Windows-x64.exe` and optionally verify it with the
adjacent `.sha256` file.

## Important notes

- The Windows release may be unsigned until Authenticode signing is configured,
  so SmartScreen can display a warning.
- Firmware signatures are not verified. Use firmware obtained from a trusted
  source.
- Stopping an operation after erase or write begins can leave an incomplete
  image; perform a complete flash again before using that board.
- WebView2 is a Windows system dependency.

## Acknowledgements

ESP32 Flasher is based on
[soofdev/serial-monitor](https://github.com/soofdev/serial-monitor) and is
released under the MIT License.
