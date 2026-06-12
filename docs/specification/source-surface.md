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
schemas, `UInt8`, `UInt16be`, `UInt24be`, `UInt31be`, `UInt32be`, and
`ReservedBits(width, value)` are accepted as schema primitives.
`ReservedBits` arguments must be literal
non-negative integers. The narrow closed tag-dispatch field type
`Dispatch(tag_field, tag => Primitive, ...)` is accepted when `tag_field`
names a previously decoded schema field and each case primitive is one of the
implemented exact-width unsigned binary primitives. These primitive names are
representation-local field vocabulary, not ordinary source types or values.
A schema may end with
structural `map to Target` clauses whose assignment lines use
`target_field = schema_field` to map schema-local fields into an ordinary
source value shape. Mapping clauses are parsed, formatted, lowered, exposed to
editor support, and used by the generated decode slice described in
[execution.md](execution.md) when the schema has a single structural mapping
and all decoded fields use the implemented exact-width unsigned primitive or
closed dispatch slice. The predicate, primitive, dispatch, and mapping text
are parsed and preserved as source-surface syntax. General schema decode,
encode, extension-tolerant dispatch, ADT constructor mapping, nested record
mapping, and multiple mapping selection are not implemented.
Schema declarations do not create ordinary value bindings or ordinary type
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
and the target schema must be `pub`. The import is not re-exported from the
importing module's qualified path. Imported private schemas report
`name.visibility` at the codec declaration. Missing schema targets report
`name.unresolved`; ordinary functions, source ADT types, and codec items at
the referenced path report `name.kind_mismatch` instead of being treated as
schemas. Importing or referencing a schema does not import schema-local field
names or create ordinary type bindings. Executable decode codec item calls are
provided by valid hand-written decode implementations and by `derive decode`
for the eligible generated binary schema decode-step slice, not by schema
references themselves.

A `decode with function_name` clause must resolve to an ordinary function in
the codec's module with exactly `ByteView` and `ByteOffset` parameters and a
`DecodeStep<T>` return type. Invalid decode signatures report
`codec.decode_signature` at the codec implementation clause, with related
context pointing to the referenced function when it is available. When the
referenced schema has one implemented structural mapping, the `T` value type
must match the mapping target record shape; mismatches report
`codec.decode_value_type` at the codec implementation clause. A codec with a
hand-written `decode with` clause is callable through the codec item name in
its declaring module, or through a written import-qualified module path when
the codec is `pub`. That call takes the same `ByteView` and `ByteOffset`
arguments and returns the referenced function's `DecodeStep<T>` unchanged.
`derive decode` codecs are callable through the same visibility and import
rules when their schema is eligible for `byte_decode_step_<schema>`, and the
call returns that generated helper's `DecodeStep<T>` result. Bare imported
codec names are not ordinary call targets.

An `encode with function_name` clause must resolve to an ordinary function in
the codec's module with an `EncodeStep<TState>` return type. When the
referenced schema has one implemented structural mapping, the function's first
parameter must match the mapping target record shape. Invalid encode
signatures report `codec.encode_signature`; mapped value parameter mismatches
report `codec.encode_value_type` at the codec implementation clause, with
related context pointing to the referenced function when it is available.
When the clause is valid, the codec item name is an ordinary call target in
the declaring module, or through a written import-qualified module path when
the codec is `pub`. The call uses the referenced function's parameters and
returns its `EncodeStep<TState>` value unchanged. General codec-generated
decode functions and derived encode execution are not implemented. Generated
`byte_decode_<schema>` helpers for the eligible binary schema slice, their
`byte_decode_step_<schema>` incremental decode-step counterparts, and derived
decode codec calls over that decode-step slice are covered by
[execution.md](execution.md).

## Expressions

See [source-surface-full.md#expressions](source-surface-full.md#expressions).

## Contract Predicates

See
[source-surface-full.md#contract-predicates](source-surface-full.md#contract-predicates).
