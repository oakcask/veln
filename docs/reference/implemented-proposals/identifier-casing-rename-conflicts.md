---
role: implementation-record
authority: supporting
update-when: Identifier casing rename-conflict evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Rename Conflicts

## Outcome

LSP rename now rejects predictable same-namespace duplicate, same-clause
handler operation parameter, visible type-alias, and ambiguity conflicts for
selected valid workspace type, constructor, function, and value-binding symbols
before it returns edits.

Current behavior is specified by
[Editor Support](../../specification/editor-support.md). The checked
`identifier-casing-rename-boundary` LSP example fixes the JSON-RPC invalid
params response, shared `rename.conflict` code, conflict detail projection,
same-clause parameter conflict boundary, unedited imported type ambiguity
boundary, unedited imported function ambiguity boundaries for calls and
function values, handler parameter capture for function rename, and edit-free
failure boundary. Focused language-service tests also cover handler context
and clause parameter shadowing boundaries, constructor ambiguity rejection, and
qualified-function identity preservation that do not need another
transport-specific fixture.

## Scope

The shared language service validates the replacement name against the selected
symbol's current identifier class, then checks the current project snapshot for
conflicts when the requested spelling differs from the selected spelling. A
class-changing replacement still fails with `rename.invalid_case`.

A conflict failure reports shared code `rename.conflict`, the selected symbol
class, the requested name, the conflicting declaration location, and the
affected scope. LSP maps that failure to JSON-RPC invalid params with code
`-32602` and returns no workspace edit. Module-scope failures report
`kind: "module"` and identify the affected module. Type rename checks the
current type namespace for type
declarations and visible type aliases in modules where the renamed type would
be visible after the complete edit, including requested-name type-role
occurrences that were not references to the selected type before the rename.
Function rename checks bare call targets and bare function-value occurrences in
modules where the renamed function would be visible after the complete edit,
including requested-name occurrences that would become ambiguous between
imported functions. Handler context parameters and operation-clause parameters
can also capture edited bare function calls or function-value references inside
their lexical scope; those failures report the handler parameter declaration as
the conflicting declaration.
Constructor rename checks constructor declarations in the selected ADT and bare
constructor expression and pattern uses in modules where the renamed constructor
would be visible after the complete edit, including requested-name constructor
uses that would become ambiguous between imported constructors.
Lexical-scope failures report `kind: "lexical"` and identify the affected file
and source start and end offsets. Local binding conflicts report the binding
declaration as the conflicting declaration, including when a function rename
would collide with an edited reference scope. Handler operation clause
parameter conflicts report the existing clause parameter as the conflicting
declaration, even when the selected parameter has no references. A clause
parameter can reuse an enclosing handler context parameter name when the
edited declaration and references remain bound to the clause parameter. An
enclosing context parameter can reuse a clause parameter name only when edited
references stay outside that clause parameter scope.

The conflict check is limited to the retained current project snapshot. It
does not claim to validate unloaded consumers, future file operations, or
transport surfaces other than LSP.

## Completion

This slice is complete for valid selected workspace symbols through LSP
rename. It does not complete recovery navigation through quarantined invalid
declarations, repair rename, source-path module rename exclusion, MCP rename
mapping, or deferred module surfaces.
