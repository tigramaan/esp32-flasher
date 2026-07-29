# ADR-0002 — Версионированная папка firmware package

## Статус

Superseded by ADR-0004, 2026-07-29. Связанные требования: REQ-004, REQ-010, REQ-014.

## Контекст

Factory требует несколько BIN/offsets, update — один BIN. Выбор произвольных
файлов в UI создаёт риск неправильного адреса, чипа и версии.

## Решение

Единица поставки — папка с `firmware.json` schema v1 и BIN. Manifest задаёт kind,
product/version, chips, monitor policy, OTA policy, sizes, SHA-256 и factory
offsets. Unknown fields и path escape запрещены. Папка создаётся атомарно
`programmer-pack`.

## Альтернативы

- Несколько file pickers — неудобно и нет воспроизводимого контракта.
- ZIP — требует дополнительной extraction/security boundary.
- Один merged BIN — удобен для factory, но не решает безопасный app-only update.

## Последствия

SHA-256 обнаруживает повреждение, но не доказывает автора. Подпись package может
быть добавлена новой версией schema.
