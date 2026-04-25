# ripr

`ripr` reads specific line ranges from files. It exists so AI coding tools (and humans) can read slices of files **without invoking `sed -n '…p'`** — which is `sed`'s edit tool and therefore usually blocked behind a permission prompt even when used read-only. By shipping a dedicated read-only binary with its own bounded access model, `ripr` can be safely whitelisted once. And because `ripr` also accepts `-n 'N,Mp'` sed syntax, tools that already emit sed invocations can be redirected to `ripr` with no prompt re-engineering required.

## Install

### From crates.io

```sh
cargo install ripr
```

> Note: v0.1.0 has not yet been published to crates.io.

### From binary releases

Download a pre-built binary for your platform from the [releases page](https://github.com/stevenwcarter/ripr/releases). Archives are `.tar.gz` on Linux and macOS, `.zip` on Windows. SHA256 checksums are provided for each archive.

Available targets:

- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl` (fully static, suitable for Alpine / Docker scratch)
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

## Quick start

Before ripr can read a file, you must whitelist it (or its parent directory):

```sh
ripr whitelist add "$(pwd)"
```

### Native range syntax

```sh
# Inclusive range with dash
ripr 5-10 file.txt

# Inclusive range with comma (same as dash — not a point-list)
ripr 5,10 file.txt

# Exclusive bounds
ripr '(5-10)' file.txt   # lines 6–9

# Multi-range in one call (semicolon separator)
ripr '5-10;20-25' file.txt

# Last line
ripr '$' file.txt

# From line 5 to end of file
ripr '5-$' file.txt

# Read from stdin (whitelist bypassed)
cat file.txt | ripr 5-10 -

# Multiple files (output concatenated in order)
ripr 1-3 file1.txt file2.txt
```

### Sed-compatible mode (`-n`)

```sh
# Single line
ripr -n '5p' file.txt

# Range
ripr -n '5,10p' file.txt

# From line 5 to end of file
ripr -n '5,$p' file.txt

# Multiple expressions
ripr -n '5,10p;20,25p' file.txt
```

## Range syntax reference

| Syntax       | Meaning                              |
|--------------|--------------------------------------|
| `5-10`       | Lines 5–10 inclusive                |
| `5,10`       | Lines 5–10 inclusive                |
| `(5-10`      | Lines 6–10 (exclusive start)        |
| `5-10)`      | Lines 5–9 (exclusive end)           |
| `(5-10)`     | Lines 6–9 (both exclusive)          |
| `5-10;20-25` | Lines 5–10 and 20–25 in one call   |
| `$`          | Last line                           |
| `5-$`        | Lines 5 to end of file              |

Lines are 1-indexed. Line 0 is rejected.

## Configuration

The whitelist is stored in a TOML config file. Default locations:

| Platform | Path                                                  |
|----------|-------------------------------------------------------|
| Linux    | `~/.config/ripr/config.toml`                         |
| macOS    | `~/Library/Application Support/ripr/config.toml`     |
| Windows  | `%APPDATA%\ripr\config.toml`                         |

Example config:

```toml
paths = [
  "/home/user/src/myproject",
  "/home/user/data.csv",
]
```

### Overriding the config path

```sh
# Via command-line flag (highest priority)
ripr --config /path/to/config.toml 5-10 file.txt

# Via environment variable
RIPR_CONFIG=/path/to/config.toml ripr 5-10 file.txt
```

Priority order: `--config` > `$RIPR_CONFIG` > platform default.

## Whitelist management

```sh
# Add a path (directory or file)
ripr whitelist add /home/user/src/myproject

# Remove a path
ripr whitelist remove /home/user/src/myproject

# List all whitelisted paths
ripr whitelist list
```

- Directory entries match any file under that directory (recursive).
- File entries match only that exact file.
- `whitelist add` is idempotent.
- `whitelist remove` works even if the path no longer exists on disk.
- Operational feedback (`Added:`, `Removed:`) is written to stderr; `whitelist list` output goes to stdout.

## Exit codes

| Code | Meaning                         |
|------|---------------------------------|
| 0    | Success                         |
| 1    | I/O error (e.g., file not found)|
| 2    | Access denied (not whitelisted) |
| 3    | Config or parse error           |

Exit code 2 specifically means "this file is not on the whitelist." The error message tells you exactly what to run:

```
ripr: access denied: /path/to/file
      → to allow access, run: ripr whitelist add '/path/to/file'
```

## Why not just use `sed`?

`sed` is a stream editor with write capabilities: substitution (`s/…/…/`), deletion (`d`), insertion (`i`), and more. AI coding assistants and sandboxed environments typically block `sed` behind a permission prompt — even when it's being used in a completely read-only way via `sed -n '5,10p'`.

`ripr` has no write capabilities. It can only read lines from files that are explicitly whitelisted. This makes it safe to whitelist once at the tool level, with no further permission prompts for subsequent reads. Because `ripr` also accepts `-n 'N,Mp'` sed syntax, AI tools that already emit `sed -n '…p'` invocations can be redirected to `ripr -n '…'` without any changes to the tool's prompts or output parsing.

See [`docs/usage/ai-tooling-drop-in.md`](docs/usage/ai-tooling-drop-in.md) for a practical configuration guide.
