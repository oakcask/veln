# Schema Binary Pattern Boundary

Status: implemented

This record preserves the completed source-level `codec` cleanup and explicit
schema operation boundary. Current behavior is specified by
`../../specification/source-surface.md`,
`../../specification/execution.md`, and executable examples under
`../../../examples/specification/`.

The completed boundary removes the source-level `codec` declaration surface
and repositions `schema` as syntax for binary pattern matching over external
byte representations. A schema remains a declarative description of wire
layout, local validation, field paths, dispatch, and byte-count ownership.
Executable protocol APIs, parser state, projection into domain values, and
stateful encoding live in ordinary Veln functions.

This record closes the remaining design direction from
[Codec Execution Boundary](../../proposals/codec-execution-boundary.md) and
narrows the schema surface described by
[Schema Declaration Surface](../../proposals/schema-declaration-surface.md)
to representation patterns plus explicit schema operations.

## Problem

The current schema and codec design has grown into several overlapping
surfaces:

- `schema` declarations define binary field layout, validation, dispatch, and
  generated helper eligibility; the older schema mapping surface has been
  removed.
- Generated helpers expose derived names such as `byte_decode_<schema>`,
  `byte_decode_step_<schema>`, and `byte_encode_<schema>`.
- `codec` declarations add another named top-level item with direction lists,
  `derive` clauses, `with` clauses, import visibility, and call behavior.
- The removed schema mapping surface could change the helper return shape and
  affect whether encode was available.
- Diagnostics use both schema-owned and codec-owned ids for closely related
  representation failures.

The result is hard to teach and hard to read. A user has to know which
implicit helper exists, which shape it returns, whether a separate codec item
wraps it, and whether imports should target the schema, generated helper,
codec item, or an ordinary function.

The HTTP/2 Sans-I/O pilot points at a simpler model. Its useful test cases
exercise schema-owned byte layout, payload windows, dispatch, byte offsets,
and protocol diagnostics. The `codec` syntax mostly acts as wrapper coverage
around generated schema helpers or hand-written functions. That is a weak
reason to keep a second top-level declaration family.

## Implemented Boundary

Source-level `codec` and `pub codec` declarations are removed.

`schema` declarations describe binary pattern declarations: a schema names a
byte pattern that can be matched against a bounded
`ByteView`, producing a visible schema-local record, consumed byte count,
readiness, or structured failure.

The public source operation is an explicit schema decode expression, not an
implicit generated helper name:

```veln
match decode Http2FrameHeaderWire from pending.bytes at pending.base_offset
Decoded(header, consumed) => accept_frame_header(state, header, consumed)
NeedMore(readiness) => wait_for_input(state, readiness)
Invalid(error) => reject_peer(state, error)
end
```

This expression returns `DecodeStep<T>`, where `T` is the schema-local visible
record shape. It uses the supplied `ByteView` as the bounded input and the
supplied `ByteOffset` only for diagnostics and consumed-position accounting.
It does not create or require a source-visible function named after the
schema.

Closed-input helper syntax is outside this completed boundary. Current
source-visible decode behavior cites the schema directly with a bounded input
view and explicit base offset:

```veln
decode Http2SettingsPayloadWire from view at base_offset
```

The source model does not expose generated helper identifiers for schema
operations as public API. The implementation may lower schema operations to
internal helper functions, but those helpers are not importable source names
or documentation targets.

## Schema Responsibility

A binary schema owns representation-local facts:

- field order, byte widths, byte order, and bit packing
- reserved and fixed field validation
- length-bounded `ByteView` payload windows
- bounded repeat fields
- schema-local dispatch selected from earlier decoded fields
- byte offsets, field paths, and byte previews for diagnostics
- the visible schema-local record shape returned by decode

The schema pattern may depend only on values decoded earlier in the same
schema. It must not read protocol state, negotiated settings, mutable decoder
state, or external resources.

Schemas return schema-local visible records. Projection into domain records or
ADTs is ordinary Veln code:

```veln
fn header_from_wire(wire: {length: Int, kind: Int, flags: Int, stream_id: Int}) -> FrameHeader
	FrameHeader {
		length: wire.length,
		kind: frame_kind_from_int(wire.kind),
		flags: wire.flags,
		stream_id: wire.stream_id,
	}
end
```

This aligns with
[Remove Schema Map To](remove-schema-map-to.md):
schema
syntax describes representation, while ordinary functions describe domain
meaning.

## Encoding

Removing `codec` keeps schema-backed encoding while removing the named codec
wrapper and implicit API selection rules.

For direct schema-local values, use an explicit schema encode operation:

```veln
encode Http2FrameHeaderWire from header_wire
```

The operation returns a representation-specific result such as
`Result<ByteChunk, EncodeError>` or an encode-step shape chosen by the
surrounding API. It is accepted only when the schema can reconstruct every
visible field and satisfy reserved, fixed, length, repeat, and dispatch facts
from the supplied schema-local value.

Stateful, budgeted, streaming, canonicalizing, or domain-value encoders are
ordinary functions. They may call the explicit schema encode operation after
projecting their domain value into the schema-local visible record:

```veln
fn encode_frame_header(header: FrameHeader) -> Result<ByteChunk, EncodeError>
	encode Http2FrameHeaderWire from header_to_wire(header)
end
```

This keeps encoder state, output budgets, partial output, and resume records
visible as ordinary Veln values instead of hiding them behind `codec`
direction rules.

## Visibility And Imports

`pub schema` controls whether other modules may cite a schema in explicit
schema operations. Private schemas remain usable only in their declaring
module. Public schema aliases remain schema aliases only.

Modules expose stable protocol APIs by exporting ordinary functions:

```veln
pub fn decode_http2_frame(input: PendingInput) -> FrameDecodeStep
	...
end
```

Importing a schema does not import generated helper names or executable codec
items. Importing a function imports an executable API. This gives facade
modules one ordinary mechanism for deciding which decode and encode entry
points they publish.

## Diagnostics

Representation failures use schema-owned diagnostic ids when the failed fact
belongs to a schema pattern:

- truncated input for a declared field
- reserved or fixed field mismatch
- length-bounded payload range failure
- repeat count mismatch
- dispatch tag mismatch for closed schema dispatch
- schema-local encode value unrepresentability

Generated schema diagnostics that were named under `codec.*` are
reclassified when their failed fact is representation-local. For example, an
encode value that cannot fit a declared binary field is reported as a schema
encode representability failure, not as a codec wrapper failure.

Ordinary protocol functions remain free to produce protocol-owned diagnostics
when a decoded representation is valid bytes but invalid for HTTP/2 state,
settings, stream lifecycle, HPACK state, or recovery policy.

## HTTP/2 Sans-I/O Shape

The HTTP/2 pilot reads as an ordinary state machine that applies schema
patterns where bytes must be interpreted:

```veln
fn receive_frame(pending: PendingInput, state: ConnectionState) -> FrameStep
	match decode Http2FrameHeaderWire from pending.bytes at pending.base_offset
	Decoded(header_wire, header_count) =>
		decode_frame_payload(pending, state, header_wire, header_count)
	NeedMore(readiness) =>
		FrameNeedMore(readiness)
	Invalid(error) =>
		FrameInvalid(schema_error_to_protocol_error(error))
	end
end
```

Frame-type payload schemas remain useful for representation-local facts:
payload lengths, reserved bits, fixed lengths, extension-tolerant dispatch,
and field paths. HPACK and HTTP/2 stream-state rules remain ordinary functions
with explicit immutable state values.

This shape matches the pilot's strongest evidence: schema helpers make binary
patterns pleasant to write, while `codec` declarations add little beyond a
second name for calls that ordinary functions can already own.

## Completion

The explicit schema decode and encode expression slices are implemented and
specified under `../../specification/source-surface.md`, with executable
coverage under `../../../examples/specification/run/schema-decode-expression/`
and `../../../examples/specification/run/schema-encode-expression/`. Direct
decode-step executable examples now apply schemas through explicit decode
expressions. The source-level `codec` declaration removal slice is
implemented and archived under
`remove-source-codec-declarations.md`.
Current parser and source-surface fixtures reject `codec` and `pub codec`
declarations with a migration diagnostic that points to ordinary functions
plus explicit schema decode and encode expressions. Former codec examples
that describe current source behavior now use ordinary functions or explicit
schema operations. The source-visible generated helper cleanup slice is
implemented and archived under
`schema-helper-public-surface-cleanup.md`.
Specification routes now describe explicit schema operations as the public
source surface, and executable examples that still call generated helper names
are compatibility or diagnostic migration fixtures. One completed migration
slice has reclassified generated schema encode value representability failures
to `schema.encode_value_unrepresentable`, as archived under
[Schema-Owned Encode Value Diagnostics](schema-owned-encode-value-diagnostics.md).
The generated schema dispatch diagnostic reclassification slice is also
implemented and archived under
[Schema-Owned Dispatch Value Diagnostics](schema-owned-dispatch-value-diagnostics.md).

Compatibility shims may remain inside the compiler or runtime, but source
examples teach explicit schema operation names and ordinary functions rather
than generated helper names or `codec` declarations.

## Non-Goals

- Do not add general parser combinators to `schema`.
- Do not let schemas access protocol state, negotiated settings, effects, or
  mutable cursors.
- Do not keep `codec` as a deprecated spelling after the replacement surface
  is available.
- Do not preserve schema `map to` as the way to project into domain values.
- Do not make generated helper names part of the public source namespace.
- Do not make HTTP/2 a standard-library commitment.

## Completion Evidence Summary

- Current specification no longer lists `codec` as a top-level declaration.
- Current specification describes schemas as representation pattern
  declarations with explicit decode and encode operations.
- Public executable examples use explicit schema operations or ordinary
  functions; source-visible generated helper names remain only in
  compatibility and diagnostic migration cases.
- HTTP/2 Sans-I/O examples decode frame headers and payloads by applying
  schema patterns inside ordinary state-transition functions.
- Representation-local failures use schema-owned diagnostic ids.
