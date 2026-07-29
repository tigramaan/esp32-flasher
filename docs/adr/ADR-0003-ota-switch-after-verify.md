# ADR-0003 — Переключение OTA только после verify

## Статус

Accepted, 2026-07-29. Связанные требования: REQ-006, REQ-007, REQ-010.

## Контекст

Отключение питания во время обновления не должно разрушать ранее активный образ
на OTA-устройстве.

## Решение

Прочитать partition table и otadata, выбрать неактивный OTA slot, проверить
capacity, записать app с verify, затем записать новую запись в неактивный
otadata sector с ESP ROM CRC. Reset выполняется после закрытия bootloader session
и открытия UART reader.

## Альтернативы

- Всегда писать factory partition — нет power-loss safety.
- Переключить otadata перед app — может загрузиться неполный образ.
- Использовать OTA через само приложение — требует сетевого/firmware protocol.

## Последствия

Повреждённая partition/otadata блокирует update. Factory-only устройство
поддерживается отдельным явно подтверждаемым in-place сценарием.
