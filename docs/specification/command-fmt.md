---
role: specification
authority: normative
update-when: The veln fmt command parsing gate, formatting behavior, source discovery behavior, or write policy changes.
---

# Format Command

`fmt` uses the same source discovery rule as `check`. It parses every selected
file before writing any file. If any parse diagnostic is present, the whole
format invocation exits with failure and writes nothing.

For parse-clean files, formatting is deterministic for the implemented syntax:
use declarations, function signatures, contract clauses, let statements,
tail expressions, holes with `satisfy`, records, lists, calls, literals, paths,
prefix operators, binary operators, postfix `?`, and supported binary schema
primitive compatibility spellings.

Canonical indentation uses one tab character per indentation level. Top-level
imports, item signatures, and item-closing `end` lines use
indentation level 0. Function body lines, including contract clauses, `let`
statements, tail expressions, and standalone comments attached to those lines,
use indentation level 1.

For formatted `match` expressions, the `match` line uses the parent expression
indentation level, each arm is one indentation level deeper than that `match`
line, and the `match` closing `end` aligns with the `match` line.
When a parse-clean `match` has exactly one `true` arm and one `false` arm,
`fmt` canonicalizes it to `if` / `else`; false-arm continuations that are also
ordinary `true` / `false` matches become `else if`. When a parse-clean boolean
`match` compares the same scrutinee to string, integer, float, or unit literals
through a `true` arm and a `false` continuation chain, `fmt` instead
canonicalizes it to a direct literal `match` with a wildcard fallback.
Commented rewritable matches are left in their lossless source form.

Formatting accepts multiple parse-clean input files in one invocation and
writes each selected file only after all selected files have parsed without
diagnostics. The implemented golden coverage includes `ensure` clauses, prefix
and binary precedence, postfix `?`, nested records, lists, calls, and
idempotent formatting across multiple input files. In `format binary` schemas,
supported compatibility spellings such as `UIntN`, representable
`ReservedBits(width, value)`, and `Repeat(count, Payload)` are formatted as
canonical lowercase field text, including dispatch payload field text.

Standalone line comments attach to the next parsed source line during
formatting. The formatter emits hash comments with the same indentation as the
formatted import, function signature, contract clause, body line, or closing
`end` line it documents. Comment-only lines between imports, function
signatures, contract clauses, body lines, and closing `end` lines do not
prevent parsing or deterministic formatting of those declarations. Trailing
line comments after source code stay on the same formatted source line.
`veln fmt` formats parse-clean source only; it does not migrate slash-prefixed
comment-like text.

