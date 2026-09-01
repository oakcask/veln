---
role: implementation-record
authority: supporting
update-when: Identifier casing namespace-by-use-role evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Namespace Roles

## Outcome

Equal-spelled schemas, effects, handlers, operations, types, constructors,
functions, and value bindings now preserve the namespace selected by the
source role. Current behavior is specified by
[Name Resolution](../../specification/name-resolution.md) and
[Editor Support](../../specification/editor-support.md). The checked
`identifier-casing-namespace-roles`,
`identifier-casing-namespace-role-visibility-json`,
`identifier-casing-namespace-role-type-alias-json`, and
`identifier-casing-namespace-role-controls-json` check examples and the LSP
example fix the executable compiler acceptance, project visibility, type alias,
duplicate, ordinary-call exclusion, schema-composition ambiguity, and
definition boundaries.

## Scope

Dedicated type, constructor, schema, effect, handler, and effect-operation
positions select their existing namespace. Schema, effect, handler, and
effect-operation names remain casing-neutral and do not become ordinary
constructor, value, or call candidates. Ordinary calls report the ordinary
unresolved fact when only casing-neutral declarations share the spelling.
Lowercase function and value-binding positions retain the existing
value-shadowing rule, so a visible local binding wins over a same-spelled
function at a call position.

Same-namespace duplicate controls now cover source types, constructors,
functions, schemas, effects, handlers, and effect operations. Cross-namespace
equal spellings remain accepted. Schema-composition positions retain the
existing type-versus-schema ambiguity when both namespaces provide the same
visible spelling. Ordinary type positions use the visible type namespace, so a
selected but unimported same-spelled type declaration cannot suppress a
same-module schema-as-type diagnostic, and concrete types and public type
aliases share the same visible lookup boundary.

Language-service definition evidence covers accepted type, constructor,
function, and value-binding occurrences beside equal-spelled neutral
declarations. It also covers neutral declaration tokens and ordinary calls
that do not navigate to schemas, effects, handlers, or effect operations. The
language service still exposes only its supported symbol classes; neutral
declaration names are not introduced as new navigation symbols by this slice.

## Completion

This slice is complete for the namespace-by-use-role acceptance row. It does
not complete module identities, remaining recovery consumers, MCP rename
mapping, or source migration beyond the focused executable examples.
