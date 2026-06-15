# Binary Schema Primitives And Dispatch

Status: proposed

This proposal defines the remaining binary-schema field vocabulary needed for
frame headers and frame-specific payload dispatch. It depends on a schema
declaration surface and a byte standard-library vocabulary.

The source-surface `ReservedBits(width, value)` declaration syntax is
implemented under `../specification/source-surface.md`.
The declaration-time exact-width primitive names `UInt1` through `UInt8`,
`UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`, `UInt32be`, and
`UInt32le` are also implemented there for `format binary` schema field type
positions only. The executable frame-header
primitive decode slice is implemented under `../specification/execution.md`:
it consumes `UInt24be`, `UInt8`, `UInt8`, `ReservedBits(1, 0)`, and
`UInt31be` from a `ByteView`, returns ordinary `Int` fields for the visible
values, and reports structured schema failures for truncated fields and
reserved-bit mismatches. Generated schema helpers also consume byte-aligned
`ReservedBits(width, value)` fields up to four bytes wide as
representation-only fields, omit those fields from decoded records and
mapping source values, encode them from the declared fixed value, and report
the same reserved-bit mismatch and truncation diagnostic shapes. Generated
schema helpers also consume and encode packed `ReservedBits(width, value)`
prefixes where widths one through seven are followed by the visible `UIntN`
primitive that completes the byte and widths nine through fifteen are
followed by the visible `UIntN` primitive that completes the same two-byte
big-endian storage unit. The helpers validate the high reserved bits, decode
or encode the low visible bits from the ordinary record field, omit the
reserved field from decoded records and mapping source values, and report the
same reserved-bit mismatch, truncation, and
`codec.encode_value_unrepresentable` diagnostic shapes. Generated schema
helpers also consume and encode the one-byte suffix form where a visible
`UIntN` field is followed immediately by `ReservedBits(width, value)` and
the two widths complete the byte. The helpers decode or encode the visible
field from the high bits, validate or emit the declared low reserved bits,
omit the reserved field from decoded records and mapping source values, and
report the same reserved-bit mismatch, truncation, and
`codec.encode_value_unrepresentable` diagnostic shapes. Generated schema
helpers also decode and encode standalone visible
`UInt1` through `UInt7` fields as one byte each, expose the declared low bits
as ordinary `Int` values, preserve structural mapping and generated
decode-step and derived codec eligibility, and report existing truncation and
`codec.encode_value_unrepresentable` range-failure shapes. The
generated helper slice also treats visible exact-width fields with a
field-local equality predicate such as `field == literal` as schema-owned
fixed fields, leaves matching values visible in the decoded result, and
reports `schema.fixed_field_mismatch` with byte offset, field path, expected
value, actual value, and byte preview details when the input differs. The
width-sample primitive decode slice consumes
`UInt16be` and `UInt32be`, returns ordinary `Int` values, and reports the
same structured truncation shape. The narrow HTTP/2 frame helper also returns
a bounded payload `ByteView` selected by the decoded length and reports
`schema.length_out_of_bounds` when closed input cannot provide that payload
range. The generated helper slice also implements `UInt16le`, `UInt24le`, and
`UInt32le` as little-endian unsigned primitives for schema decode and encode
helpers, returns ordinary `Int` values, preserves structural decode mappings,
and reports width-specific encode range failures. The narrow closed
dispatch slice implements
`Dispatch(tag_field, tag => Primitive, ...)` for generated binary schema
decode helpers, decodes known case payloads as `Int`, and reports
`schema.dispatch_unknown_tag` with structured tag and byte context for unknown
tags. The narrow extension-tolerant dispatch slice implements
`ExtensionDispatch(tag_field, length_field, tag => Primitive, ...)` for
generated binary schema decode helpers, decodes known case payloads as
`SchemaDispatchPayload::Known(Int)`, preserves unknown tags and bounded raw
payload bytes as `SchemaDispatchPayload::Unknown(tag, payload)`, and still
reports `schema.length_out_of_bounds` for malformed payload ranges. The
same-module nested payload slice also implements known
`Dispatch(..., tag => SchemaName, ...)` and
`ExtensionDispatch(..., tag => SchemaName, ...)` cases for generated binary
schema decode helpers, returns the nested schema's decoded record shape for
known cases, keeps extension-tolerant unknown tags opaque, and reports nested
payload failures with the nested schema field path and absolute byte offset.
Public imported nested binary schema payloads named through written `use`
paths are accepted by those same dispatch decode helper slices and decode to
the imported schema's record shape. Imported private, missing, wrong-kind,
non-binary, recursive, or otherwise ineligible payload schemas use the
existing `schema.dispatch_payload` diagnostic shape.
The same-module and imported public nested payload encode slice implements known
`Dispatch(..., tag => SchemaName, ...)` and
`ExtensionDispatch(..., tag => SchemaName, ...)` cases for generated binary
schema encode helpers, uses the nested schema decoded record shape for closed
payload fields and `SchemaDispatchPayload<NestedRecord>` for
extension-tolerant payload fields, accepts public imported payload schemas
named through written `use` paths, preserves extension-tolerant unknown raw
payload bytes, and keeps nested schema encode failures on the nested schema
field path. Those nested dispatch payload decode and encode slices route
selected nested payload schemas through the same generated binary schema
helper path as ordinary schema fields; focused executable examples cover
fixed-field validation, byte-aligned reserved fields, little-endian primitive
payload fields, extension-tolerant known payloads, and nested helper
diagnostics. A checked non-HTTP telemetry envelope combines the implemented
helper vocabulary in one generated decode-and-encode schema. Recursive or
otherwise ineligible dispatch payload schemas, broader unsupported field
layouts, and schema value mapping beyond the implemented structural slices
remain proposal work.
The narrow one-byte visible flag bitset slice is implemented as `Flag8` for
generated binary schema decode and encode helpers. `Flag8` consumes and emits
one byte through the existing `UInt8` representation path, decodes to the
source-visible `Flag8(bits: Int)` value instead of a raw `Int`, preserves
existing `UInt8` field behavior, shares exact-width truncation behavior, and
reports existing encode value-representation failures when `bits` cannot be
represented in one byte. The structural mapping slice also treats decoded
`Flag8` fields as schema-local `Flag8` values for direct target-field
assignment, same-module ADT constructor expressions, one pure same-module
converter call, and one imported public pure converter call through a written
`use` path or alias. Generated encode helpers keep schema-local `Flag8`
encode behavior and accept a direct mapped-record encode boundary when every
visible encode field, such as `target_flags = flags`, can be projected by the
existing direct assignment rule.
The narrow bounded repeated payload slice is implemented as
`Repeat(count_field, Payload)` for generated binary schema decode and encode
helpers. The count field must be an earlier visible `Int` field in the same
schema, and the payload must be one of the implemented byte-aligned
exact-width unsigned primitives or an eligible nested binary schema payload.
Primitive repeats decode and encode as `List<Int>`; nested schema repeats
decode and encode as lists of the nested schema's decoded record shape.
Encode rejects list length, primitive range, and nested element
representation mismatches through `EncodeError`, and element failures append
an index segment before nested schema field path segments.
The generated length-bounded byte payload slice is implemented as
`ByteView(length_field)` and `ByteView(left_length - right_length)` for
generated binary schema decode and encode helpers. The length operands must be
earlier visible `Int` fields in the same schema, the encoded value record keeps
the length operand fields and the `ByteView` payload field, the helper writes
the earlier fields normally and then writes exactly the bounded bytes from the
supplied view, negative computed decode lengths report
`schema.length_out_of_bounds`, and mismatched encode view counts return the
existing structured `EncodeError` value-representation shape.

## Problem

HTTP/2 frame decoding needs more than ordinary records. A frame header contains
non-byte-aligned semantic fields, endian-sensitive integers, flags, reserved
bits, and a payload whose interpretation depends on a tag value. These are
external representation facts, not internal Veln type declarations.

## Scope

Define remaining binary schema support beyond the implemented narrow
primitive, little-endian primitive width, payload-boundary, closed-dispatch,
extension-dispatch, same-module nested dispatch payload, imported nested
dispatch payload decode, and imported nested dispatch payload encode slices
for:

- executable exact-width unsigned field reads and writes beyond the
  implemented primitive helper slices
- endian-aware field reads and writes
- reserved-bit forms beyond the implemented byte-aligned representation-only
  fields, one-byte and two-byte packed reserved prefixes, one-byte packed
  reserved suffixes, and `ReservedBits(1, 0)` plus `UInt31be` shared-bit
  layout
- flag vocabulary beyond the implemented one-byte `Flag8` bitset and its
  structural mapping boundary, including raw-bit variants and frame-specific
  ADTs
- general schema-declared length-prefixed payloads beyond the implemented
  `ByteView(length_field)` and `ByteView(left_length - right_length)` decode
  and encode helper slices
- field references inside later field definitions beyond implemented
  bounded repeat counts, byte-view lengths, dispatch tags, and extension
  dispatch lengths
- recursive or otherwise ineligible dispatch payload schemas beyond the
  implemented same-module and imported public nested helper slices
- schema-level structural validation

## Discussion Result: Dependent Structure Boundary

Binary schemas should support only representation-local dependencies over
fields decoded earlier in the same schema. A prior field may size a later byte
range, select a tagged payload schema, drive fixed or reserved-field
validation, constrain a payload multiple, or participate in mapping into an
independently declared Veln value.

The schema vocabulary should not include general loops, arbitrary function
calls, negotiated settings lookup, connection or stream state access, mutation,
or recovery behavior. Those concerns belong in explicit codec functions,
library codec state, or protocol-core transition functions.

Bounded repeated structures may be considered as schema primitives when their
count or byte length is derived from a prior field and diagnostics remain
field-path and byte-offset based. Unbounded repetition and stateful parsing
must stay outside schema declarations.

## Discussion Result: Exact-Width Primitive Names

The declaration-time source-surface slice for exact-width unsigned names now
lives under `../specification/source-surface.md`. Those names belong to the
binary schema primitive vocabulary as field representation names, not ordinary
source-visible numeric types.

The remaining proposal work is to make the primitive name record the external
width and byte order that the schema must consume or emit. A decoded field
should map to `Int` by default, or to an independently declared Veln record,
ADT, or wrapper through an explicit mapping rule. This keeps schema
declarations responsible for byte layout while keeping ordinary Veln values
responsible for protocol meaning.

The implemented narrow executable slices already make `UInt1` through
`UInt8`, `UInt16be`, `UInt24be`, `UInt31be`, and `UInt32be` consume
fixed-width unsigned big-endian fields, and `UInt16le`, `UInt24le`, and
`UInt32le` consume fixed-width unsigned little-endian fields, then return
ordinary `Int` values for visible fields.
The implemented exact-width primitive encode helper slice emits those visible
ordinary `Int` fields in their declared byte order as `ByteChunk` output and
reports structured `EncodeError` range failures. The implemented reserved-bit
encode slice also
accepts byte-aligned `ReservedBits(width, value)` fields, omits the reserved
field from the encoder value record, and writes the declared fixed value. It
also accepts `ReservedBits(1, 0)` immediately before `UInt31be` and writes
the required zero high bit in the shared four-byte stream identifier
position. The implemented standalone sub-byte primitive slice consumes
`UInt1` through `UInt7` visible fields from one byte each, masks the declared
low bits into ordinary `Int` values, emits one byte per field from accepted
`Int` values, preserves structural mapping and generated decode-step and
derived codec eligibility, and reports existing truncation and
`codec.encode_value_unrepresentable` range-failure shapes. The implemented
closed-dispatch primitive encode slice accepts an earlier visible exact-width
unsigned tag field and exact-width unsigned primitive payload cases, chooses
the payload case from the encoded tag value, and reports structured
`EncodeError` failures for unknown tags or selected payload values outside the
primitive range. The implemented extension-dispatch primitive encode slice
accepts earlier visible exact-width unsigned tag and length fields plus a
`SchemaDispatchPayload<Int>` payload field, writes known primitive payloads,
preserves unknown raw bounded payload bytes, rejects tag or payload variant
disagreements, rejects length fields that do not match the emitted payload byte
count, and reports primitive range failures through structured `EncodeError`
values. The implemented same-module nested dispatch payload encode slice uses
the same earlier-schema eligibility boundary as nested dispatch decode, writes
selected nested records for closed dispatch, writes
`SchemaDispatchPayload::Known` nested records for extension-tolerant dispatch,
keeps unknown extension-tolerant raw payload preservation unchanged, and
reports nested field failures through structured `EncodeError` values. The
implemented imported nested dispatch payload encode slice accepts public
imported payload schemas named through written `use` paths in the same closed
and extension-tolerant encode helper shapes, writes selected imported nested
records through the imported schema helper, keeps unknown extension-tolerant
raw payload preservation unchanged, and reports nested imported field failures
through structured `EncodeError` values. The implemented generalized nested
dispatch payload helper path reuses the generated binary schema helper for
selected nested same-module and imported public payload schemas, including
the supported primitive, reserved-field, fixed-field decode, endian, and
diagnostic behavior already available to ordinary generated schema fields.
The implemented `Flag8` helper slice consumes and emits one-byte visible
bitsets as source-visible `Flag8(bits: Int)` values while leaving existing
`UInt8` fields as ordinary `Int` values. Structural decode mappings can use
that decoded `Flag8` value through the implemented field reference,
same-module ADT constructor, pure same-module converter, and imported public
pure converter expression forms. Direct mapped-record encode is implemented
only when every visible encode field can be projected back to a schema-local
field by the existing direct assignment rule.
The implemented bounded repeated helper slice consumes and emits
`Repeat(count_field, Payload)` fields when `count_field` names an earlier
visible `Int` field in the same schema and `Payload` is `UInt8`, `UInt16be`,
`UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`, `UInt32be`, `UInt32le`, or an
eligible nested binary schema payload.
General schema-owned decode and encode beyond the implemented slices,
recursive or otherwise ineligible dispatch payload schemas, and mapping
beyond the implemented slices remain proposal work. A `UInt31be` field
represents the 31-bit unsigned value in a big-endian field position whose
remaining bit is handled as a reserved or fixed schema bit. The 31-bit value
should not become a general-purpose source type.

## Discussion Result: Reserved Bit Spelling

Reserved bits are spelled as schema-local fixed fields that are consumed from
the external representation but omitted from the mapped Veln value by default.
The byte-aligned `ReservedBits(width, value)` slice, one-byte and two-byte
packed reserved prefix slices, one-byte packed reserved suffix slice, and the
`ReservedBits(1, 0)` plus `UInt31be` shared-bit layout are implemented under
`../specification/execution.md`.
Remaining proposal work is limited to non-byte-aligned shapes outside those
layouts and any later opt-in mapping exposure.

Use a `ReservedBits(width, value)` binary schema primitive for this purpose.
The field still has a schema-local name so diagnostics can report a stable
field path, but the primitive marks the field as representation-only so it is
not mapped into the produced Veln record or ADT unless a later explicit mapping
rule opts in.

For HTTP/2, the stream identifier field is therefore written as a one-bit
reserved field followed by the visible 31-bit value:

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

`ReservedBits` is only for representation bits whose required value is fixed
by the external format and whose decoded value is not semantically meaningful
to the program. Visible flags, extension bits, and protocol values should use
ordinary fields with validation or mapping rules instead.

Invalid reserved bits are schema structural failures. Diagnostics should point
at the reserved field path and byte offset, report the expected bit pattern and
actual bit pattern, and keep protocol-state causes out of the primary schema
failure.

## Discussion Result: Unknown Dispatch Preservation

Tag dispatch should preserve unknown tags when the schema author explicitly
marks the dispatch as extension-tolerant. The decoded value should keep the raw
tag value, the already validated header fields that are part of the enclosing
schema, and the bounded payload bytes selected by the length field. Unknown
payload bytes are opaque; schemas should not invent a partial internal shape
for a tag whose representation is not declared.

Unknown dispatch is not an error by itself in an extension-tolerant schema.
Errors still come from structural facts the schema owns, such as truncated
input, invalid fixed or reserved fields, a length that cannot be sliced from
the buffered bytes, or an unknown tag in a closed dispatch. Protocol code can
then choose whether to ignore the unknown value, surface it to callers, or
reject it because of protocol state.

This keeps binary schemas useful for extensible protocols without making the
codec silently discard bytes that fixtures, diagnostics, forwarding, or later
extension handling may need. The retained payload must remain bounded by the
decoded length field so extension preservation cannot keep unrelated consumed
input alive.

The implemented narrow slice exposes this through
`ExtensionDispatch(tag_field, length_field, tag => Payload, ...)`, where the
tag and length fields must already be decoded in the same schema, known cases
use implemented exact-width unsigned primitive payloads, same-module nested
binary schema payloads, or public imported nested binary schema payloads, and
unknown cases retain the bounded raw `ByteView`. Recursive or otherwise
ineligible nested payload schemas and protocol-state legality checks remain
outside this slice.

## Discussion Result: Binary Schema Value Mapping

Binary schema values should use the structural mapping rule from the schema
declaration surface: a schema maps validated schema-local fields into an
independently declared record or ADT constructor through an explicit mapping
clause.

Exact-width integer primitives produce ordinary `Int` values unless a
schema-declared representation conversion maps the field into a visible domain
type. `Flag8` fields produce source-visible `Flag8(bits: Int)` values in the
implemented helper and mapping slices. Byte ranges produce `ByteView` or
`ByteChunk` values according to the field vocabulary. Reserved fields, fixed
fields, and other representation-only fields stay available for validation
and diagnostics but are omitted from the mapped value unless the mapping
explicitly includes them.

Tag dispatch maps known cases to explicit target constructors or records. An
extension-tolerant unknown case must map to a target shape that can carry the
raw tag value and bounded payload bytes, such as an `Unknown` constructor. A
closed dispatch has no unknown mapping; an unrecognized tag remains a schema
structural failure.

The mapping checker should reject missing target fields, duplicate target
assignments, unknown constructors, and assignments whose schema-local value
type does not match the target field. It should not run arbitrary source
functions or consult protocol state; those conversions belong in explicit
codec functions after schema decoding.

## Discussion Result: Field Reference Scope

Field references inside binary schemas should be schema-local, unique, and
forward-only. A field definition may reference fields decoded earlier in the
same schema by their field name. It must not reference later fields, ordinary
source values, imported functions, runtime settings, connection state, or
stream state.

Schema field names are unique within the field scope that declares them.
Shadowing is rejected so every field path used by diagnostics remains stable.
Dispatch cases and nested payload schemas do not implicitly capture outer
fields; any shared value must be passed through an explicit schema field or
context parameter accepted by a later syntax proposal.

References denote the validated schema value of the earlier field, not the raw
bytes that produced it. Type checking uses the role required by the consuming
primitive: byte lengths and consumed counts require a `ByteCount` or a
non-negative integer field with a checked conversion; dispatch tags require a
field whose ordinary decoded value can be compared with the declared case tag;
fixed and reserved-field validation may only compare compatible integer or bit
patterns.

A failed reference is a schema declaration error, not a codec failure at input
time. Diagnostics should report the reference span, name the missing,
forward, or wrong-typed field, and include the candidate field path when the
author likely referred to an earlier field with a compatible role.

## Non-Goals

- Do not encode HTTP/2 stream-state legality in schema declarations.
- Do not require HPACK support.
- Do not define network effects or task scheduling.
- Do not optimize binary layout.

## Remaining Completion Criteria

- The HTTP/2 design driver can express frame header and payload boundaries
  through general schema declarations instead of the current narrow helper.
