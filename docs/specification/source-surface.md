# Source Surface

This is the routing page for implemented source syntax. Use it to choose the
smallest section to read before opening the full grammar notes.

## Read First

- Source path derived local module identity, local and external package
  imports, functions, tests, source ADT type declarations, schema and codec
  declarations, public member aliases, canonical
  `#` comments, `##` documentation comments, doctests, ADR-lite metadata, and
  manifest dependency metadata plus `[lib].exports` source-file exports:
  [source-surface-full.md](source-surface-full.md).
  Use [commands.md](commands.md) for formatter layout and canonical comment
  spelling behavior.
- Doctest fence metadata, `runtime=contract`, `runtime=ensure`, and
  `runtime=result` expectations, expected-output fences, `> ` hidden setup,
  visible hash comments inside doctests, and negative examples:
  [source-surface-full.md#documentation-comments-and-doctests](source-surface-full.md#documentation-comments-and-doctests).
- Expression forms, constructors, records, dictionaries, vecs, matches,
  pipelines, and method-call diagnostics:
  [source-surface-full.md](source-surface-full.md#expressions).
- Contract predicate grammar:
  [source-surface-full.md](source-surface-full.md#contract-predicates).

## Read When

- Updating parser behavior, AST source shape, source metadata, or declaration
  rules.
- Checking whether a syntax feature is implemented rather than proposed.
- Aligning examples, diagnostics, or command behavior with accepted source
  syntax.

## Skip Unless Needed

- Do not read proposal or phase history before this page and the relevant
  section of [source-surface-full.md](source-surface-full.md).
- Use [source-decisions.md](source-decisions.md) only when rationale is needed
  after the implemented source behavior is clear.

## Grammar

See [source-surface-full.md#grammar](source-surface-full.md#grammar).

Top-level `schema Name` and `pub schema Name` declarations are implemented as
source module items. The implemented schema body slice requires a single
`format binary` clause before schema fields. Schema field lines contain a field
name, `:`, type text, and an optional field-local `where` predicate. In binary
schemas, `UInt8`, `UInt24be`, `UInt31be`, and `ReservedBits(width, value)` are
accepted as schema primitives. `ReservedBits` arguments must be literal
non-negative integers. These primitive names are representation-local field
vocabulary, not ordinary source types or values. The predicate and primitive
text are parsed and preserved as source-surface syntax. The implemented
execution slice for these primitive names is limited to the frame-header
decode behavior routed from [execution.md](execution.md). General schema
decode, encode, dispatch, and value mapping are not implemented. Schema
declarations do not create ordinary value bindings or ordinary type
declarations.

Top-level `codec Name for SchemaName ...` and
`pub codec Name for SchemaName ...` declarations are implemented as source
module items. A codec head lists one or both explicit directions, `decode` and
`encode`. The body contains one implementation clause for each listed
direction: `derive decode`, `derive encode`, `decode with function_name`, or
`encode with function_name`. The parser reports declaration-shape errors for
empty, unknown, or duplicate directions, missing clauses, clauses for unlisted
directions, and duplicate clauses. The source model preserves codec visibility,
schema ownership, directions, and body clauses for metadata, formatting,
editor support, and checker boundaries.

A codec schema reference resolves through schema-aware name lookup. Bare
`codec Name for SchemaName` references are limited to schemas declared in the
codec's own module. Qualified `codec Name for imported::SchemaName` references
require a matching written `use imported` path or alias in the codec's module,
and the target schema must be `pub`. Imported private schemas report
`name.visibility` at the codec declaration. Missing schema targets report
`name.unresolved`; ordinary functions, source ADT types, and codec items at
the referenced path report `name.kind_mismatch` instead of being treated as
schemas. Importing or referencing a schema does not import schema-local field
names, create ordinary type bindings, or expose executable codec APIs.

A `decode with function_name` clause must resolve to an ordinary function in
the codec's module with exactly `ByteView` and `ByteOffset` parameters and a
`DecodeStep<T>` return type. Invalid decode signatures report
`codec.decode_signature` at the codec implementation clause, with related
context pointing to the referenced function when it is available. Codec
execution, generated decode or encode functions, `derive decode`, generated or
checked encode function signatures, and schema value mapping are not
implemented.

## Expressions

See [source-surface-full.md#expressions](source-surface-full.md#expressions).

## Contract Predicates

See
[source-surface-full.md#contract-predicates](source-surface-full.md#contract-predicates).
