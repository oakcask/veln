# Binary Schema Primitives And Dispatch

Status: proposed

This proposal defines the remaining binary-schema field vocabulary needed for
frame headers and frame-specific payload dispatch. It depends on a schema
declaration surface and a byte standard-library vocabulary.

The source-surface `ReservedBits(width, value)` declaration syntax is
implemented under `../specification/source-surface.md`.
The declaration-time exact-width primitive names `UInt8`, `UInt16be`,
`UInt16le`, `UInt24be`, `UInt31be`, and `UInt32be` are also implemented there
for `format binary` schema field type positions only. The executable frame-header
primitive decode slice is implemented under `../specification/execution.md`:
it consumes `UInt24be`, `UInt8`, `UInt8`, `ReservedBits(1, 0)`, and
`UInt31be` from a `ByteView`, returns ordinary `Int` fields for the visible
values, and reports structured schema failures for truncated fields and
reserved-bit mismatches. The width-sample primitive decode slice consumes
`UInt16be` and `UInt32be`, returns ordinary `Int` values, and reports the
same structured truncation shape. The narrow HTTP/2 frame helper also returns
a bounded payload `ByteView` selected by the decoded length and reports
`schema.length_out_of_bounds` when closed input cannot provide that payload
range. The generated helper slice also implements `UInt16le` as a two-byte
little-endian unsigned primitive for schema decode and encode helpers, returns
ordinary `Int` values, preserves structural decode mappings, and reports the
same unsigned 16-bit encode range failures as `UInt16be`. The narrow closed
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
field path. General schema decode, general schema encode, generalized
dispatch payload decode, generalized dispatch payload encode, and schema value
mapping remain proposal work.

## Problem

HTTP/2 frame decoding needs more than ordinary records. A frame header contains
non-byte-aligned semantic fields, endian-sensitive integers, flags, reserved
bits, and a payload whose interpretation depends on a tag value. These are
external representation facts, not internal Veln type declarations.

## Scope

Define remaining binary schema support beyond the implemented narrow
primitive, payload-boundary, closed-dispatch, extension-dispatch,
same-module nested dispatch payload, imported nested dispatch payload decode,
and imported nested dispatch payload encode slices
for:

- executable exact-width unsigned field reads and writes beyond the
  implemented narrow primitive decode slices
- endian-aware field reads and writes
- reserved bits that are consumed but not exposed as ordinary data
- flags that decode as raw bits, bitsets, or frame-specific ADTs
- general schema-declared length-prefixed payloads
- field references inside later field definitions
- general dispatch from a tag field to payload schemas
- generalized extension-tolerant unknown tags that preserve raw payload bytes
  when permitted
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

The implemented narrow executable slices already make `UInt8`, `UInt16be`,
`UInt24be`, `UInt31be`, and `UInt32be` consume fixed-width unsigned
big-endian fields, and `UInt16le` consume a fixed-width unsigned
little-endian field, then return ordinary `Int` values for visible fields.
The implemented exact-width primitive encode helper slice emits those visible
ordinary `Int` fields in their declared byte order as `ByteChunk` output and
reports structured `EncodeError` range failures. The implemented reserved-bit
encode slice also
accepts `ReservedBits(1, 0)` immediately before `UInt31be`, omits the reserved
field from the encoder value record, and writes the required zero high bit in
the shared four-byte stream identifier position. The implemented
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
through structured `EncodeError` values. General schema-owned decode and
encode beyond the implemented slices, generalized dispatch payload encode, and
mapping beyond the implemented slices remain proposal work. A `UInt31be` field
represents the 31-bit unsigned value in a big-endian field position whose
remaining bit is handled as a reserved or fixed schema bit. The 31-bit value
should not become a general-purpose source type.

## Discussion Result: Reserved Bit Spelling

Reserved bits should be spelled as schema-local fixed fields that are consumed
from the external representation but omitted from the mapped Veln value by
default.

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
unknown cases retain the bounded raw `ByteView`. Generalized nested payload
schemas and protocol-state legality checks remain outside this slice.

## Discussion Result: Binary Schema Value Mapping

Binary schema values should use the structural mapping rule from the schema
declaration surface: a schema maps validated schema-local fields into an
independently declared record or ADT constructor through an explicit mapping
clause.

Exact-width integer primitives produce ordinary `Int` values unless a
schema-declared representation conversion maps the field into a visible domain
type. Byte ranges produce `ByteView` or `ByteChunk` values according to the
field vocabulary. Reserved fields, fixed fields, and other representation-only
fields stay available for validation and diagnostics but are omitted from the
mapped value unless the mapping explicitly includes them.

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

- Executable examples show binary schema writes and general schema-owned
  fixed-width reads beyond the implemented frame-header, width-sample,
  `UInt16le` little-endian primitive, primitive encode helper, reserved-bit
  encode helper, closed-dispatch
  primitive plus same-module and imported public nested encode helper,
  extension-dispatch primitive plus same-module and imported public nested
  encode helper, HTTP/2 payload boundary helper, and narrow closed-dispatch
  and extension-dispatch decode slices.
- Generalized dispatch payload schemas can decode nested known payload shapes,
  and generalized dispatch payload schemas can encode nested known payload
  shapes, while keeping extension-tolerant unknown payload bytes opaque.
- Invalid fixed fields in general schema decode produce structured
  diagnostics beyond the implemented frame-header truncation and reserved-bit
  mismatch details.
- The schema vocabulary is general enough for another binary protocol example.
- The HTTP/2 design driver can express frame header and payload boundaries
  through general schema declarations instead of the current narrow helper.
