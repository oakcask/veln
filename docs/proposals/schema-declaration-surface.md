# Schema Declaration Surface

Status: proposed

This proposal tracks remaining source syntax needed to declare schemas as
external representation boundaries. The first top-level declaration slice is
implemented as current behavior under `../specification/source-surface.md` and
checked examples under `../../examples/specification/`.

## Problem

Current source syntax has a first top-level `schema` declaration slice with a
single `format binary` clause, field declarations, field-local `where`
predicate syntax, and a narrow executable field-local validation helper slice.
It does not yet have complete binary primitive semantics or executable codec
bindings outside the implemented helper boundaries.

The HTTP/2 design driver needs a declaration that can say:

- a field is read from bytes rather than from an internal Veln value
- a field has a fixed external width
- a field is validated at the schema boundary
- a schema reports structural failures with field paths and byte positions

Without a schema declaration, binary protocol examples must encode external
layout in ordinary functions, which hides the boundary the driver is meant to
exercise.

## Scope

The implemented first slice covers:

- top-level `schema` declarations
- named schema fields
- field type annotations that may name `UInt1` through `UInt8`, `UInt16be`,
  `UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`, `UInt31le`, `UInt32be`,
  `UInt32le`, `UInt40be`, `UInt40le`, `UInt48be`, `UInt48le`, `UInt56be`,
  `UInt56le`, `UInt64be`, `UInt64le`, and
  `ReservedBits(width, value)` as binary schema primitives
- source-surface `ReservedBits(width, value)` declaration checking for literal
  integer arguments in `format binary` schemas
- schema visibility and module ownership rules for `schema` and `pub schema`
- schema declarations without a `format` clause when their fields use
  format-neutral type text, while binary-only field vocabulary remains gated
  by a preceding `format binary` clause
- schema references from the existing codec declaration head compatibility
  surface, including same-module bare references and imported public schema
  references through written `use` paths
- top-level public schema member aliases that re-export existing public
  schemas through the declaring module's public path and resolve through
  schema-aware lookup
- explicit schema decode and encode expressions that resolve imported public
  schema aliases through the same eligible binary helper boundary as the
  aliased public schema
- HTTP/2 frame-header decoding through explicit schema operation expressions,
  preserving the visible `length`, `kind`, `flags`, and `stream_id` record
  fields while keeping the reserved stream-id bit representation-only
- executable field-local validation helper slices that decode binary schema
  fields in declaration order and evaluate supported `where` predicates after
  the owning field is decoded
- generated `validate_<schema>` helper bindings for eligible source
  `format binary` schemas that validate a supplied schema-local decoded record
  with the same supported field-local `where` predicate language used by
  generated binary decode helpers
- generated `byte_decode_<schema>` helper bindings for source `format binary`
schemas whose fields use implemented exact-width unsigned primitives,
  visible flag bitset fields, supported representation-only
  `ReservedBits(width, value)` fields,
  length-bounded `ByteView(length_field)` or
  `ByteView(left_length - right_length)` payload fields, bounded repeat
  fields over implemented primitive, nested schema, `ByteView(length_field)`,
  or `ByteView(left_length - right_length)` payloads, direct nested binary
  schema fields, or the implemented dispatch payload slices
- generated `byte_decode_<schema>` helper bindings for format-neutral schemas
  without a `format` clause when every field is a recursive format-neutral
  visible shape made from scalar leaves, anonymous record fields, `Option<T>`,
  `List<T>`, `Vec<T>`, `Dict<String, T>`, `Result<Ok, Err>` where both payloads are
  recursive visible shapes, and same-module or public imported source ADTs
  whose constructor payloads are recursive visible shapes, with declaration
  diagnostics for unsupported helper field types
- generated `byte_encode_<schema>` helper bindings and explicit schema encode
  expressions for format-neutral schemas without a `format` clause when every
  field is a scalar leaf, `Option<scalar>` field,
  `Option<List<scalar>>` field, `List<scalar>` field,
  `Vec<scalar>` field, `Vec<Option<scalar>>` field,
  `Dict<String, scalar>` field, `Result<scalar, scalar>` field, or anonymous
  record field whose fields are supported format-neutral encode shapes. The
  supported scalar leaves are `Int`, `Bool`, `Float`, and `String`
- generated encode-time field-local validation for eligible
  `byte_encode_<schema>` helpers, using the supported schema predicate
  language over the current visible `Int` field and earlier visible `Int`
  fields after primitive, fixed-field, length, repeat, and dispatch
  representability checks have succeeded
- parser, AST, formatter, editor token, and documentation behavior for the
  implemented source surface. The completed documentation-comment schema
  reference slice is archived under
  [Schema Documentation References](../reference/implemented-proposals/schema-documentation-references.md).

The completed visible flag bitset decode binding slice is archived under
[Binary Schema Flag Decode Bindings](../reference/implemented-proposals/binary-schema-flag-decode-bindings.md).

The completed bounded repeat helper binding slice is archived under
[Binary Schema Repeat Helper Bindings](../reference/implemented-proposals/binary-schema-repeat-schema-payload-helpers.md).

The completed bounded repeat `ByteView(left_length - right_length)` payload
helper slice is archived under
[Binary Schema Repeat ByteView Subtract Helpers](../reference/implemented-proposals/binary-schema-repeat-byteview-subtract-helpers.md).

The completed direct nested binary schema decode binding slice is archived
under
[Binary Schema Direct Nested Decode Bindings](../reference/implemented-proposals/binary-schema-direct-nested-decode-bindings.md).

The completed dispatch nested repeat helper slice is archived under
[Binary Schema Dispatch Nested Repeat Helpers](../reference/implemented-proposals/binary-schema-dispatch-nested-repeat-helpers.md).

The completed general representation-only `ReservedBits(width, value)`
generated helper slice is archived under
[Binary Schema General Reserved Bitfield Layouts](../reference/implemented-proposals/binary-schema-general-reserved-bitfield-layouts.md).

The completed format-neutral `Option` helper slice, including
`Option<scalar>` fields inside nested record-shaped fields, is archived under
[Format-Neutral Schema Option Helpers](../reference/implemented-proposals/format-neutral-schema-option-helpers.md).

The completed format-neutral top-level `Option<List<scalar>>` helper slice is
archived under
[Format-Neutral Schema Option List Helpers](../reference/implemented-proposals/format-neutral-schema-option-list-helpers.md).

The completed format-neutral nested record list helper slice is archived under
[Format-Neutral Schema Nested List Helpers](../reference/implemented-proposals/format-neutral-schema-nested-list-helpers.md).

The completed format-neutral nested record `Option<List<scalar>>` helper slice
is archived under
[Format-Neutral Schema Nested Option List Helpers](../reference/implemented-proposals/format-neutral-schema-nested-option-list-helpers.md).

The completed format-neutral nested record dictionary helper slice is archived
under
[Format-Neutral Schema Nested Dict Helpers](../reference/implemented-proposals/format-neutral-schema-nested-dict-helpers.md).

The completed format-neutral `Option<Dict<String, scalar>>` helper slice is
archived under
[Format-Neutral Schema Option Dict Helpers](../reference/implemented-proposals/format-neutral-schema-option-dict-helpers.md).

The completed format-neutral `Result<scalar, scalar>` helper slice is archived
under
[Format-Neutral Schema Result Helpers](../reference/implemented-proposals/format-neutral-schema-result-helpers.md).

The completed format-neutral recursive `Result` visible-shape helper slice is
archived under
[Format-Neutral Schema Result Visible Shapes](../reference/implemented-proposals/format-neutral-schema-result-visible-shapes.md).

The completed recursive format-neutral container helper slice is archived
under
[Format-Neutral Schema Recursive Container Helpers](../reference/implemented-proposals/format-neutral-schema-recursive-container-helpers.md).

The completed format-neutral source ADT visible-shape helper slice is archived
under
[Format-Neutral Schema Source ADT Helpers](../reference/implemented-proposals/format-neutral-schema-source-adt-helpers.md).

The completed format-neutral `Vec<T>` helper slice is archived under
[Format-Neutral Schema Vec Helpers](../reference/implemented-proposals/format-neutral-schema-vec-helpers.md).

The completed scalar-only format-neutral encode helper slice is archived under
[Format-Neutral Schema Scalar Encode Helpers](../reference/implemented-proposals/format-neutral-schema-scalar-encode-helpers.md).

The completed format-neutral `Option<scalar>` encode helper slice is archived
under
[Format-Neutral Schema Option Scalar Encode Helpers](../reference/implemented-proposals/format-neutral-schema-option-scalar-encode-helpers.md).

The completed format-neutral `List<scalar>` encode helper slice is archived
under
[Format-Neutral Schema List Scalar Encode Helpers](../reference/implemented-proposals/format-neutral-schema-list-scalar-encode-helpers.md).

The completed format-neutral `Vec<scalar>` encode helper slice is archived
under
[Format-Neutral Schema Vec Scalar Encode Helpers](../reference/implemented-proposals/format-neutral-schema-vec-scalar-encode-helpers.md).

The completed format-neutral `Vec<Option<scalar>>` encode helper slice is
archived under
[Format-Neutral Schema Option Vec Encode Helpers](../reference/implemented-proposals/format-neutral-schema-option-vec-encode-helpers.md).

The completed format-neutral `Dict<String, scalar>` encode helper slice is
archived under
[Format-Neutral Schema Dict Scalar Encode Helpers](../reference/implemented-proposals/format-neutral-schema-dict-scalar-encode-helpers.md).

The completed first format-neutral container encode helper slice is archived
under
[Format-Neutral Schema Container Encode Helpers](../reference/implemented-proposals/format-neutral-schema-container-encode-helpers.md).

Historical mapping slices that predate the source-surface removal remain
archived under implemented proposal records. Current behavior removes
schema-level `map to` clauses as recorded in
[Remove Schema Map To](../reference/implemented-proposals/remove-schema-map-to.md).

This proposal remains open for:

- generated runtime helper bindings for binary schema fields outside the
  implemented exact-width unsigned primitive, visible flag bitset,
  supported representation-only reserved-bit, direct nested binary schema,
  bounded repeat, length-bounded `ByteView`, closed dispatch, and extension
  dispatch slices, and format-neutral encode helper fields beyond the
  implemented scalar, supported container, scalar-result, and anonymous
  record shapes
- schema-aware references from later schema composition surfaces beyond codec
  declaration heads, public schema member aliases, documentation comments,
  binary fixture metadata, and explicit schema operations

## Discussion Result: Binary Primitive Execution Boundary

Binary primitive execution is limited to fixed-width unsigned schema
representation fields, representation-only reserved bits, length-bounded byte
views, bounded repeats, and dispatch payloads over those eligible shapes.

Exact-width unsigned fields decode to ordinary `Int` values and encode from
ordinary `Int` values with structured range failures. Primitive names remain
schema-local representation vocabulary, not ordinary source-visible numeric
types.

Bit packing is supported only for declared adjacent primitive and
reserved-bit groups whose total width fits a fixed byte storage unit already
accepted by the schema helper surface. Schemas do not define arbitrary
bitstream parsing, signed integer families, floating-point encodings,
variable-length integers, or text encoding primitives; those require separate
proposal work when a concrete protocol slice needs them.

## Superseded Discussion Result: Codec Binding Direction

This discussion result is superseded for new design work by the completed
[Schema Binary Pattern Boundary](../reference/implemented-proposals/schema-binary-pattern-boundary.md).

A schema is still the reusable external representation contract, and it still
must not name executable API entry points in its body. The replacement design
removes the source-level `codec` declaration family instead of moving
executable names into schemas. New work should expose decode and encode
through explicit schema operations inside ordinary functions.

The older source-level rule pointed from a codec declaration to the schema it
implemented. Treat that rule as compatibility history for implemented slices,
not as the future schema composition model.

## Discussion Result: Removed Schema Value Mapping

Schema-level value mapping is not current proposal work. The source surface
rejects `map to` in schema bodies, and the implemented removal is recorded in
[Remove Schema Map To](../reference/implemented-proposals/remove-schema-map-to.md).
Projection between schema-local visible records and domain values belongs in
ordinary source functions or explicit schema operations.

## Discussion Result: Top-Level Schema Declarations

`schema` should be a normal top-level declaration beside declarations such as
`type`, `fn`, and `test`, not a modifier on a type declaration and not a
specialized `codec schema` form.

The declaration name owns an external representation boundary. Its body owns
schema-local fields, validation clauses, and the format-specific vocabulary
selected by its `format` clause. It does not create an ordinary Veln value
type by itself, and it does not imply executable decode or encode APIs. New
work should cite schemas from explicit schema operations inside ordinary
functions when a module wants to expose execution.

This keeps the parser, formatter, editor support, documentation, and module
item model direct: a source file contains a schema item with a stable name and
span. It also keeps schemas reusable across explicit schema operations,
ordinary protocol functions, fixtures, documentation, and diagnostic tests
without forcing every schema to commit to one executable direction.

## Superseded Compatibility History: Codec Schema Imports And References

The older codec declaration head slice is no longer current behavior. Current
source rejects source-level codec declarations and uses explicit schema decode
and encode operations inside ordinary functions instead. Schema visibility
still follows the ordinary source module boundary. A private schema is visible
only in its declaring module. A `pub schema` declaration is part of the
declaring module's public API when the module's source file is listed by the
package manifest's `[lib].exports`.

References to schemas use schema-aware name resolution rather than value
resolution. Explicit schema operations may reference a schema by bare name
inside the declaring module. From another module, they reference public schemas
through the written `use` module path, such as `http2::FrameHeader`. A `use`
declaration does not re-export the schema from the importing module.

Importing or referencing a schema imports only the schema item. It does not
import schema-local field names as ordinary bindings, expose a generated record
type, or make any decoder or encoder available. Executable APIs are ordinary
functions that cite explicit schema operations.

The implemented surface also accepts top-level public schema member aliases:

```text
pub schema PublicPacket = wire::Packet
```

Schema aliases resolve their targets through schema-aware lookup rather than
ordinary value or type lookup. A schema alias may publish a public schema from
an imported module through the declaring module's public path. Explicit schema
operations resolve through exported schema aliases wherever they resolve public
schemas through written module paths. Missing, private, function,
source ADT type, and codec targets are rejected at the alias declaration.
Schema aliases do not import schema-local field names, generated helper names,
codec names, or ordinary source type bindings, and they do not create wrapper
schemas, new schema identities, or generated codec aliases.

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

The implemented binary schema form starts with `format binary`. It makes
binary schema primitives such as exact-width integers, reserved bits, byte
ranges, and dispatch forms available only in that schema's field vocabulary.
The clause does not import ordinary source values, does not create executable
codec APIs, and does not change the module visibility of the schema item.

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

The parser requires the format clause before format-specific fields or
validation forms are used. A schema with no format clause may contain only
format-neutral source surface; binary primitives in that context are
wrong-kind schema diagnostics, not ordinary unresolved value names.

Future formats can add new `format <name>` clauses with their own field
vocabularies. A single schema uses one format in the first surface. Shared
ordinary Veln types, records, ADTs, and domain projection targets are still
referenced through normal module imports, while representation primitives
remain owned by the selected schema format.

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
  unsigned primitives, visible flag bitset fields, direct nested binary schema
  fields, supported representation-only `ReservedBits(width, value)` fields,
  or bounded repeat fields over implemented primitive, nested schema, or
  `ByteView(length_field)` payloads expose generated
  `byte_decode_<schema>` helper bindings.
- Format-neutral schemas without a `format` clause whose fields are recursive
  visible shapes made from scalar leaves, anonymous record fields,
  `Option<T>`, `List<T>`, `Vec<T>`, `Dict<String, T>`, and
  `Result<Ok, Err>` where both payloads are recursive visible shapes, plus
  same-module or public imported source ADTs whose constructor payloads are
  recursive visible shapes, expose generated `byte_decode_<schema>` helper
  bindings that accept and return the schema-local visible record through
  `Result<T, String>`.
  Unsupported format-neutral helper fields report
  `schema.format_neutral_decode_helper` at the field declaration with a
  related note for the generated helper boundary.
- Eligible generated `byte_encode_<schema>` helpers evaluate supported
  field-local `where` predicates over schema-local visible `Int` values during
  encode and report `schema.validation_failed` with field path, predicate
  text, owning field value, supplied schema-local values, and command-facing
  JSON details after representation failures have had priority.

Remaining:

- General schema decode can synthesize executable bindings for fields outside
  the implemented exact-width unsigned primitive, visible flag bitset,
  supported representation-only reserved-bit, direct nested binary schema,
  bounded repeat, length-bounded `ByteView`, closed dispatch, extension
  dispatch, and recursive format-neutral visible-shape helper boundary.
