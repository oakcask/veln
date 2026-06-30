# Source Surface

This page routes implemented source syntax. Open
[source-surface-full.md](source-surface-full.md) only when a short route here
is not enough.

## Read First

- Source path derived local module identity, local and external package
  imports, functions, tests, source ADT type declarations, schema and codec
  declarations, public member aliases, canonical `#` comments, `##`
  documentation comments, doctests, ADR-lite metadata, and manifest dependency
  metadata plus `[lib].exports` source-file exports:
  [source-surface-full.md](source-surface-full.md).
- Expression forms, constructors, records, dictionaries, vecs, matches,
  `if` / `else if` / `else` expressions, pipelines, ordinary and variadic
  calls, standard channel calls, zero-argument task spawns, one-context
  `task::spawn_with` calls, and method-call diagnostics:
  [source-surface-full.md](source-surface-full.md).
- Contract predicate grammar:
  [source-surface-full.md](source-surface-full.md).
- Formatter layout and canonical comment spelling:
  [commands.md](commands.md).

## Schemas

Top-level `schema Name` and `pub schema Name` declarations are source module
items. A schema body may omit its `format` clause when every field uses
format-neutral type text. When present, the single `format binary` clause must
appear before schema fields.

Schema field lines contain a field name, `:`, type text, and an optional
field-local `where` predicate. One schema-level `validate` predicate may
appear after fields. Binary schema field vocabulary includes the implemented
exact-width unsigned primitives, lowercase `uint...` fields, visible flag
bitset primitives, lowercase `flag...` fields, `uint... reserves <value>`
reserved-bit fields, `ReservedBits(width, value)`, `Repeat(count, Payload)`,
`ByteView(length)`, closed dispatch, and extension dispatch forms documented in
[source-surface-full.md](source-surface-full.md) and checked
by `docs/specification/source-surface-executable.pl`.

Schema declarations return and accept schema-local visible record shapes
through generated helpers. Projection into domain records is ordinary source
code at the helper-call or codec boundary. Schema-level `map to` clauses,
selected schema mappings, mapping assignments, and `inverse` projection
annotations are not accepted source syntax.

## Codecs

Top-level `codec Name for Schema directions...` declarations remain source
module items. `derive decode`, `derive encode`, `decode with function_name`,
and `encode with function_name` clauses use the schema-local helper boundary.
When a public codec type differs from the schema-local record shape, ordinary
source functions perform the projection.

## Diagnostics

The parser rejects schema-level `map to` with
`parse.schema_mapping_removed` at the `map` token. Mapping-only semantic and
runtime diagnostics are not current behavior. Current schema diagnostics cover
format placement, field references, primitive kind checks, field-local and
schema-level validation predicates, dispatch payload eligibility, codec schema
references, and helper availability.

## Read When

- Updating parser behavior, AST source shape, source metadata, declaration
  rules, or formatter output.
- Checking whether a syntax feature is implemented rather than proposed.
- Aligning examples, diagnostics, or command behavior with accepted source
  syntax.

## Skip Unless Needed

- Do not read proposal or phase history before this page and the relevant
  section of [source-surface-full.md](source-surface-full.md).
- Use [source-decisions.md](source-decisions.md) only when rationale is needed
  after the implemented source behavior is clear.
