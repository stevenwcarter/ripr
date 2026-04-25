# 0002: Range Grammar

## Status
Accepted

## Context
A range syntax is needed that is human-friendly, unambiguous, and avoids conflicts with shell special characters when used unquoted. The syntax must also accommodate exclusive-bound ranges (common in APIs and documentation) and multi-range requests in a single invocation. Existing tools use different conventions: sed uses commas as range separators, Python uses `[start:stop]` with exclusive ends, and many CLIs accept point-lists via comma.

## Decision
Dash `-` and comma `,` both denote inclusive ranges (not point-lists). Opening parenthesis `(` makes the start bound exclusive; closing parenthesis `)` makes the end bound exclusive. The multi-range separator is `;`, borrowed from sed's expression separator. `$` is reserved to mean the last line of the file. Lines are 1-indexed; line 0 is rejected.

| Syntax      | Meaning                              |
|-------------|--------------------------------------|
| `5-10`      | Lines 5–10 inclusive                |
| `5,10`      | Lines 5–10 inclusive                |
| `(5-10`     | Lines 6–10 (exclusive start)        |
| `5-10)`     | Lines 5–9 (exclusive end)           |
| `(5-10)`    | Lines 6–9 (both exclusive)          |
| `5-10;20-25`| Lines 5–10 and 20–25 in one call   |
| `$`         | Last line                           |
| `5-$`       | Lines 5 to end of file              |

## Consequences
`5,10` meaning "lines 5 through 10 inclusive" surprises users who expect comma to mean "these specific lines" — this must be documented explicitly. Ranges containing `(`, `)`, or `;` must be shell-quoted in most shells. The grammar is simple enough to parse with a hand-written parser with no external dependencies.

## Alternatives considered
- **Comma as point-list** (`5,10` = lines 5 and 10 only): Inconsistent with sed convention and less useful for the primary use case of reading a contiguous block.
- **Square brackets for exclusivity** (`[5-10)` = exclusive end): Square brackets are already used by shell globbing and would require quoting in common contexts.
- **`..` as range separator** (`5..10`): Conflicts with shell brace expansion in some shells, and `..` has no established convention for inclusive vs exclusive bounds.
- **Slash-delimited** (`5/10`): Visually confusing in file path contexts.
