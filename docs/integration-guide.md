# Integration Guide

## Подготовка firmware

Для factory передайте стандартные артефакты PlatformIO в одной папке:

```text
bootloader.bin
partitions.bin
firmware.bin
boot_app0.bin    # optional
```

Для update пользователь выбирает конкретный application `.bin` с любым именем.
Дополнительная подготовка служебного файла не требуется. Optional
`programmer-pack` по
[CLI contract](../specs/001-esp32-programmer/contracts/cli.md) остаётся для
версионированных legacy-релизов.

Если factory layout содержит `otadata`, но `boot_app0.bin` отсутствует,
ESP32 Flasher безопасно инициализирует два OTA metadata sector пустым состоянием.

## Firmware boot marker

Marker необязателен. Для direct PlatformIO-папки оператор вводит и сохраняет
уникальную ASCII/UTF-8 строку, например `NOVA_READY`, в производственном экране.
Legacy manifest использует marker из своего monitor policy. Детектор работает с
raw UART bytes и находит marker между chunks. Без marker flash verify считается
успехом записи, UART открывается, но `boot_confirmed=false`.

UART monitor сохраняет реальные переводы строк. Границы порций чтения COM-порта
не отображаются как новые строки; длинная строка прокручивается горизонтально.

Если ESP-IDF rollback включён, firmware должна подтвердить образ до marker.

## Поддерживаемые chips

Поддерживаются ESP32, C2/C3/C5/C6, H2, P4, S2 и S3. Chip определяется из
application image header и сверяется с подключённым устройством.

## OTA

- OTA layout: ESP32 Flasher пишет неактивный app slot, verify, затем неактивный
  otadata sector.
- Standalone update читает partition table и `otadata` непосредственно с платы.
- Factory-only/one-slot layout: запись активного application partition разрешается только
  после явного подтверждения и не является power-loss safe.
- NVS и другие data partitions в update mode не изменяются.

## Производственная линия

1. Выбрать одну папку PlatformIO и проверить показанную flash-карту.
2. Начать новую сессию.
3. Подключить одну плату, дождаться определения, нажать «Прошить».
4. Дождаться `OK/ERROR`, отключить плату.
5. Повторить. В конце сопоставить counters и CSV.

Опция полного erase удаляет NVS и применяется только по отдельному подтверждению.
