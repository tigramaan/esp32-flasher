# generate.ps1

Создаёт минимальные ESP32 image headers и checksum-valid `partitions.bin` для
ESP-IDF и direct PlatformIO/update fixtures. Это fixtures, а не рабочая прошивка.

```powershell
powershell -ExecutionPolicy Bypass -File tools/test-fixtures/generate.ps1 -DryRun
powershell -ExecutionPolicy Bypass -File tools/test-fixtures/generate.ps1
powershell -ExecutionPolicy Bypass -File tools/test-fixtures/generate.ps1 -Verify
```

Скрипт ограничен заранее заданными путями внутри `tests/fixtures`, проверяет
полное содержимое каждого файла. Другие файлы не изменяет.
