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
schemas, `UInt1` through `UInt8`, `UInt16be`, `UInt16le`, `UInt24be`,
`UInt24le`, `UInt31be`, `UInt32be`, `UInt32le`, `UInt64be`, `UInt64le`, and
`ReservedBits(width, value)` are accepted as schema primitives. `Flag8` is
accepted as an opt-in one-byte visible flag bitset field; it decodes and
encodes through the source-visible `Flag8(bits: Int)` value type instead of
the raw `Int` used by `UInt8`.
`ReservedBits` arguments must be literal
non-negative integers. `Repeat(count_field, Payload)` is accepted as a
bounded repeated field when `count_field` names a previously decoded visible
`Int` field in the same binary schema. `Repeat(left_count - right_count,
Payload)` is accepted when both operands name earlier visible `Int` fields in
the same binary schema. `Payload` is either an implemented byte-aligned
exact-width unsigned primitive, an eligible nested binary schema payload, or
`ByteView(length_field)` when `length_field` is another earlier visible `Int`
field in the same schema. A repeated primitive field decodes and encodes as
`List<Int>`; a repeated nested schema field decodes and encodes as a list of
the nested schema's decoded record shape; and a repeated `ByteView` field
decodes as `List<ByteView>`. The narrow closed tag-dispatch field type
`Dispatch(tag_field, tag => Payload, ...)` is accepted when `tag_field` names a
previously decoded schema field and each case payload is either one of the
implemented exact-width unsigned binary primitives, a same-module binary
schema item, or a public imported binary schema named through a written `use`
path. The extension-tolerant field type
`ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` is accepted
when both referenced fields were decoded earlier in the same schema. Its known
cases use the same payload vocabulary, and its unknown cases preserve a
bounded raw payload selected by `length_field`. These primitive names are
representation-local field vocabulary, not ordinary source types or values.
One schema-level `validate` predicate may appear after binary schema fields.
It uses the same predicate syntax as field-local `where` clauses, but may
reference only `Int` fields decoded by the same schema helper. Unknown field
names, non-`Int` decoded fields, ordinary source bindings, and additional
schema-level validations are rejected through schema validation diagnostics.
Dispatch reference diagnostics report `schema.dispatch_reference` when the tag
or length field is missing, forward, or not an `Int`-decoded schema field.
Nested dispatch payload diagnostics report `schema.dispatch_payload` when a
payload name is missing, resolves to a non-schema item, names a private
imported schema, names a non-binary schema, refers forward or recursively, or
decodes to an incompatible payload shape. The checked diagnostics case is
`../../examples/specification/check/binary-schema-dispatch-payload-diagnostics/`.
A schema may end with
structural `map to Target` clauses whose assignment lines use
`target_field = expression` to map schema-local fields into an ordinary
source value shape. The implemented mapping expression slice supports
schema-local field references, record construction, ADT constructor
construction resolved through ordinary source module rules, one pure
same-module converter function call, or one imported public pure converter
function call through a written `use` path or alias. Converter calls take one
schema-local field argument and assign the returned value to the target field.
Other ordinary calls, bare imported converter names, private imported
converters, effects, runtime settings, stream state, and recovery behavior are
not mapping expressions.
Mapping clauses are parsed, formatted, lowered, exposed to editor support,
and used by the generated decode slice described in
[execution.md](execution.md) when the schema has one structural mapping, or
multiple structural mappings selected by `when field == literal`, and all
assignment expressions use implemented decoded field types:
exact-width unsigned primitive fields as `Int`, `Flag8` fields as `Flag8`,
length-bounded
`ByteView(length_field)` or `ByteView(left_length - right_length)` payload
fields as `ByteView`, bounded
`Repeat(count_field, Payload)` fields as lists of their payload value shape,
including `List<ByteView>` for `Repeat(count_field, ByteView(length_field))`,
closed nested dispatch payload fields as the nested schema record shape, and
extension dispatch payload fields as `SchemaDispatchPayload<T>`. Multiple selected mappings must
all use the same decoded `Int` selector field, distinct selector literal
values, and the same decoded record shape. Missing, duplicate, and unsupported
selectors report `schema.mapping_selection_required`,
`schema.mapping_selection_ambiguous`, `schema.mapping_selection`, or
`schema.mapping_selection_unsupported`. The predicate, primitive, dispatch,
and mapping text are parsed and preserved as source-surface syntax.
General schema decode, general schema encode beyond the exact-width
primitive, `Flag8`, supported reserved-bit, closed dispatch, extension
dispatch, bounded repeated primitive or nested schema field, length-bounded
`ByteView`, and same-module or imported public nested dispatch payload helper slices, general
ADT constructor mapping beyond schema-local structural expressions,
recursive or otherwise ineligible dispatch payload schemas, arbitrary mapping
expressions, and mapping selection beyond decoded-field integer equality are
not implemented.
The checked diagnostics case
`../../examples/specification/check/schema-mapping-selection-diagnostics/`
pins the mapping selection boundary. The checked diagnostics case
`../../examples/specification/check/schema-mapping-expression-boundary-diagnostics/`
pins unsupported mapping expression, unresolved constructor, constructor
arity, and constructor payload type diagnostics. The checked diagnostics case
`../../examples/specification/check/schema-mapping-converter-diagnostics/`
pins unresolved converter, converter arity, converter input type, converter
return type through `schema.mapping_converter_return`, converter purity, and
unsupported converter shape diagnostics.
The checked diagnostics case
`../../examples/specification/check/schema-imported-mapping-converter-diagnostics/`
pins imported converter visibility and missing written import-path diagnostics.
Eligible binary schemas whose fields are visible exact-width unsigned
primitives, including standalone `UInt1` through `UInt7` fields that consume
one byte each, `Flag8` bitset fields, supported byte-aligned
`ReservedBits(width, value)` fields,
the supported `ReservedBits(1, 0)` before `UInt31be` layout, supported
packed prefix `ReservedBits(width, value)` plus `UIntN` layouts whose widths
sum to eight, sixteen, twenty-four, or thirty-two bits, supported `UIntN`
plus reserved suffix layouts whose widths sum to eight, sixteen,
twenty-four, or thirty-two bits,
bounded `Repeat(count_field, Payload)` fields whose count names an earlier
visible exact-width unsigned `Int` field, bounded
`Repeat(left_count - right_count, Payload)` fields whose operands both name
earlier visible exact-width unsigned `Int` fields, and whose payload is either
`UInt8`, `UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`,
`UInt32be`, `UInt32le`, `UInt64be`, `UInt64le`, or an eligible nested binary
schema payload,
length-bounded `ByteView(length_field)` payload fields whose length names an
earlier visible exact-width unsigned `Int` field, or
`ByteView(left_length - right_length)` payload fields whose operands both name
earlier visible exact-width unsigned `Int` fields,
closed `Dispatch(tag_field, tag => Payload, ...)` fields, and
extension-tolerant `ExtensionDispatch(tag_field, length_field, tag => Payload,
...)` fields whose tag and length names are earlier visible exact-width fields
and whose cases are exact-width unsigned primitive payloads or earlier
same-module binary schema payloads or public imported binary schema payloads
named through written `use` paths, also expose generated
`byte_encode_<schema>` helpers described in [execution.md](execution.md);
one direct structural `map to Target` clause can make that helper accept the
mapping target record shape when every visible encode field, including
`Flag8` fields, is assigned from a schema-local field reference, or when the
only visible encode field is a `Flag8` field wrapped by a direct
single-constructor ADT call. Multiple selected mapping clauses, mapping
expressions that cannot be projected back to schema-local fields, encode-time
field-local validation beyond primitive
representation ranges, recursive or otherwise ineligible dispatch payload
schemas, non-byte-aligned reserved fields outside the supported packed and
`UInt31be` shared-bit layouts, and derived codec encode execution over
unsupported schemas are outside that encode helper slice.
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
`codec Name for SchemaName` references are limited to schemas and schema
aliases declared in the codec's own module. Qualified
`codec Name for imported::SchemaName` references require a matching written
`use imported` path or alias in the codec's module, and the target schema or
schema alias must be `pub`. A `pub schema Public = imported::Packet` member
alias publishes an existing public schema through the declaring module's
public path without creating a wrapper schema, schema identity, generated
codec alias, ordinary type binding, generated helper binding, or schema-local
field binding. Schema alias targets use schema-aware lookup. Missing, private,
function, source ADT type, and codec targets are rejected at the alias
declaration. Imported private schemas report `name.visibility` at the codec
declaration. Missing schema targets report `name.unresolved`; ordinary
functions, source ADT types, and codec items at the referenced path report
`name.kind_mismatch` instead of being treated as schemas. Importing or
referencing a schema does not import schema-local field names or create
ordinary type bindings. Executable decode codec item calls are provided by
valid hand-written decode implementations and by `derive decode` for the
eligible generated binary schema decode-step slice, not by schema references
themselves.

A `decode with function_name` clause must resolve to an ordinary function in
the codec's module with exactly `ByteView` and `ByteOffset` parameters and a
`DecodeStep<T>` return type. Invalid decode signatures report
`codec.decode_signature` at the codec implementation clause, with related
context pointing to the referenced function when it is available. When the
referenced schema has an implemented structural mapping slice, the `T` value
type must match the mapping target record shape, including selected mappings
that all resolve to that same record shape; mismatches report
`codec.decode_value_type` at the codec implementation clause. A codec with a
hand-written `decode with` clause is callable through the codec item name in
its declaring module, or through a written import-qualified module path when
the codec is `pub`. That call takes the same `ByteView` and `ByteOffset`
arguments as the referenced function. It returns valid `Decoded`,
`NeedMore`, and `Invalid` results unchanged, and projects an oversized
consumed count to `codec.consumed_count_invalid` as specified in
`execution.md`.
`derive decode` codecs are callable through the same visibility and import
rules when their schema is eligible for `byte_decode_step_<schema>`, and the
call returns that generated helper's `DecodeStep<T>` result. For the
implemented structural mapping slice, `T` is the mapping target record
shape when each assignment source has the same implemented decoded field type
as the target field and all selected mappings resolve to that same record
shape. Bare imported codec names are not ordinary call targets.

An `encode with function_name` clause must resolve to an ordinary function in
the codec's module with an `EncodeStep<TState>` return type. When the
referenced schema has an implemented structural mapping slice, the function's first
parameter must match the mapping target record shape. Invalid encode
signatures report `codec.encode_signature`; mapped value parameter mismatches
report `codec.encode_value_type` at the codec implementation clause, with
related context pointing to the referenced function when it is available.
When the clause is valid, the codec item name is an ordinary call target in
the declaring module, or through a written import-qualified module path when
the codec is `pub`. The call uses the referenced function's parameters and
returns its `EncodeStep<TState>` value unchanged. General codec-generated
decode functions are not implemented. Generated
`byte_decode_<schema>` helpers for the eligible binary schema slice, generated
`byte_encode_<schema>` helpers for the exact-width including standalone
`UInt1` through `UInt7`, supported reserved-bit, closed dispatch, extension
dispatch, length-bounded `ByteView`, and same-module nested dispatch payload
encode slices,
their `byte_decode_step_<schema>` incremental decode-step counterparts,
derived decode codec calls over that decode-step slice, and derived encode
codec calls over that encode helper slice are covered by
[execution.md](execution.md).
When a mapped schema cannot expose the mapping target through a generated
encode boundary, the `derive encode` clause reports `codec.encode_value_type`.
The implemented direct structural mapping slice exposes that target record as
the generated encode boundary.

Documentation comments may reference schemas with `{@schema Name}` or
`{@schema module::Name}`. These references use schema-aware lookup, not value
or type lookup. Bare references resolve schemas and schema aliases in the same
module. Qualified references require a written `use` path and a public schema
or public schema alias. Missing, private, function, source ADT type, and codec
targets are rejected at the documentation reference span. Schema references in
documentation do not expose schema-local field names, generated helper names,
codec names, or ordinary source type bindings.

## Expressions

See [source-surface-full.md#expressions](source-surface-full.md#expressions).

## Contract Predicates

See
[source-surface-full.md#contract-predicates](source-surface-full.md#contract-predicates).
