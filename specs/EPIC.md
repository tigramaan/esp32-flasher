# EPIC — ESP32 Flasher

## Цель

Создать один portable `ESP32 Flasher.exe` для Windows 10/11 x64, который:

- обновляет клиентские устройства одним application BIN;
- серийно прошивает новые платы обычной папкой PlatformIO;
- читает bundled/device partition table и проверяет folder/device guards;
- показывает прогресс, диагностический лог и UART-монитор;
- не требует Python, установки приложения или внешнего `esptool`.
- автоматически использует русский интерфейс для `ru-*` Windows locale и
  английский во всех остальных случаях;
- публикуется как англоязычный MIT open-source проект с Pages и portable release.

## Объём v1

Источник требований — [feature spec](001-esp32-programmer/spec.md). Реализуются
`REQ-001`—`REQ-027`, пользовательские сценарии `US-001`—`US-005` и публичные
контракты из `specs/001-esp32-programmer/contracts/`.

Не входят: параллельная прошивка нескольких плат, сетевой каталог, автообновление
EXE, подпись firmware-пакета и Linux/macOS.

## Результаты

- `ESP32 Flasher.exe`;
- `programmer-pack.exe`;
- direct PlatformIO folder/standalone BIN contract;
- legacy JSON Schema `firmware.json`;
- Rust/TypeScript тесты;
- CI portable build;
- английские README, GitHub Pages и release notes;
- HIL-процедуры для factory-only, OTA и серийного цикла.

## Приёмка

Программная часть принимается по [Verification Runbook](VERIFICATION_RUNBOOK.md).
Аппаратная приёмка требует реальной ESP32 и отмечается отдельно: отсутствие платы
не заменяется ложным автоматическим результатом.
