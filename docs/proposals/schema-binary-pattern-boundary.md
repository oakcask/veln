# Schema Binary Pattern Boundary

Status: proposed

This proposal removes the source-level `codec` declaration surface and
repositions `schema` as syntax for binary pattern matching over external byte
representations. A schema remains a declarative description of wire layout,
local validation, field paths, dispatch, and byte-count ownership. Executable
protocol APIs, parser state, projection into domain values, and stateful
encoding move back to ordinary Veln functions.

If accepted, this proposal supersedes the remaining open design direction in
[Codec Execution Boundary](codec-execution-boundary.md) and narrows the
schema surface described by
[Schema Declaration Surface](schema-declaration-surface.md) to representation
patterns plus explicit schema operations.

## Problem

The current schema and codec design has grown into several overlapping
surfaces:

- `schema` declarations define binary field layout, validation, mapping,
  dispatch, and generated helper eligibility.
- Generated helpers expose derived names such as `byte_decode_<schema>`,
  `byte_decode_step_<schema>`, and `byte_encode_<schema>`.
- `codec` declarations add another named top-level item with direction lists,
  `derive` clauses, `with` clauses, import visibility, and call behavior.
- Schema mapping can change the helper return shape and can affect whether
  encode is available.
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

## Proposal

Remove source-level `codec` and `pub codec` declarations.

Keep `schema` declarations, but describe them as binary pattern declarations:
a schema names a byte pattern that can be matched against a bounded
`ByteView`, producing a visible schema-local record, consumed byte count,
readiness, or structured failure.

The primary source operation should be an explicit schema decode expression,
not an implicit generated helper name. The exact syntax can be settled during
implementation, but the shape should read like a schema pattern application:

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

Closed-input helpers may be ordinary library functions or syntax sugar over
the same operation, but they should still cite the schema directly:

```veln
decode closed Http2SettingsPayloadWire from payload
```

The source model should not expose generated helper identifiers for schema
operations. An implementation may lower schema operations to internal helper
functions, but those helpers are not importable source names, documentation
targets, or public API.

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

Schemas should return schema-local visible records. Projection into domain
records or ADTs should be written as ordinary Veln functions:

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

This aligns with [Remove Schema Map To](remove-schema-map-to.md): schema
syntax describes representation, while ordinary functions describe domain
meaning.

## Encoding

Removing `codec` should not remove schema-backed encoding. It should remove
the named codec wrapper and the implicit API selection rules.

For direct schema-local values, use an explicit schema encode operation:

```veln
encode Http2FrameHeaderWire from header_wire
```

The operation returns a representation-specific result such as
`Result<ByteChunk, EncodeError>` or an encode-step shape chosen by the
surrounding API. It is accepted only when the schema can reconstruct every
visible field and satisfy reserved, fixed, length, repeat, and dispatch facts
from the supplied schema-local value.

Stateful, budgeted, streaming, canonicalizing, or domain-value encoders should
be ordinary functions. They may call the explicit schema encode operation
after projecting their domain value into the schema-local visible record:

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

Representation failures should use schema-owned diagnostic ids when the failed
fact belongs to a schema pattern:

- truncated input for a declared field
- reserved or fixed field mismatch
- length-bounded payload range failure
- repeat count mismatch
- dispatch tag mismatch for closed schema dispatch
- schema-local encode value unrepresentability

Diagnostics currently named under `codec.*` should be reclassified when their
failed fact is representation-local. For example, an encode value that cannot
fit a declared binary field should be reported as a schema encode
representability failure, not as a codec wrapper failure.

Ordinary protocol functions remain free to produce protocol-owned diagnostics
when a decoded representation is valid bytes but invalid for HTTP/2 state,
settings, stream lifecycle, HPACK state, or recovery policy.

## HTTP/2 Sans-I/O Shape

The HTTP/2 pilot should read as an ordinary state machine that applies schema
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

## Migration

Implementation can be staged:

1. Add explicit schema decode expressions that call the same internal logic as
   the generated decode-step helper.
2. Rewrite executable examples to call schemas through explicit decode
   expressions instead of `byte_decode_step_<schema>` helper names.
3. Add explicit schema encode expressions for schema-local visible records.
4. Rewrite encode examples to use ordinary projection functions plus explicit
   schema encode expressions.
5. Remove source-visible generated helper names from documentation and public
   examples.
6. Remove parser, formatter, AST, lowering, semantic, editor-token, and
   documentation support for top-level `codec` declarations.
7. Reclassify representation-local `codec.*` diagnostics as schema-owned
   diagnostics.
8. Archive implemented codec proposal records as historical implementation
   records, not current design direction.

During migration, compatibility shims may remain inside the compiler or
runtime, but source examples should stop teaching generated helper names and
`codec` declarations once the explicit schema operation exists.

## Non-Goals

- Do not add general parser combinators to `schema`.
- Do not let schemas access protocol state, negotiated settings, effects, or
  mutable cursors.
- Do not keep `codec` as a deprecated spelling after the replacement surface
  is available.
- Do not preserve schema `map to` as the way to project into domain values.
- Do not make generated helper names part of the public source namespace.
- Do not make HTTP/2 a standard-library commitment.

## Open Questions

- Should the explicit decode operation be a new expression form, a qualified
  prelude function taking a schema witness, or a schema-specific match arm
  form?
- Should closed-input decoding return `Result<T, DecodeError>` directly or
  remain syntax sugar over `DecodeStep<T>` plus end-of-input projection?
- Should direct schema encode return `Result<ByteChunk, EncodeError>` or
  reuse `EncodeStep<()>` for consistency with budgeted encoders?
- How long should source-visible generated helper names remain accepted for
  existing executable specification cases?

## Completion Criteria

- Current specification no longer lists `codec` as a top-level declaration.
- Current specification describes schemas as representation pattern
  declarations with explicit decode and encode operations.
- Executable examples use explicit schema operations or ordinary functions,
  not source-visible generated helper names.
- HTTP/2 Sans-I/O examples decode frame headers and payloads by applying
  schema patterns inside ordinary state-transition functions.
- Parser and semantic diagnostics reject `codec` declarations with a focused
  migration message.
- Representation-local failures use schema-owned diagnostic ids.
