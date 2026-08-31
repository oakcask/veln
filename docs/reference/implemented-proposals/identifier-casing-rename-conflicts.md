---
role: implementation-record
authority: supporting
update-when: Identifier casing rename-conflict evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Rename Conflicts

## Outcome

LSP rename now rejects predictable same-namespace duplicate and ambiguity
conflicts for selected valid workspace type, constructor, function, and
value-binding symbols before it returns edits.

Current behavior is specified by
[Editor Support](../../specification/editor-support.md). The checked
`identifier-casing-rename-boundary` LSP example fixes the JSON-RPC invalid
params response, shared `rename.conflict` code, conflict detail projection,
and edit-free failure boundary.

## Scope

The shared language service validates the replacement name against the selected
symbol's current identifier class, then checks the current project snapshot for
conflicts when the requested spelling differs from the selected spelling. A
class-changing replacement still fails with `rename.invalid_case`.

A conflict failure reports shared code `rename.conflict`, the selected symbol
class, the requested name, the conflicting declaration location, and the
affected scope. LSP maps that failure to JSON-RPC invalid params with code
`-32602` and returns no workspace edit. Module-scope failures identify the
affected module. Lexical-scope failures identify the affected file and source
offset range.

The conflict check is limited to the retained current project snapshot. It
does not claim to validate unloaded consumers, future file operations, or
transport surfaces other than LSP.

## Completion

This slice is complete for valid selected workspace symbols through LSP
rename. It does not complete recovery navigation through quarantined invalid
declarations, repair rename, source-path module rename exclusion, MCP rename
mapping, or deferred module surfaces.
