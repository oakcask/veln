# Source Surface Details

Status: routing

Use [source-surface.md](source-surface.md) first. The executable grammar in
[source-surface-executable.pl](source-surface-executable.pl) and checked cases
under `../../examples/specification/` are the primary source-surface evidence.

## Current Schema Boundary

Schemas contain fields, an optional `format binary` clause, field-local
`where` predicates, and at most one schema-level `validate` predicate.
Generated helpers use schema-local visible records. Domain projection is
ordinary source code outside the schema body.

Schema-level mapping clauses are rejected by the parser and are not part of
the implemented grammar.

## Read When

- Use this page only as a stable route for old links.
- Prefer the executable grammar and focused checked examples for current
  syntax details.
