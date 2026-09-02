---
role: implementation-record
authority: supporting
update-when: Identifier casing namespace-by-use-role evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Namespace Use Roles

## Outcome

Source analysis and editor-neutral definition selection now keep equal
spellings separated by the namespace fixed by each source position.

Current behavior is specified by
[Name Resolution](../../specification/name-resolution.md) and
[Editor Support](../../specification/editor-support.md). The checked
`identifier-casing-namespace-use-roles` example covers equal-spelled schemas,
effects, handlers, operations, types, constructors, functions, and value
bindings where the source grammar permits them. It includes lower-case exact
spelling collisions between casing-neutral declarations and accepted function
and local-binding names. It also covers same-namespace duplicates, ordinary
calls that exclude casing-neutral declarations, and schema composition
ambiguity when both a type and a schema are visible. Focused
language-service tests cover definition selection for accepted schema, effect,
handler, operation, type, constructor, function, and value-binding
occurrences.

## Scope

This slice preserves the existing namespaces. It does not add new namespaces,
change visibility, or change duplicate rules inside one namespace.

Type annotations select the type namespace before schema-reference diagnostics
consider same-spelled schema declarations. Ordinary value calls continue to use
the value and call-target namespace and do not select schema, effect, handler,
or effect-operation declarations. Schema composition still admits both ordinary
type and schema candidates and reports its existing ambiguity when both are
visible.

## Completion

This slice completes only the namespace-by-use-role acceptance row. The
remaining identifier-casing proposal still owns module identities, source-span
range coverage, remaining recovery boundaries, MCP rename mappings, deferred
language-service consumers, and repository-wide source-carrier audit work.
