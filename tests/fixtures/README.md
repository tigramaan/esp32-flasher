# Тестовые firmware fixtures

- `packages/factory-valid` — минимальный структурно корректный factory-пакет.
- `packages/update-valid` — минимальный структурно корректный update-пакет.
- `idf-build` — минимальная ESP-IDF build-папка с `flasher_args.json`.
- `direct/platformio` — стандартная direct factory trio.
- `direct/update` — standalone application BIN.

BIN в `packages/` являются безопасными текстовыми заглушками. Direct/IDF BIN
содержат только синтетические headers/partition entries. Ни один fixture не
предназначен для записи в устройство.
