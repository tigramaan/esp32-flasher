# Integration Handover

## Артефакты

- приложение: `target/release/ESP32 Flasher.exe`;
- package tool: `target/release/programmer-pack.exe`;
- direct folder contract: `specs/001-esp32-programmer/contracts/ipc.md`;
- legacy package schema: `specs/001-esp32-programmer/contracts/firmware-package.schema.json`;
- CI artifact: `esp32-flasher-windows-x64`;
- HIL procedure: `tests/hil/README.md`.

## Release

CI собирает raw EXE на `windows-latest`. Если заданы secrets сертификата,
выполняется Authenticode; иначе артефакт явно остаётся unsigned. Installer не
создаётся. Tag `v*` публикует `ESP32-Flasher-Windows-x64.exe` и `.sha256`;
README/Pages используют version-independent latest-release URL.

## Firmware team

Для update поставляет один application BIN с любым именем. Для factory —
`bootloader.bin`, `partitions.bin`, `firmware.bin` и optional `boot_app0.bin`.
`programmer-pack` и manifest не обязательны. Для direct factory оператор может
сохранить optional UART marker в UI. Если marker используется, firmware выводит
его только после завершения инициализации; при rollback — после подтверждения
firmware.

## Support diagnostics

Пользователь передаёт:

- версию ESP32 Flasher;
- код ошибки из UI;
- файл `data/logs/<operation-id>.log`;
- при factory — соответствующую строку `data/reports/factory-*.csv`;
- тип платы/кабеля и COM-порт.

Не запрашивать содержимое NVS или firmware BIN без необходимости.

## Migration

Это первая версия публичных контрактов. Любое несовместимое изменение
direct-folder convention, `firmware.json`, IPC DTO или CLI требует ADR,
contract update и при breaking change нового version/migration notes.
