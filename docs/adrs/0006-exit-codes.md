# 0006: Exit Codes

## Status
Accepted

## Context
AI tooling needs to distinguish between different failure modes to take corrective action without prompting the user. In particular, "file not found" and "access denied (not whitelisted)" require different remediation steps: the former means the caller should fix the path, while the latter means the caller should run `ripr whitelist add`. Using only exit 0/1 conflates these two cases and forces the caller to parse stderr — which is brittle and locale-dependent.

## Decision

| Code | Meaning             |
|------|---------------------|
| 0    | Success             |
| 1    | I/O error           |
| 2    | Access denied       |
| 3    | Config/parse error  |

Exit code 2 specifically and exclusively means "the requested file is not on the whitelist." Exit code 1 covers all other I/O errors, including file not found (ENOENT). Exit code 3 covers config file parse errors and range/expression parse errors.

## Consequences
Tooling can detect exit code 2 and know that it should run `ripr whitelist add <path>` rather than treating the error as a general failure or retrying. Exit 1 for a nonexistent file signals that the caller should fix the path, not the whitelist — preventing a useless `whitelist add` for a typo'd filename. The distinction between exit 1 (I/O) and exit 3 (config/parse) allows tooling to detect misconfiguration vs. environmental failures.

## Alternatives considered
- **Conventional 0/1 only**: Loses the access-denied signal entirely, forcing callers to parse stderr text which may change across versions or locales.
- **Stderr-only discrimination**: Brittle; any change in error message wording breaks callers. Not machine-friendly.
- **Separate codes for config vs. parse errors**: Unnecessary granularity for v0.1.0; both indicate a problem the user must fix before retrying, and the error message provides the specific detail.
- **Exit 127 for access denied**: Not conventional for application-defined codes outside a shell context; exit 2 is the established Unix convention for misuse/permission.
