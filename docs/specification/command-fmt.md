---
role: specification
authority: normative
update-when: The veln fmt command parsing gate, formatting behavior, source discovery behavior, or write policy changes.
specification-coverage: usage=#format-command; behavior=#formatting-rules; limits=#limits-and-errors
---

# Format Command

Use `veln fmt [PATH ...]` to format selected source files. Selection follows
`check`. The command parses every selected file before writing any file. One
parse diagnostic makes the invocation fail and leaves every file unchanged.

## Formatting rules

The formatter deterministically formats imports, function signatures, contract
clauses, `let` statements, tail expressions, holes with `satisfy`, records,
lists, calls, literals, paths, prefix and binary operators, postfix `?`, and
supported binary-schema primitive compatibility spellings. Indentation is one
tab per level: top-level items and closing `end` lines use level zero, and
function body lines use level one.

A `match` line uses its parent indentation; arms use one deeper level and its
closing `end` aligns with the `match`. A two-arm boolean match becomes
`if`/`else`; a false continuation becomes `else if`. A boolean match
against string, integer, float, or unit literals becomes a literal `match`
with a wildcard fallback. Comments make a rewritable match lossless, so it is
left unchanged.

Standalone comments attach to the next parsed source line and receive that
line's indentation. Comment-only lines do not interrupt declarations. Trailing
comments stay on their source line. Slash-prefixed comment-like text is not
migrated.

## Limits and errors

The parse gate applies to the complete selected set; semantic analysis is not
a formatting gate. After that gate, files are written sequentially. A
filesystem write error fails the command but does not roll back earlier
successful writes.

## References

Implementation: `crates/veln-cli/src/commands/fmt.rs`. Formatter golden tests
cover precedence, comments, schema spellings, multi-file atomicity, and
idempotence.
