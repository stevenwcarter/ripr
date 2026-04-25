# Onboarding a Project with ripr

This guide walks through setting up `ripr` for a new project so that AI coding tools (or any script) can read line ranges from your project files without triggering permission prompts.

## 1. Install ripr

```sh
cargo install ripr
```

Or download a pre-built binary from the [releases page](https://github.com/stevenwcarter/ripr/releases) and place it on your `PATH`. See [README.md](../../README.md#install) for details.

## 2. Whitelist the project directory

Navigate to your project root and whitelist it:

```sh
ripr whitelist add "$(pwd)"
```

This grants ripr read access to every file under the current directory. The entry is stored in your ripr config file and persists across shell sessions.

If this is your first `whitelist add`, ripr will create the config file automatically. No manual config file creation is needed.

## 3. Verify the whitelist

```sh
ripr whitelist list
```

You should see the absolute path to your project directory in the output. Example:

```
/home/user/src/myproject
```

## 4. Test read access

Try reading the first few lines of a file to confirm access is working:

```sh
ripr 1-5 README.md
```

You should see lines 1–5 of the file. If you see an access denied error instead, double-check that the path printed in the error matches what `ripr whitelist list` shows (paths are canonicalized, so symlinks may resolve differently than expected).

## 5. Using ripr in your workflow

Once whitelisted, you can read any file in the project:

```sh
# Read lines 20–40 of a source file
ripr 20-40 src/main.rs

# Read the last line
ripr '$' Cargo.toml

# Read two non-contiguous ranges in one call
ripr '1-5;50-60' src/lib.rs

# Sed-compatible syntax (useful when redirecting AI tool output)
ripr -n '20,40p' src/main.rs

# Read from stdin (whitelist not required)
cat src/main.rs | ripr 20-40 -
```

## 6. Revoking access

To remove a project from the whitelist:

```sh
ripr whitelist remove "$(pwd)"
```

This works even if the project directory has been moved or deleted — ripr stores the path as a string and removes it by string match, no filesystem access required.

## 7. Using a custom config per project

If you want an isolated whitelist that does not affect your global ripr config (useful for CI environments or shared machines), you can use a project-local config file.

Via the `--config` flag:

```sh
ripr --config .ripr.toml whitelist add "$(pwd)"
ripr --config .ripr.toml 1-5 README.md
```

Via the `RIPR_CONFIG` environment variable (useful for scripting and CI):

```sh
export RIPR_CONFIG="$(pwd)/.ripr.toml"
ripr whitelist add "$(pwd)"
ripr 1-5 README.md
```

Priority order: `--config` > `RIPR_CONFIG` > platform default config location.

You may want to add `.ripr.toml` to your `.gitignore` if you use a project-local config, since it contains absolute paths that are machine-specific.

## 8. What to do if you get exit code 2 (access denied)

If ripr prints an access denied error:

```
ripr: access denied: /home/user/src/myproject/src/main.rs
      → to allow access, run: ripr whitelist add '/home/user/src/myproject/src/main.rs'
```

The error message already tells you exactly what to run. You can whitelist either the specific file or (more commonly) the parent directory:

```sh
# Whitelist just this file
ripr whitelist add '/home/user/src/myproject/src/main.rs'

# Or whitelist the whole project (recommended)
ripr whitelist add '/home/user/src/myproject'
```

After adding to the whitelist, retry your original command — no restart needed.

> Note: Exit code 2 means "not whitelisted." Exit code 1 means an I/O error (such as file not found). If you get exit code 1, check that the file path is correct rather than adding it to the whitelist.

## Quick reference

| Task | Command |
|------|---------|
| Whitelist current directory | `ripr whitelist add "$(pwd)"` |
| List whitelisted paths | `ripr whitelist list` |
| Read lines 1–5 | `ripr 1-5 file.txt` |
| Read last line | `ripr '$' file.txt` |
| Read multiple ranges | `ripr '1-5;20-25' file.txt` |
| Sed syntax | `ripr -n '1,5p' file.txt` |
| Remove from whitelist | `ripr whitelist remove "$(pwd)"` |
| Use project-local config | `RIPR_CONFIG=.ripr.toml ripr ...` |
