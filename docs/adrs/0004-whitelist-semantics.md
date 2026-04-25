# 0004: Whitelist Semantics

## Status
Accepted

## Context
The whitelist determines which files ripr may read. Key design questions: how should directory entries behave (match only themselves, or recursively match everything under them?), how should symlinks be handled, and what happens when a whitelisted path no longer exists on disk?

## Decision
All whitelist entries use `starts_with` for matching against the canonicalized target path. This means:

- Directory entries match any file underneath them (recursive, unlimited depth).
- File entries match only themselves, since a canonical file path will not be a prefix of any other canonical path.

Both the target file path and each whitelist entry are canonicalized (symlinks fully resolved) at access-check time. If a whitelist entry no longer exists on disk, its raw stored string is used as a fallback — this allows removal of stale entries without requiring the path to still be present. Nonexistent target paths produce `RipError::Io(ENOENT)` (exit 1), not `AccessDenied` (exit 2), because the issue is the path, not the permissions.

## Consequences
Clean recursive directory semantics without storing metadata about whether each entry is a file or directory — the `starts_with` logic handles both cases correctly by definition. Canonicalization is TOCTOU-safe in the relevant sense: we check permissions at open time, so a path that passes the whitelist check will be opened from the same canonical location. Deleted whitelisted paths can still be cleanly removed from config via `ripr whitelist remove`.

## Alternatives considered
- **Storing file/dir type alongside each entry**: Extra bookkeeping that breaks if the path type changes (file replaced by directory or vice versa) and adds config schema complexity.
- **Glob patterns in whitelist entries**: More expressive but significantly more complex to evaluate correctly (especially for canonicalization and symlink resolution), and deferred to a future version.
- **Using `is_dir()` at check time**: Subject to TOCTOU races, and fails entirely for paths that have been deleted from disk — which is a common scenario when a project directory is moved or removed.
