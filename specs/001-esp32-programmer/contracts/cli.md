# `programmer-pack` CLI Contract v1

## Commands

```text
programmer-pack factory --build-dir <path> --out <path>
  --package-id <id> --version <value> --success-marker <value>
  [--display-name <value>] [--monitor-baud <value>]
  [--success-timeout-ms <value>] [--dry-run] [--force]

programmer-pack update --build-dir <path> --out <path>
  --package-id <id> --version <value> --success-marker <value>
  --rollback <enabled|disabled>
  [--display-name <value>] [--monitor-baud <value>]
  [--success-timeout-ms <value>] [--dry-run] [--force]

programmer-pack validate <package-path> [--json]
```

## Exit codes

- `0`: success;
- `2`: invalid arguments;
- `4`: invalid ESP-IDF build metadata, package or existing target without `--force`;
- `6`: filesystem failure;
- `10`: internal invariant failure.

## Output

Human-readable UTF-8 is the default. `validate --json` returns a stable JSON `PackageSummary`. Diagnostic output goes to stderr. `--dry-run` performs all source validation and prints the planned files/manifest without creating the destination.

## Guards

- build/output canonical paths must differ;
- output cannot be an ancestor of build-dir;
- absolute or escaping paths in `flasher_args.json` are rejected;
- encrypted segments are rejected in v1;
- missing chip or partition metadata fails with an actionable error;
- an existing output is never removed without `--force`;
- publication uses a sibling staging directory and rename.
