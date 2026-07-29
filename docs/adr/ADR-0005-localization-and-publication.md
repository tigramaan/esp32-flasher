# ADR-0005 — Windows locale и публичная поставка ESP32 Flasher

## Статус

Accepted, 2026-07-29. Связанные требования: REQ-023—REQ-027.

## Контекст

Portable-приложение должно быть понятным русскоязычным пользователям и
одновременно публиковаться как англоязычный open-source продукт. Ручной выбор
языка увеличивает основной UX, а WebView locale может отличаться от Windows
user locale. Публичная страница и release должны оставаться простыми,
индексируемыми и воспроизводимыми.

## Решение

- До первого render backend получает Windows user default locale через
  `GetUserDefaultLocaleName`.
- `ru-*` выбирает русский каталог; любое другое значение или ошибка — English.
- UI локализует labels/dialogs/statuses, errors по стабильному `ErrorCode` и
  operation messages по стадии. Кириллический backend detail не выводится в
  English mode; исходный detail остаётся в локальном operation log.
- Публичное имя приложения и portable binary — `ESP32 Flasher`.
- README, GitHub Pages и release notes полностью английские.
- Статический `site/` публикуется GitHub Pages Actions и содержит
  SoftwareApplication/FAQ structured data, canonical/OG metadata, sitemap,
  robots и `llms.txt`, совпадающие с видимым содержимым.
- Tagged workflow создаёт portable Windows x64 EXE и SHA-256; installer не
  создаётся. MIT и upstream attribution сохраняются.

## Альтернативы

- `navigator.language` как основной источник — отклонён из-за зависимости от
  WebView preferences; остаётся fallback.
- Ручной language switch — отложен, так как пользователь запросил
  автоматический выбор.
- Jekyll/SPA для Pages — отклонены как лишние runtime/build dependencies.
- Перевод backend domain-текста внутри Rust — отклонён: domain остаётся
  независимым, а стабильная локализация выполняется на UI boundary.

## Trade-offs

- English error details с кириллическим исходником скрываются, поэтому
  пользователь видит стабильное понятное сообщение, а support использует
  локальный diagnostic log.
- Изменение Windows locale применяется при следующем запуске.
- Unsigned release может вызвать SmartScreen, пока не настроен Authenticode
  certificate secret.

## Последствия

Добавляются native locale IPC, RU/EN catalog tests, публичный site validator и
Pages/release workflows. Новый язык требует полного каталога и English-leak
regression. Search metadata нельзя расширять неподтверждёнными рейтингами,
download counts или compatibility claims.
