# System Specification

## Контекст

ESP32 Flasher — локальное desktop-приложение без сетевых функций. Недоверенные входы:
папка с прямыми BIN или legacy manifest, COM-порт и UART-байты. Изменяемые ресурсы:
flash одной выбранной платы и выбранная portable data directory.

## Компоненты

| Компонент | Ответственность | Контракт |
|---|---|---|
| TypeScript UI | режимы, ввод пользователя, отображение state/events | `contracts/ipc.md` |
| Tauri IPC | строгие DTO и запуск blocking workflows вне UI thread | `src-tauri/src/ipc` |
| Application | одна операция, update/factory orchestration, события | `OperationEvent` |
| programmer-core | ESP image/partition parsing, legacy manifest, guards, OTA, state, errors | Rust public API |
| Platform | espflash, serial, filesystem, logs, CSV | внутренние module API |
| programmer-pack | сборка/проверка папок поставки | `contracts/cli.md` |
| Locale boundary | Windows locale и RU/EN user copy | `system_locale`, `src/i18n.ts` |
| Public site | англоязычная документация и latest release | `site/`, Pages workflow |

Направление зависимостей: `UI -> IPC -> application -> core/platform`.
Core не зависит от UI, Tauri, serial или filesystem.

## Поведение update

1. Распознать единственный application BIN либо legacy manifest и проверить ESP image chip.
2. Подключиться к выбранному ROM bootloader на `921600`.
3. Проверить chip и flash capacity.
4. Обнаружить и прочитать partition table с платы и, если существует, `otadata`.
5. Для OTA выбрать неактивный slot; записать и проверить app, затем записать
   новый otadata sector. Для factory-only запросить явное подтверждение.
6. Открыть UART reader и выполнить reset; ждать marker только если он настроен.
7. При отсутствии marker вернуть `success=true` после flash verify с `boot_confirmed=false`.

## Поведение factory

1. Распознать стандартные PlatformIO BIN или legacy package и создать/переиспользовать сессию.
2. Разобрать bundled `partitions.bin`, вычислить карту, проверить chip, диапазоны и flash capacity.
3. При подтверждённой опции выполнить полный erase.
4. Записать все сегменты по offsets на `921600`, verify включён.
5. Открыть монитор, reset и ждать marker только если он настроен.
6. Немедленно flush одной CSV-строки `OK`/`ERROR`, обновить счётчики.
7. Оставаться в мониторе до отключения; следующий цикл только по кнопке.

## Поведение standalone UART

1. Вход на вкладку UART при выбранном порте выполняет passive open без reset.
2. Переход на «Процесс» не закрывает монитор и не теряет входящие чанки.
3. Ручной disconnect останавливает поток, ждёт закрытия COM до двух секунд и
   запрещает автоподключение до явной команды.
4. Reset выполняется внутри владеющего портом monitor thread и запускает normal
   boot без входа в ROM bootloader.
5. Flash operation атомарно блокирует monitor-команды, закрывает monitor с
   ожиданием и только затем открывает ROM bootloader.

## Guards

- folder root canonicalized; symlink, path traversal и неоднозначные BIN запрещены;
- direct factory принимает только известную trio/quartet PlatformIO и проверенную partition table;
- standalone update обнаруживает partition table ограниченным aligned scan;
- manifest v1 использует `deny_unknown_fields`;
- package ограничен 256 MiB, segment — 64 MiB;
- ranges не пересекаются и помещаются во flash/partition;
- фоновое обнаружение только перечисляет COM-порты; открытие порта и DTR/RTS
  разрешены после явного запуска операции;
- каждая созданная ESP-сессия завершается normal-boot reset до открытия
  UART-монитора; ошибка до создания сессии выполняет best-effort recovery;
- concurrent operation и подключённый старый monitor блокируют старт;
- retry ROM handshake ограничен библиотекой; baud fallback отсутствует;
- terminal buffer ограничен 10 000 строк;
- direct monitor принимает только шесть разрешённых baud; скрытые retry запрещены;
- operation logs ограничены 10 MiB на файл и 100 MiB на каталог.
- locale `ru-*` является единственным входом в русский каталог; любое другое
  значение и native failure детерминированно выбирают English;
- English UI не выводит непереведённые кириллические backend details;
- public site не выполняет runtime network requests, кроме явного перехода по
  download/source links, и не содержит неподтверждённых search claims.

## Ошибки и наблюдаемость

Публичная ошибка: `code`, `message`, optional `detail`, `retryable`. Коды
стабильны и покрыты contract tests. Каждая стадия направляется в UI channel и
operation log. Factory result дополнительно фиксируется в UTF-8 BOM CSV.
BIN-содержимое, секреты и произвольные UART-данные не записываются в telemetry;
telemetry и сеть отсутствуют.

Внутренний operation log сохраняет исходную диагностику для поддержки. UI
локализует errors по стабильному `ErrorCode`, progress/state по стадии; raw UART
никогда не переводится, поскольку является пользовательскими данными устройства.

## Security boundaries

- приложение не аутентифицирует локального пользователя;
- firmware package доверяется только после guards, но SHA-256 не является
  цифровой подписью;
- COM и filesystem используются с правами текущего пользователя;
- полный erase отделён подтверждением;
- CSP запрещает произвольные remote sources;
- runtime source проверяется regression test на отсутствие network clients.
- Windows locale не считается sensitive data, не сохраняется и не логируется;
- Pages не собирает analytics, cookies, firmware или device data.
