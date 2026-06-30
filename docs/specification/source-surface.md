# Source Surface

This page routes implemented source syntax. Open
[source-surface-full.md](source-surface-full.md) only when a short route here
is not enough.

## Read First

- Source path derived local module identity, local and external package
  imports, functions, tests, source ADT type declarations, schema
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
canonical repeated fields `[Payload; count]`, `ByteView(length)`, closed
dispatch, and extension dispatch forms documented in
[source-surface-full.md](source-surface-full.md) and checked by
`docs/specification/source-surface-executable.pl`. Canonical repeated fields
write the payload field type before `;` and the count expression after it; the
count expression may name an earlier visible count field or use implemented
arithmetic forms over earlier count fields, and the payload may use an
exact-width primitive, a lowercase exact-width primitive, a nested binary
schema, or `ByteView(length)`.
Closed and extension dispatch payload cases accept the same lowercase
exact-width `uint...` and `flag...` spelling wherever the compatible
upper-case exact-width primitive payload spelling is accepted. Byte-aligned
lowercase `uint... reserves <value>` payloads are accepted wherever direct
reserved-bit dispatch payloads are supported. Binary-only primitive vocabulary
remains gated by `format binary`. In `format binary` schemas, `veln fmt`
writes supported compatibility primitive spellings as the canonical lowercase
schema vocabulary, including direct fields, supported reserved fields,
repeated fields, and dispatch payload field text.

Schema declarations return and accept schema-local visible record shapes
through explicit schema operation expressions. The expression
`decode SchemaName from view at base_offset`
accepts eligible binary schemas, `ByteView`, and `ByteOffset` operands and
returns a `DecodeStep<T>` for the schema-local visible record shape. The
expression `encode SchemaName from value` accepts the schema-local visible
record shape for eligible binary schemas and returns
`Result<ByteChunk, EncodeError>`. Qualified public schema paths are accepted
when the imported schema is visible. The
executable coverage is
`examples/specification/run/schema-decode-expression/` and
`examples/specification/run/schema-encode-expression/`; encode expression
diagnostics are checked by
`examples/specification/check/schema-encode-expression-diagnostics/` and
runtime value failures by
`examples/specification/run/schema-encode-expression-unrepresentable-human/`
and
`examples/specification/run/schema-encode-expression-unrepresentable-json/`.
Projection into domain records is ordinary source code at the caller or
schema-operation boundary. Schema-level `map to` clauses, selected
schema mappings, mapping assignments, and `inverse` projection annotations are
not accepted source syntax.

## Codecs

Top-level `codec Name for Schema directions...` and
`pub codec Name for Schema directions...` declarations are not accepted source
syntax. The parser reports `parse.codec_declaration_removed` at the `codec`
token and directs source toward ordinary functions plus explicit
`decode Schema from view at base_offset` and `encode Schema from value`
expressions.

## Diagnostics

The parser rejects schema-level `map to` with
`parse.schema_mapping_removed` at the `map` token. Mapping-only semantic and
runtime diagnostics are not current behavior. The parser rejects top-level
codec declarations with `parse.codec_declaration_removed`. Current schema diagnostics cover
format placement, field references, primitive kind checks, field-local and
schema-level validation predicates, dispatch payload eligibility, explicit
schema decode expression schema-path resolution, and helper availability.

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
