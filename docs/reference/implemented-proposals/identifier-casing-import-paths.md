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
function alias-use overlap behavior. The checked
`identifier-casing-import-type-cascade-boundary-json`,
`identifier-casing-import-constructor-cascade-boundary-json`,
`identifier-casing-import-missing-type-control-json`,
`identifier-casing-import-missing-type-export-json`, and
`identifier-casing-import-missing-constructor-control-json` examples fix the
qualified imported type and constructor quarantine boundary and the
missing-target controls. The checked
`identifier-casing-import-schema-cascade-boundary-json` and
`identifier-casing-import-private-schema-boundary-json` examples fix the
schema composition quarantine boundary, the schema missing-target control,
and the private-schema visibility control. The checked
`identifier-casing-import-effect-cascade-boundary-json` and
`identifier-casing-import-handler-cascade-boundary-json` examples fix the
public effect and handler quarantine boundary. The checked
`identifier-casing-import-order-json` example fixes source-ordering between
an invalid import path segment and a later invalid declaration. The checked
`identifier-casing-import-alias-run-boundary-json` example fixes the same
invalid implicit-alias boundary for `run` reachability. The checked
`identifier-casing-unselected-import-path-json` and
`identifier-casing-unused-import-path-json` examples fix the negative `run`
selection boundary for invalid written import paths outside the selected entry
closure and unused by that closure.

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
not enter normal value, call, type, constructor, schema, effect, handler,
inference, lowering, or reachability consumers.
Duplicate import-alias analysis still uses the original written alias.
Project-level unresolved import checks still report a missing selected source
for the written local module path. A qualified use through an invalid implicit
alias suppresses derivative unresolved, type-origin, constructor-arity, or
exhaustiveness diagnostics, and derivative schema composition diagnostics,
unknown-effect diagnostics, and unknown-handler diagnostics only when a
matching visible public selected source export proves quarantine is the sole
failure. Missing target modules, missing exports, private targets, wrong-kind
targets, and other independently provable failures remain reported.

## Completion

This slice is complete. It does not add source-path-derived module identity
casing, explicit import-alias casing, recovery navigation, definition,
references, prepare-rename, repair rename, rename conflict prediction, MCP
rename mapping, or source migration beyond the focused executable examples.
Non-import qualified-use path casing is completed separately in
[Identifier Casing Qualified Use Paths](identifier-casing-qualified-use-paths.md).
