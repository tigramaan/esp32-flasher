# Traceability verifier

Read-only PowerShell tool that verifies every `REQ-XXX` from the feature
specification has one complete row in `specs/TRACEABILITY_MATRIX.md`.

## Input

- `-ProjectRoot <path>` — optional project root; defaults to this repository.
- `-Json` — emit a machine-readable JSON result.

## Output

Exit code `0` and `PASS` when requirement counts match and every matrix row has
contract/behavior, tasks and verification columns. Exit code `1` includes
missing, unexpected or incomplete rows.

## Examples

```powershell
tools\traceability\verify.ps1
tools\traceability\verify.ps1 -Json
tools\traceability\verify.ps1 -ProjectRoot D:\firmwares\programmer
```

The tool does not modify files, so a dry-run mode is not applicable.
