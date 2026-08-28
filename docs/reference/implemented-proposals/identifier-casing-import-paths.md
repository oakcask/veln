---
role: implementation-record
authority: supporting
update-when: Written import-path identifier casing evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Import Paths

## Outcome

Written import path segments now use the source identifier casing diagnostic
contract for module-class path segments. Current behavior is specified by
[Name Resolution](../../specification/name-resolution.md) and
[Check JSON And Diagnostics](../../specification/diagnostics-json.md). The
checked `identifier-casing-import-path-json` and
`identifier-casing-import-path-human` examples fix exact JSON and human
diagnostics. The checked `identifier-casing-import-missing-module-overlap-json`,
`identifier-casing-import-duplicate-overlap-json`, and
`identifier-casing-import-alias-cascade-boundary-json` examples fix the
corrected overlap behavior.

## Scope

The implemented slice retains one token span for each written import path
segment through parsing, AST lowering, and AST wire round trips. Each segment
requires an ASCII lowercase initial.

The diagnostic uses `name.invalid_case` with `phase: name`, `origin: source`,
`occurrence: path_segment`, the exact written segment spelling, `name_class:
module`, the required and observed initial classes, and the zero-based
`segment_index` inside the written import path.

An implicit alias derived from the final import path segment is the same
occurrence as that final path segment. The implementation emits at most one
casing diagnostic for it. An import with an invalid module path segment does
not enter normal import lookup. Duplicate import-alias analysis still uses the
original written alias. Project-level unresolved import checks still report a
missing selected source for the written local module path. A qualified use
through an invalid implicit alias suppresses `name.unresolved` only when the
selected source export exists and quarantine is the sole failure.

## Completion

This slice is complete. It does not add source-path-derived module identity
casing, explicit import-alias casing, non-import qualified-use segment casing,
recovery navigation, definition, references, prepare-rename, repair rename,
rename conflict prediction, MCP rename mapping, or source migration beyond
the focused executable examples.
