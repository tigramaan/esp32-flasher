# Security

## Reporting a vulnerability

Please do not open a public issue for a vulnerability that could expose device
data, local files, or firmware. Use GitHub's private vulnerability reporting
feature for this repository when available.

Include the affected version, impact, reproduction steps, and the smallest
sanitized diagnostic sample needed to understand the problem. Do not include
firmware images, credentials, keys, private serial output, or unrelated local
paths.

## Security boundaries

ESP32 Flasher is a local desktop tool. It does not authenticate firmware
publishers and does not verify firmware signatures. Users must obtain firmware
from a trusted source. The application has no telemetry or cloud backend and
runs with the current Windows user's filesystem and COM-port permissions.
