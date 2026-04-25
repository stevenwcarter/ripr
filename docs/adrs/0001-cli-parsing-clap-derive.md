# 0001: CLI Parsing — clap with derive

## Status
Accepted

## Context
A CLI parser is needed for Rust. Multiple options exist with varying levels of ergonomics, feature support, and compile-time overhead. The ripr CLI has a non-trivial shape: a global `--config` flag, a positional `RANGE` argument, one or more file arguments, a `-n EXPR` sed-compat mode, and a `whitelist` subcommand. The `-n` mode creates ambiguity: when clap sees `ripr -n '5,10p' file.txt`, it must not accidentally assign `file.txt` to the range slot.

## Decision
Use `clap` with the `derive` feature. The primary invocation (`ripr RANGE FILE...`) merges the range and file arguments into a single `Vec<String>` of positional args, with in-code splitting based on whether `-n` is present. The `whitelist` subcommand is declared explicitly via clap's subcommand derive support.

## Consequences
The derive macro provides automatic `--help` and `--version` output and type-safe argument structs with minimal boilerplate. The `Vec<String>` positional design — rather than separate `range: Option<String>` + `files: Vec<PathBuf>` fields — was necessary to avoid clap incorrectly assigning the file path to the range slot when running in `-n EXPR FILE` mode. The trade-off is that argument splitting and validation happens in application code rather than at the parser layer.

## Alternatives considered
- **`argh`**: Lacks subcommand support, making the `whitelist add/remove/list` subcommand hierarchy awkward to express.
- **`pico-args`**: Too low-level; subcommand dispatch and help generation would all require hand-rolling.
- **Hand-rolled parser**: Unnecessary complexity given that clap is the de facto standard and its derive API keeps boilerplate low.
