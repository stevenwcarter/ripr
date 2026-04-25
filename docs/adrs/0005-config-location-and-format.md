# 0005: Config Location and Format

## Status
Accepted

## Context
The ripr config (which stores the whitelist) must be findable without user configuration on Linux, macOS, and Windows. The format must be simple to read and write, support comments, and be idiomatic for Rust tooling. The config path resolution must support per-project overrides for isolated test environments.

## Decision
TOML format with a single `paths` key containing an array of absolute path strings:

```toml
paths = [
  "/home/user/src/myproject",
  "/home/user/data.csv",
]
```

Config location is resolved via the `etcetera` crate, which provides platform-appropriate config directories:

- **Linux**: `~/.config/ripr/config.toml`
- **macOS**: `~/Library/Application Support/ripr/config.toml`
- **Windows**: `%APPDATA%\ripr\config.toml`

Override priority (highest to lowest):

1. `--config PATH` command-line flag
2. `RIPR_CONFIG` environment variable
3. `etcetera` platform default

A missing config file is not an error — deny-all is the correct default for a new install. The first `ripr whitelist add` call creates the config directory and file automatically.

## Consequences
Zero-config install works correctly: no file means no files are accessible, which is the right security posture. First use is guided by the error message on access denied (exit 2). Config path override via `--config` or `RIPR_CONFIG` enables isolated test environments and per-project configs without modifying the global config.

## Alternatives considered
- **JSON**: No comment support, less idiomatic in the Rust ecosystem, verbosity not warranted for a single-key config.
- **INI / key=value**: Not a standard Rust config format; less expressive if config grows in future versions.
- **XDG-only**: Would use `~/.config` on all platforms, breaking Windows conventions and making the tool feel unpolished on macOS.
- **Storing path kind alongside path**: See ADR 0004 — rejected in favor of `starts_with` matching.
