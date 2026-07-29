# ADR-0001 — Rust + Tauri для portable Windows EXE

## Статус

Accepted, 2026-07-29. Связанные требования: REQ-001, REQ-008, REQ-016.

## Контекст

Нужен один EXE без Python, installer и внешнего flashing tool, с современным UI
и прямым COM/ESP bootloader доступом.

## Решение

Использовать Tauri 2: TypeScript/WebView2 UI и Rust backend. `espflash` встроен
как Rust library. Windows MSVC release собирается с static CRT; Tauri bundler
выключен, публикуется raw EXE.

## Альтернативы

- Python/PyInstaller — большой bundle и runtime-антивирусные проблемы.
- Electron — существенно больше размер и память.
- C# WPF — хороший Windows UI, но повторное использование `espflash` потребовало
  бы subprocess или новую реализацию протокола.
- WinUI 3 — packaging/runtime сложнее portable delivery.

## Последствия

Клиенту нужен системный WebView2. Разработчику достаточно VS Build Tools, полная
Visual Studio IDE не требуется. Unsigned build может вызвать SmartScreen.
