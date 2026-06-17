# Schema Declaration Surface

Status: proposed

This proposal tracks remaining source syntax needed to declare schemas as
external representation boundaries. The first top-level declaration slice is
implemented as current behavior under `../specification/source-surface.md` and
checked examples under `../../examples/specification/`.

## Problem

Current source syntax has a first top-level `schema` declaration slice with a
single `format binary` clause, field declarations, field-local `where`
predicate syntax, structural `map to Target` mapping clauses, and a narrow
executable field-local validation and mapped-record helper slice. It does not
yet have runtime mapping beyond the implemented structural expression slices,
complete binary primitive semantics, or executable codec bindings outside the
implemented helper boundaries.

The HTTP/2 design driver needs a declaration that can say:

- a field is read from bytes rather than from an internal Veln value
- a field has a fixed external width
- a field is validated at the schema boundary
- a field may map into an independently declared Veln type
- a schema reports structural failures with field paths and byte positions

Without a schema declaration, binary protocol examples must encode external
layout in ordinary functions, which hides the boundary the driver is meant to
exercise.

## Scope

The implemented first slice covers:

- top-level `schema` declarations
- named schema fields
- field type annotations that may name `UInt1` through `UInt8`, `UInt16be`,
  `UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`, `UInt32be`, `UInt32le`,
  `UInt64be`, `UInt64le`, and `ReservedBits(width, value)` as binary schema
  primitives
- source-surface `ReservedBits(width, value)` declaration checking for literal
  integer arguments in `format binary` schemas
- schema visibility and module ownership rules for `schema` and `pub schema`
- schema references from codec declaration heads, including same-module bare
  references and imported public schema references through written `use` paths
- top-level public schema member aliases that re-export existing public
  schemas through the declaring module's public path and resolve through
  schema-aware lookup
- structural schema value mapping clauses with explicit `map to Target`
  headers and `target_field = schema_field` assignments preserved by the
  parser, formatter, lowered AST, and editor token metadata
- executable field-local validation helper slices that decode binary schema
  fields in declaration order and evaluate supported `where` predicates after
  the owning field is decoded
- generated `validate_<schema>` helper bindings for eligible source
  `format binary` schemas that validate a supplied schema-local decoded record
  with the same supported field-local `where` predicate language used by
  generated binary decode helpers
- generated `byte_decode_<schema>` helper bindings for source `format binary`
schemas whose fields use implemented exact-width unsigned primitives,
  length-bounded `ByteView(length_field)` or
  `ByteView(left_length - right_length)` payload fields, or the implemented
  dispatch payload slices
- generated runtime mapping for one structural `map to Target` clause, or
  multiple clauses selected by `when field == literal` or
  `when field != literal`, when each assignment expression uses the
  implemented structural expression slice and type checks against the target
  record field
- generated `byte_encode_<schema>` helper and `derive encode` support for one
  structural `map to Target` clause whose assignments project the visible
  encode fields through direct field references, record-shaped direct field
  projections, field selection from those record-shaped projections, and the
  implemented direct ADT constructor wrapper forms, plus multiple selected
  structural mapping clauses when all selected mappings resolve to one target
  record shape and every schema-local encode field projects back from that
  selected target record through direct source-field assignments
- generated encode-time field-local validation for eligible
  `byte_encode_<schema>` helpers, using the supported schema predicate
  language over the current visible `Int` field and earlier visible `Int`
  fields after primitive, fixed-field, length, repeat, and dispatch
  representability checks have succeeded
- codec decode boundaries over schemas with multiple decoded-field selected
  mappings when all selected mappings resolve to one record shape already
  accepted by the generated decode-step helper
- semantic rejection for ambiguous or unsupported mapping selection, including
  missing selectors, duplicate or overlapping selector clauses, selector field
  mismatches, and selected target shape mismatches
- schema mapping expressions that reference schema-local fields, construct
  records, construct ADT payloads through ordinary source module constructor
  resolution, call one pure same-module representation converter, or call one
  imported public pure converter through a written `use` path or alias from a
  schema-local field or structural mapping expression into a target field, or
  select a visible field from a record-shaped structural mapping expression,
  plus `+`, `-`, and `*` integer arithmetic over decoded schema-local `Int`
  fields and nested supported mapping arithmetic expressions into an `Int` target
  field
- parser, AST, formatter, editor token, and documentation behavior for the
  implemented source surface, including documentation comments that reference
  schemas through schema-aware lookup

This proposal remains open for:

- generated runtime decode bindings for binary schema fields outside the
  implemented exact-width unsigned primitive, length-bounded `ByteView`,
  closed dispatch, and extension dispatch slices
- runtime mapping beyond the implemented schema-local field reference, record
  construction, ADT constructor construction, single pure same-module or
  imported public representation conversion hook expression slice, field
  selection from record-shaped structural mapping expressions, decoded-field
  integer mapping arithmetic, and decoded-field integer equality or inequality
  mapping selection
- general binary primitive execution semantics beyond the implemented narrow
  primitive decode slices
- schema-aware references from later schema composition surfaces beyond codec
  declaration heads, public schema member aliases, documentation comments, and
  binary fixture metadata

## Discussion Result: Codec Binding Direction

Schema declarations should not name the codec declarations that decode or
encode them. A schema is the reusable external representation contract; it
remains meaningful when a module exposes a decoder, an encoder, both
directions, fixture-only helpers, or no executable codec yet.

The binding should point from a codec declaration to the schema it implements.
That keeps schema declarations free of executable API ownership and lets the
codec boundary decide direction, readiness variants, consumed byte counts,
offset handling, imports, and exported names. A schema may still import or
select field vocabularies for its representation format, but that selection is
not the same thing as naming executable codec entry points.

This means a schema can be imported and referenced as a boundary contract on
its own. Modules expose executable decoding or encoding by exporting codec
declarations that cite the schema, rather than by adding codec names inside the
schema body.

## Discussion Result: Schema Value Mapping

The structural mapping clause syntax is implemented as current behavior under
`../specification/source-surface.md`. The generated runtime mapping slices are
implemented under `../specification/execution.md`: an eligible binary schema
may use one `map to Target` clause, or multiple clauses selected by
`when field == literal` or `when field != literal`, to construct an ordinary
mapped record after field-local validation succeeds when each assignment
expression type checks against the target field. Selected mappings must use
one decoded `Int` selector field, non-overlapping selector clauses, and one
decoded record shape. The
implemented expression slice supports schema-local field references, record
construction, ADT constructor construction resolved through ordinary source
module rules, one pure same-module converter function call, and one imported
public pure converter function call through a written `use` path or alias from
a schema-local field or structural mapping expression into a target field, and
field selection from record-shaped structural mapping expressions. An `Int`
target field may also use `+`, `-`, and `*` over decoded schema-local `Int`
fields and nested supported mapping arithmetic expressions.

The implemented runtime mapping slice maps schema field values through
schema-local field references, record construction, ADT constructor
construction, a single same-module pure converter call, and a single imported
public pure converter call through a written `use` path or alias, and field
selection from record-shaped structural mapping expressions, plus
decoded-field integer mapping arithmetic for `Int` target fields. Converter
arguments may be schema-local field references or structural mapping
expressions made from schema-local fields, records, ADT constructors, and
nested combinations of those forms, including supported integer arithmetic
mapping expressions. A schema does not implicitly publish a record type just
because it names fields, and importing a schema does not make its schema-local
field names available as ordinary source bindings.

The runtime checker should resolve the target value shape and assign
schema-local fields to the target's record fields or ADT constructor payload
fields. Fields marked representation-only by the selected schema vocabulary are
omitted from the produced value unless the mapping explicitly includes them.
This keeps reserved bits and other representation-only facts available for
validation and diagnostics without turning them into protocol-domain data by
accident.

Mapping is checked after schema field validation and before the decoded value
is returned by a codec. The checker should resolve target record fields and ADT
constructors through the normal source module rules, reject missing or
duplicate assignments, and require each assigned schema value to type check
against the target field. Schema primitive values may use the primitive's
declared ordinary value type, such as `Int` for exact-width unsigned fields, or
a field-local representation conversion when the schema vocabulary defines one.

Mapping expressions stay structural in the first surface: field selection,
record construction, ADT construction, one pure same-module converter call,
and one imported public pure converter call through a written `use` path or
alias, plus decoded-field integer `+`, `-`, and `*` mapping arithmetic, are
implemented. Arbitrary function calls, bare imported converter
names, private imported converters, runtime settings, stream state, and
recovery behavior belong in explicit codec functions rather than in schema
mapping.

## Discussion Result: Top-Level Schema Declarations

`schema` should be a normal top-level declaration beside declarations such as
`type`, `fn`, and `test`, not a modifier on a type declaration and not a
specialized `codec schema` form.

The declaration name owns an external representation boundary. Its body owns
schema-local fields, validation clauses, mapping clauses, and the
format-specific vocabulary selected by its `format` clause. It does not create
an ordinary Veln value type by itself, and it does not imply executable decode
or encode APIs. Codecs cite schemas from their own declarations when a module
wants to expose execution.

This keeps the parser, formatter, editor support, documentation, and module
item model direct: a source file contains a schema item with a stable name and
span. It also keeps schemas reusable across generated codecs, hand-written
codecs, fixtures, documentation, and diagnostic tests without forcing every
schema to commit to one executable direction.

## Implemented Slice: Codec Schema Imports And References

The codec declaration head slice is implemented as current behavior under
`../specification/source-surface.md`. Schema visibility follows the ordinary
source module boundary for codec schema references. A private schema is visible
only in its declaring module. A `pub schema` declaration is part of the
declaring module's public API when the module's source file is listed by the
package manifest's `[lib].exports`.

References to schemas use schema-aware name resolution rather than value
resolution. Codec declarations may reference a schema by bare name inside the
declaring module. From another module, codec declarations reference public
schemas through the written `use` module path, such as `http2::FrameHeader`. A
`use` declaration does not re-export the schema from the importing module.

Importing or referencing a schema imports only the schema item. It does not
import schema-local field names as ordinary bindings, expose a generated record
type, or make any decoder or encoder available. Executable APIs remain owned
by public codec declarations that explicitly cite the schema.

The implemented surface also accepts top-level public schema member aliases:

```text
pub schema PublicPacket = wire::Packet
```

Schema aliases resolve their targets through schema-aware lookup rather than
ordinary value or type lookup. A schema alias may publish a public schema from
an imported module through the declaring module's public path. Codec schema
references resolve through exported schema aliases wherever they resolve
public schemas through written module paths. Missing, private, function,
source ADT type, and codec targets are rejected at the alias declaration.
Schema aliases do not import schema-local field names, generated helper names,
codec names, or ordinary source type bindings, and they do not create wrapper
schemas, new schema identities, or generated codec aliases.

## Implemented Slice: Documentation Schema References

The documentation-comment schema reference slice is implemented as current
behavior under `../specification/source-surface.md` and
`../specification/commands.md`, with executable examples under
`../../examples/specification/doc/schema-references/` and
`../../examples/specification/doc/schema-reference-diagnostics/`.

Documentation comments may write `{@schema Name}` for same-module schema
references or `{@schema module::Name}` for imported public schemas and public
schema aliases reached through a written `use` path. These references use
schema-aware lookup rather than ordinary value or type lookup. Missing,
private, function, source ADT type, and codec targets are rejected at the
reference span. Documentation schema references do not make schema-local field
names, generated helper names, codec names, or ordinary source type bindings
visible.

## Implemented Slice: Binary Fixture Schema References

The binary fixture metadata schema reference slice is implemented as current
behavior under `../specification/source-surface.md` and
`../specification/execution.md`, with executable coverage under
`../../examples/specification/run/binary-fixture-invalid-field/`,
`../../examples/specification/run/binary-fixture-schema-references/`, and
`../../examples/specification/run/binary-fixture-schema-reference-diagnostics/`.

Executable specification cases may add `schema = "Name"` or
`schema = "module::Name"` to a `[[binary_fixture]]` record. The fixture
reference uses schema-aware lookup rather than ordinary value or type lookup.
Bare names resolve schemas and schema aliases in the fixture source module.
Qualified references require a written `use` path and a public schema or
public schema alias. Missing, private, function, source ADT type, codec, and
generated helper targets are rejected as invalid fixture schema references.
When fixture metadata also writes `field_path`, its first segment must name
the resolved schema. Fixture schema references do not make schema-local field
names, generated helper names, codec names, or ordinary source type bindings
visible.

## Discussion Result: Field Validation Semantics

The implemented source surface accepts field-local `where` clauses on schema
fields and preserves the predicate with the owning field:

```text
schema PaddedPayload
  format binary

  length: UInt24be
  padding_length: UInt8 where padding_length <= length
  payload: ByteView(length - padding_length)
end
```

The executable validation helper slice is implemented under
`../specification/execution.md`: schema helper definitions decode fields in
declaration order and evaluate supported `where` predicates after the owning
field is decoded. Source `format binary` schema declarations whose fields all
use implemented exact-width unsigned primitives expose generated
`byte_decode_<schema>` helpers without hand-written prelude entries. The
implemented examples include `SchemaValidationSample` and another schema that
checks an arithmetic boolean predicate. At decode time, a `where` clause is
checked after its field has been decoded and before later fields may reference
the validated value. The predicate may name the current field and fields
decoded earlier in the same schema. It must not name later fields, ordinary
source bindings, runtime settings, connection state, stream state, or imported
functions.

The predicate language should reuse the familiar comparison, boolean, literal,
field-reference, and arithmetic operators from contract predicates, but with a
schema-local resolver and without `require`, `ensure`, or `invariant`
keywords. This keeps representation checks readable without making schema
validation part of the ordinary function contract system.

A failed `where` clause is a schema structural failure at the owning field
path. The implemented generated byte-decode helper slice reports the field
path, byte offset, failed predicate, owning field value, decoded values
available to the predicate, and bounded byte preview through structured data
and related notes. The implemented `validate_<schema>` value helper slice
checks ordinary supplied decoded records at the schema boundary and reports the
same failed predicate, schema/field path, owning supplied field value, and
supplied decoded values through structured value diagnostics. Malformed
declaration syntax remains a source `check` diagnostic rather than a runtime
schema validation failure.
Protocol-state limits that depend on negotiated settings remain explicit
codec or protocol-core checks rather than schema `where` clauses.

## Discussion Result: Format Vocabulary Selection

Schema declarations should select their external representation vocabulary
inside the schema body with an explicit format clause, rather than by using a
specialized declaration keyword such as `codec schema`.

The first clause should be `format binary`. It makes binary schema primitives
such as exact-width integers, reserved bits, byte ranges, and dispatch forms
available only in that schema's field vocabulary. The clause does not import
ordinary source values, does not create executable codec APIs, and does not
change the module visibility of the schema item.

```text
schema Http2FrameHeader
  format binary

  length: UInt24be
  kind: UInt8 as FrameKind
  flags: UInt8
  stream_reserved: ReservedBits(1, 0)
  stream_id: UInt31be
end
```

The parser should require the format clause before format-specific fields or
validation forms are used. A schema with no format clause may contain only the
format-neutral surface accepted by the schema declaration proposal; binary
primitives in that context are wrong-kind schema diagnostics, not ordinary
unresolved value names.

Future formats can add new `format <name>` clauses with their own field
vocabularies. A single schema uses one format in the first surface. Shared
ordinary Veln types, records, ADTs, and mapping targets are still referenced
through normal module imports, while representation primitives remain owned by
the selected schema format.

## Non-Goals

- Do not define the complete binary primitive vocabulary here.
- Do not implement HTTP/2 protocol state rules here.
- Do not require a network runtime.
- Do not treat schemas as aliases for internal Veln types.

## Completion Criteria

Implemented:

- The accepted grammar includes top-level `schema` and `pub schema`
  declarations.
- Schema fields may carry field-local `where` predicates in source syntax.
- Parser, AST, formatter, and editor support understand the first declaration
  slices.
- Examples show schema declarations as boundary contracts, not ordinary types.
- Parser diagnostics distinguish malformed schema syntax from ordinary type and
  value use.
- Human and JSON check examples keep malformed schema declaration syntax on the
  source diagnostic side of the boundary instead of treating it as a runtime
  schema validation failure.
- Codec declaration heads resolve schemas through schema-aware lookup,
  including same-module references, imported public schemas through written
  `use` paths and import aliases, private imported schema diagnostics,
  wrong-kind diagnostics, missing schema diagnostics, and non-reexport
  boundaries.
- Binary fixture metadata in executable specification cases resolves optional
  schema references through schema-aware lookup and checks field-path schema
  segments against the resolved schema.
- Executable schema `where` validation helper slices evaluate supported
  predicates after the owning field is decoded, including arithmetic, boolean,
  prefix `not`, and grouped predicates over the current field and earlier
  decoded fields, and report `schema.validation_failed` with byte offset,
  field path, predicate text, decoded values, and structured byte preview
  fields.
- Executable `validate_<schema>` helpers evaluate supported field-local
  `where` predicates over supplied schema-local decoded records, return the
  supplied record on success, and report `schema.validation_failed` with
  field path, predicate text, owning supplied field value, and supplied decoded
  values on failure.
- Source `format binary` schemas whose fields all use implemented exact-width
  unsigned primitives expose generated `byte_decode_<schema>` helper bindings.
- Structural schema value mapping clauses are accepted, formatted, lowered, and
  exposed to editor token metadata, including schema-local field reference,
  record construction, ADT constructor construction, pure same-module
  representation converter call, and imported public pure converter call
  assignment expressions through a written `use` path or alias, field
  selection from record-shaped structural mapping expressions, and
  decoded-field integer `+`, `-`, and `*` mapping arithmetic. Converter arguments
  may be schema-local field references or structural mapping expressions.
- The generated helper slice resolves one structural `map to Target` clause,
  or multiple clauses selected by `when field == literal` or
  `when field != literal`, when assignment expressions type check against
  target record fields, rejects invalid mapping assignments before execution,
  and returns the selected mapped record shape after field-local validation
  passes.
- Binary schemas that declare ambiguous or unsupported mapping selection report
  focused `schema.mapping_selection_*` diagnostics.
- Eligible generated `byte_encode_<schema>` helpers and `derive encode`
  codec boundaries accept one structural mapping target record when every
  visible encode field is projected through direct schema-local field
  references, record-shaped direct field projections, field selection from
  those record-shaped projections, or the implemented direct ADT constructor
  wrapper forms.
- Eligible generated `byte_encode_<schema>` helpers evaluate supported
  field-local `where` predicates over schema-local visible `Int` values during
  encode and report `schema.validation_failed` with field path, predicate
  text, owning field value, supplied schema-local values, and command-facing
  JSON details after representation failures have had priority.

Remaining:

- Runtime schema value mapping beyond the implemented schema-local field
  reference, record construction, ADT constructor construction, one pure
  same-module or imported public converter call, field selection from
  record-shaped structural mapping expressions, decoded-field integer
  arithmetic, and decoded-field integer equality or inequality selection
  slices.
- General schema decode can synthesize executable bindings for fields outside
  the implemented exact-width unsigned primitive, length-bounded `ByteView`,
  closed dispatch, and extension dispatch slices.
- The HTTP/2 design driver can express its full frame header boundary without
  placeholder text syntax.
