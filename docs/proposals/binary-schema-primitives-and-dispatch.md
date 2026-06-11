# Binary Schema Primitives And Dispatch

Status: proposed

This proposal defines the remaining binary-schema field vocabulary needed for
frame headers and frame-specific payload dispatch. It depends on a schema
declaration surface and a byte standard-library vocabulary.

The source-surface `ReservedBits(width, value)` declaration syntax is
implemented under `../specification/source-surface.md`.

## Problem

HTTP/2 frame decoding needs more than ordinary records. A frame header contains
non-byte-aligned semantic fields, endian-sensitive integers, flags, reserved
bits, and a payload whose interpretation depends on a tag value. These are
external representation facts, not internal Veln type declarations.

## Scope

Define binary schema support for:

- exact-width unsigned fields such as 8-bit, 24-bit, and 31-bit values
- endian-aware field reads and writes
- reserved bits that are consumed but not exposed as ordinary data
- flags that decode as raw bits, bitsets, or frame-specific ADTs
- length-prefixed payloads
- field references inside later field definitions
- dispatch from a tag field to payload schemas
- unknown tags that preserve raw payload bytes when permitted
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

Exact-width unsigned names should belong to the binary schema primitive
vocabulary. They are field representation names, not ordinary source-visible
numeric types.

The primitive name records the external width and byte order that the schema
must consume or emit. A decoded field maps to `Int` by default, or to an
independently declared Veln record, ADT, or wrapper through an explicit mapping
rule. This keeps schema declarations responsible for byte layout while keeping
ordinary Veln values responsible for protocol meaning.

HTTP/2 frame headers should use schema primitives such as `UInt8`, `UInt24be`,
and `UInt31be`. `UInt24be` consumes a three-byte unsigned big-endian field.
`UInt31be` represents the 31-bit unsigned value in a big-endian field position
whose remaining bit is handled as a reserved or fixed schema bit. The 31-bit
value should not become a general-purpose source type.

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

## Completion Criteria

- Examples show a binary frame header schema with fixed widths and reserved
  bits.
- Examples show tag-based payload dispatch and unknown tag preservation.
- Invalid fixed fields and truncated fields produce structured diagnostics.
- The schema vocabulary is general enough for another binary protocol example.
- The HTTP/2 design driver can express frame header and payload boundaries
  without ordinary parsing functions doing all layout work.
