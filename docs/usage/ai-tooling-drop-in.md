# Using ripr as an AI Tool Drop-in for `sed -n`

This guide is for AI tool operators — people who configure which commands an AI assistant is permitted to call and how those calls are routed.

## The problem

AI coding assistants commonly read specific line ranges from files using:

```sh
sed -n '5,10p' /path/to/file.txt
```

`sed` is a **stream editor**. It can substitute (`s/…/…/`), delete (`d`), insert (`i`), and write to files. Because of these write capabilities, most AI tool sandboxes block `sed` behind an interactive permission prompt — even when it is being used in a completely read-only manner with `-n … p`.

The result: the AI must pause and ask the user "Can I run `sed`?" for every file read. On a project with many files, this becomes friction that degrades the assistant's usefulness.

## The solution

1. Install `ripr` (see [README.md](../../README.md#install)).
2. Whitelist your project directory (once, not per file).
3. Configure your AI tool to allow `ripr` instead of `sed`.

`ripr` is read-only by design — it has no write capabilities. It can only output lines from files that are explicitly whitelisted. This makes it safe to approve once at the tool level with no further prompts for subsequent reads.

## Whitelisting a project

```sh
ripr whitelist add /path/to/project
```

This grants `ripr` read access to every file under `/path/to/project`. The whitelist entry persists in the ripr config file (`~/.config/ripr/config.toml` on Linux, `~/Library/Application Support/ripr/config.toml` on macOS) and survives shell sessions.

To verify:

```sh
ripr whitelist list
```

## Using ripr directly

Instead of:

```sh
sed -n '5,10p' file.txt
```

use:

```sh
ripr 5-10 file.txt
# or equivalently in sed syntax:
ripr -n '5,10p' file.txt
```

Configure your AI tool's system prompt to direct it toward `ripr RANGE FILE` for file reading. Because `ripr` also understands sed's `-n 'N,Mp'` expression syntax, no changes to the tool's output format are required if it already emits sed-style invocations.

## Using ripr as a drop-in for sed invocations

`alias sed='ripr'` will not work in the general case — ripr only implements a read-only subset of sed, and aliasing would break legitimate sed use elsewhere.

Instead, configure your AI tool's allowed command list to map `sed -n` patterns to `ripr -n`. Most AI tool platforms support an allowlist of permitted commands. A generic example:

```yaml
# Pseudocode — adapt to your platform's config format
allowed_commands:
  - command: ripr
    description: "Read line ranges from whitelisted files"
    # ripr is explicitly approved; no per-invocation prompt needed

blocked_commands:
  - sed    # or route sed -n invocations to ripr instead
  - awk
```

For platforms that support command routing or shell hooks, you can transparently redirect `sed -n` calls to `ripr -n`:

```sh
# Example shell wrapper for AI tool environments
sed() {
  if [[ "$1" == "-n" ]]; then
    ripr -n "$2" "${@:3}"
  else
    command sed "$@"
  fi
}
```

## The exit code 2 advantage

When `ripr` is asked to read a file that is not on the whitelist, it exits with code **2** (access denied) and prints:

```
ripr: access denied: /path/to/file
      → to allow access, run: ripr whitelist add '/path/to/file'
```

AI tools can detect exit code 2 specifically and know that:

- The file exists (exit 1 would indicate ENOENT).
- The problem is the whitelist, not the path.
- The exact remediation command is in the error message.

This allows the AI tool to automatically surface a targeted prompt ("Should I whitelist this directory?") rather than treating the error as a general failure and retrying or stopping.

## Example: per-project whitelist with isolated config

For a CI or sandboxed environment where you don't want to touch the global config:

```sh
# Create a project-local config
ripr --config .ripr.toml whitelist add "$(pwd)"

# Run the AI tool with the project config
RIPR_CONFIG=.ripr.toml your-ai-tool ...
```

The `--config` flag and `RIPR_CONFIG` environment variable override the default config location, enabling fully isolated per-project whitelists.

## Summary

| Step | Command |
|------|---------|
| Install | `cargo install ripr` |
| Whitelist project | `ripr whitelist add /path/to/project` |
| Verify whitelist | `ripr whitelist list` |
| Read a file (native) | `ripr 5-10 file.txt` |
| Read a file (sed syntax) | `ripr -n '5,10p' file.txt` |
| Check exit code | 0=ok, 1=io error, 2=not whitelisted, 3=parse error |
