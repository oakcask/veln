---
role: implementation-record
authority: supporting
update-when: Source-written module-header identifier casing evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Source-Written Module Headers

## Outcome

Source-written module headers now use the source identifier casing diagnostic
contract for the module-name declaration token. Current behavior is specified
by [Name Resolution](../../specification/name-resolution.md),
[Check JSON And Diagnostics](../../specification/diagnostics-json.md), and
[Run JSON](../../specification/run-json.md). The checked
`identifier-casing-module-header-json` example fixes the uppercase diagnostic,
the exact token range after non-ASCII-prefixed source text, and the
unresolved-use proof that an invalid header does not provide a normal module
identity. The checked `identifier-casing-module-header-accepted-json` example
fixes the lowercase accepted-control boundary. The run
`identifier-casing-module-header-json` example fixes the selected-source
pre-execution diagnostic envelope for an underscore-led header. Focused parser
coverage fixes the CRLF token range for an underscore-led module header.

## Scope

A parse-clean `mod` header whose name starts with an ASCII uppercase letter or
underscore produces `name.invalid_case` with `phase: name`, `origin: source`,
`occurrence: declaration`, `name_class: module`, `required_initial:
ascii_lowercase`, and the observed initial class. The diagnostic span is the
exact header-name token. The invalid header is preserved in AST lowering and
AST wire round trips as an invalid-name record. It does not assign the invalid
module name to enclosed declarations, does not make that spelling usable as a
normal module identity, and does not produce checked core or typed IR.

Source `mod` declarations remain unsupported as package module identities. A
lowercase source-written header does not report `name.invalid_case`, but it
still reports the existing `module.source_mod` diagnostic in package command
analysis.

## Completion

This slice completes source-written module-header casing for `check` and the
selected-source `run` static gate. It does not complete explicit import-alias
syntax, source-path-derived module identities, written import paths,
language-service definition or rename behavior for module headers, MCP rename
mapping, or other remaining identifier-casing surfaces.
