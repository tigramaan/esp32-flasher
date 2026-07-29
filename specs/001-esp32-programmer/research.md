# Research Decisions

## Base project

**Decision**: fork `soofdev/serial-monitor` at `03aa2879b226af831f7b11c1aea2139c6b3f6d79`.

**Rationale**: it already supplies Tauri 2, native Rust serial support, `espflash` integration, progress callbacks and a bidirectional UART monitor under MIT.

**Alternatives considered**: Python/PyInstaller flashers were rejected because the product must not depend on Python; larger generic ESP tools were rejected because their UI and feature set conflict with the minimal two-workflow product.

## Portable Windows runtime

**Decision**: ship a raw Windows x64 Tauri executable with static MSVC CRT and rely on system WebView2.

**Rationale**: Windows 10 April 2018+ and Windows 11 normally provide WebView2, keeping the deliverable small.

**Alternatives considered**: fixed WebView2 runtime adds substantial size; MSI/NSIS violates the no-installer requirement.

## Firmware package (legacy)

**Decision**: strict `firmware.json` schema v1 plus referenced BIN files остаётся
необязательным advanced/legacy форматом.

**Rationale**: manifest полезен для воспроизводимого релиза, но обязательная
генерация ухудшает основной PlatformIO/client UX.

**Alternatives considered**: обязательный manifest отклонён для основного
сценария; полностью удалить формат нельзя без breaking change.

## Direct PlatformIO folders

**Decision**: default UX принимает `bootloader.bin`, `partitions.bin`,
`firmware.bin` и optional `boot_app0.bin` напрямую. `partitions.bin`
checksum-валидируется и задаёт application/otadata offsets. Bootloader address
берётся из chip family. Direct factory использует стандартный PlatformIO
partition-table address `0x8000`; неизвестные дополнительные BIN блокируются.

**Rationale**: PlatformIO уже создаёт необходимые артефакты. Сам `partitions.bin`
не хранит адрес, по которому должен быть записан, поэтому custom factory offset
невозможно безопасно вывести только из этих файлов.

**Alternatives considered**: фиксировать все offsets по именам, сканировать
пустую плату или требовать upload log. Первый вариант ломает custom layout,
второй невозможен для новой платы, третий возвращает обязательные метаданные.

## Standalone application update

**Decision**: выбранный обычный `.bin` с валидным ESP application image является
update. Папка с одним BIN остаётся совместимым IPC-входом, но не основным UX.
После подключения выполняется bounded aligned scan partition table на flash
платы, чтение `otadata`, выбор неактивного слота и metadata switch после verify.
Фиксированный `ota_0` не используется.

**Rationale**: установленная плата является источником истины для своей
разметки; до подключения корректный application offset неизвестен.

**Alternatives considered**: всегда `0x10000`, всегда `ota_0` или требовать
bundled `partitions.bin`. Первые два могут повредить активный образ, последний
усложняет клиентское обновление.

## OTA update

**Decision**: parse the device partition table; write and verify an inactive OTA slot; update one inactive `otadata` sector only after verification.

**Rationale**: the previously active image remains bootable until the final small metadata switch. The sequence and CRC follow ESP-IDF bootloader semantics.

**Alternatives considered**: overwriting the active OTA slot is faster but loses recovery; flashing both slots doubles write time.

## Rollback

**Decision**: legacy manifest может задавать `rollback_enabled`. Direct BIN
использует `false`, потому что bootloader-build property нельзя надёжно вывести
из application image.

**Rationale**: rollback is a bootloader-build property and cannot be inferred reliably from the application image alone.

## Device discovery

**Decision**: enumerate all serial ports and rank known bridge VID/PID without
opening them. Selecting or refreshing a port never toggles DTR/RTS. Active ROM
bootloader detection starts only after the user explicitly starts update/factory
flash; every failed or aborted connection performs a best-effort normal-boot reset.

**Rationale**: opening a USB-UART port can toggle DTR/RTS, enter the ESP32 ROM
bootloader or hold EN low. Enumeration is sufficient for zero-action auto-selection;
chip/MAC/flash size are required only when the guarded operation begins.

## Factory result

**Decision**: оператор может сохранить optional marker в production UI. При
настроенном marker `OK` требует его получения. Для direct folder без marker
`OK` означает successful write/verify и запуск UART monitor; результат явно
содержит `boot_confirmed=false`.

**Rationale**: verify не доказывает достижение application-ready, но marker
невозможно вывести из BIN и он не должен требовать подготовки служебного файла.

## Report format

**Decision**: semicolon-delimited UTF-8 BOM CSV, one file per factory session, flushed after each row.

**Rationale**: opens reliably in common Russian Windows spreadsheet configurations and survives process failure after completed records.

## Windows locale and UI fallback

**Decision**: получать Windows user default locale через
`GetUserDefaultLocaleName`. Только primary language `ru` выбирает русский
каталог; любой другой locale и ошибка native API выбирают английский. Locale не
сохраняется в settings и не выводится из timezone, region, keyboard layout или
WebView state.

**Rationale**: user default locale является документированным Windows
интерфейсом для текущего пользователя и даёт детерминированное поведение до
первого render. English fallback гарантирует доступный интерфейс во всех
неподдерживаемых случаях.

**Alternatives considered**: `navigator.language` оставлен только fallback при
ошибке IPC; ручной language switch и сохранённый override не входят в v1.

## GitHub Pages and search metadata

**Decision**: публиковать статический англоязычный `site/` через official GitHub
Pages actions. Страница содержит canonical, Open Graph, SoftwareApplication и
FAQ JSON-LD, фактический FAQ, version-independent latest-release URL,
`robots.txt`, `sitemap.xml` и `llms.txt`.

**Rationale**: статическая страница не требует runtime dependency, быстро
индексируется и одинаково доступна людям и генеративным поисковым системам.
Structured data дублирует видимый контент и не содержит рейтингов, загрузок или
других неподтверждённых claims.

**Alternatives considered**: Jekyll и отдельный hosting отклонены как лишние
deployment dependencies; keyword stuffing и искусственные comparison claims
запрещены.
