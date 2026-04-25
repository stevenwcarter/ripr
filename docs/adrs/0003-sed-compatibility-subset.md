# 0003: Sed Compatibility Subset

## Status
Accepted

## Context
AI coding tools frequently emit `sed -n 'N,Mp' file` invocations to read line ranges. Because `sed` is an editor with write capabilities, it is commonly blocked behind permission prompts even when used in a read-only manner (`-n ... p`). The goal of `ripr` is to serve as a safe, read-only replacement for these invocations. For `ripr` to function as a drop-in, it should accept the same expression syntax that AI tools already produce — without requiring those tools to be re-prompted or their output to be rewritten.

## Decision
Accept a subset of sed `-n` expressions via `ripr -n EXPR FILE`:

- `Np` — print single line N
- `N,Mp` — print lines N to M inclusive
- `$p` — print last line
- `N,$p` — print from line N to end of file
- Multiple expressions via `;` (e.g., `-n '5,10p;20,25p'`)

Rejected forms (with explicit parse errors pointing to native ripr syntax):

- `/regex/p` — regex address matching
- `!p` — inversion
- `n~m` — step/stride expressions
- `$` as a start address (e.g., `$,10p`)

## Consequences
Drop-in coverage for the most common sed read patterns AI tools produce. Unsupported sed expressions produce a clear parse error that names the rejected token and suggests the equivalent native ripr syntax — rather than silently doing nothing or producing wrong output. Tools that emit sed invocations can be redirected to `ripr -n` with no prompt re-engineering.

## Alternatives considered
- **Full sed interpreter**: Out of scope and defeats the purpose of a focused, auditable, read-only tool. A full sed interpreter would reintroduce write capabilities (substitution, deletion, insertion) and expand the attack surface.
- **Accept but ignore unsupported patterns**: Silently ignoring `/regex/p` would produce empty output with exit 0, which is more confusing than a clear error.
- **No sed compatibility at all**: AI tools would continue to trigger permission prompts for `sed` rather than being redirectable to `ripr`, eliminating the primary operational benefit.
