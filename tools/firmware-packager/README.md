# programmer-pack

Rust CLI для создания проверяемой папки прошивки из ESP-IDF build output.
Поддерживаются именованные записи современных ESP-IDF и legacy-ключ
`partition_table`; если metadata не содержит chip, он определяется по ESP image
header application BIN. Несовпадение metadata и header блокирует публикацию.

## Factory

```powershell
cargo run -p programmer-pack -- factory `
  --build-dir D:\build\nova `
  --out D:\releases\nova-factory `
  --package-id nova `
  --display-name NOVA `
  --version 1.4.2 `
  --success-marker APP_READY
```

## Update

```powershell
cargo run -p programmer-pack -- update `
  --build-dir D:\build\nova `
  --out D:\releases\nova-update `
  --package-id nova `
  --version 1.4.2 `
  --rollback disabled `
  --success-marker APP_READY
```

`--rollback enabled` означает, что приложение вызывает `esp_ota_mark_app_valid_cancel_rollback()` до вывода UART-маркера.

## Проверка и dry-run

```powershell
cargo run -p programmer-pack -- validate D:\releases\nova-update --json
cargo run -p programmer-pack -- factory ... --dry-run
```

Публикация выполняется через staging-каталог. Существующая папка не заменяется без `--force`.

Exit codes:

- `0` — успех;
- `2` — аргументы CLI;
- `4` — неверная сборка или пакет;
- `6` — ошибка файловой системы;
- `10` — внутренняя ошибка.
