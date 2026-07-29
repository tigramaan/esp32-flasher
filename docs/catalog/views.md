# Catalog: Views

| View/state | Назначение | Действия | Guards | REQ |
|---|---|---|---|---|
| Update | клиентское app update | single BIN file, port, start | `.bin`, regular file, valid image/device layout, confirmation | REQ-002/004/006/018/019 |
| Factory | серийный cycle | PlatformIO folder, optional UART marker, erase, flash | computed map, marker ≤256 bytes, one device, manual start | REQ-005/009/011/013/017/021 |
| Process terminal | стадии и diagnostics | tab, clear | 10k lines | REQ-008/015 |
| UART terminal | raw boot output | tab, clear, disconnect, horizontal scroll | chunk-safe bounded stream; no soft wrap; marker optional | REQ-008/009 |
| Standalone UART controls | passive auto-connect, baud, reconnect, normal reset | selected port + explicit controls | flash lock, no hidden reconnect, 2 s close timeout | REQ-003/022 |
| Working folder state | portable fallback | choose directory | absolute writable folder | REQ-012 |
| Localized shell | полный RU/EN интерфейс | auto-select до первого render | `ru-*` only, English fallback, no Cyrillic leak | REQ-023/024/025 |
| Public landing | features, workflow, FAQ, download | latest release/source links | factual metadata, responsive layout | REQ-026/027 |

Все представления находятся в одном responsive shell; отдельной навигации и
скрытых wizard state нет.
