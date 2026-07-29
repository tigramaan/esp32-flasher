# ADR-0004 — Прямые BIN-входы без обязательного manifest

## Статус

Accepted, 2026-07-29. Связанные требования: REQ-004, REQ-007, REQ-009,
REQ-017—REQ-021.

## Контекст

PlatformIO уже выдаёт `bootloader.bin`, `partitions.bin`, `firmware.bin` и иногда
`boot_app0.bin`. Обязательная предварительная генерация `firmware.json`
добавляет лишний шаг и противоречит минимальному клиентскому UX. Для update
application offset можно получить только из partition table конкретной платы.

## Решение

Основной EXE распознаёт три источника:

- стандартную PlatformIO factory-папку;
- выбранный application BIN с любым именем;
- legacy manifest package для обратной совместимости.

Factory application/otadata offsets берутся из проверенного `partitions.bin`;
bootloader address — из семейства чипа. Partition table direct factory пишется
по стандартному PlatformIO offset `0x8000`. Standalone update выполняет
ограниченное aligned-сканирование flash платы, выбирает неактивный OTA slot и
переключает `otadata` только после verify. Неоднозначность приводит к fail closed.
Если `boot_app0.bin` отсутствует, но layout содержит `otadata`, factory plan
стирает два metadata sector до состояния `0xFF`: bootloader выбирает factory
partition, а при её отсутствии — `ota_0`.

UART marker становится необязательным: без него flash verify является
результатом записи, монитор открывается всегда, `boot_confirmed=false`.

## Альтернативы

- Обязательный manifest — воспроизводим, но ухудшает основной UX.
- Всегда писать application в `0x10000` или `ota_0` — опасно для custom layout
  и активного OTA slot.
- Полностью искать адрес bootloader/partition table эвристикой — создаёт риск
  ложной карты; direct factory ограничен стандартной PlatformIO раскладкой.

## Последствия

CLI packager и manifest остаются advanced workflow. Direct-folder SHA-256 не
аутентифицирует релиз. Custom partition-table offset для factory требует legacy
manifest; update может обнаружить таблицу на плате в ограниченном диапазоне.
