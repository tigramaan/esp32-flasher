# Catalog: API

Public desktop API: [IPC contract](../../specs/001-esp32-programmer/contracts/ipc.md).
Primary input contract: PlatformIO trio/quartet folder or standalone application BIN file,
documented in the [IPC contract](../../specs/001-esp32-programmer/contracts/ipc.md).
Legacy package API: [JSON Schema](../../specs/001-esp32-programmer/contracts/firmware-package.schema.json).
Public CLI: [CLI contract](../../specs/001-esp32-programmer/contracts/cli.md).

## Версионирование

- legacy manifest: `schema_version: 1`;
- portable settings: `schema_version: 1`;
- IPC/CLI: documentation contract v1, packaged with app version `0.1.0`.
- UI locale: `ru`/`en`; `system_locale` is additive IPC v1.

## Compatibility

Unknown legacy JSON fields are rejected. Direct-folder ambiguity fails closed.
Stable error codes may be added, but existing
meaning must not change within v1. Breaking changes require ADR, contract update,
version bump and migration note.
