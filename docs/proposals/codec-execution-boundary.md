# Codec Execution Boundary

Status: proposed

This proposal separates schema declarations from executable decoding and
encoding. It is a prerequisite for the HTTP/2 binary schema design driver
because incremental protocol parsing needs explicit readiness, consumed byte
counts, and state transitions.

## Problem

A schema can describe an external representation boundary, but HTTP/2 parsing
also needs executable behavior:

- decode from the bytes currently buffered
- report how many bytes were consumed
- distinguish incomplete input from invalid input
- preserve undecoded suffix bytes
- encode typed values into output chunks
- attach byte offsets and field paths to failures

Treating schema declarations as mutable cursors would make the source model
larger and less consistent with Veln's immutable value style.

## Scope

The source-surface declaration slice is implemented in
`../specification/source-surface.md`: top-level `codec` and `pub codec`
items preserve explicit `decode` and `encode` directions plus `derive` and
`with` body clauses. The implemented execution slice covers hand-written
decode calls, hand-written encode calls, derived decode calls for schemas that
are already eligible for the generated exact-width binary schema decode-step
helper, including supported middle reserved layouts, and derived encode calls
for schemas that are already eligible for the generated binary schema encode
helper, including the checked non-HTTP composite helper shape, selected
structural mapping encode slice, and caller-owned parser-state retention
around `Decoded` and `NeedMore` in `../specification/execution.md`. The
implemented derived decode boundary also covers generated-helper-backed
schemas with quotient-sized `ByteView(left_length / right_length)` and
product-sized `ByteView(left_length * right_length)` payload fields, plus
additive `ByteView(left_length + right_length)` and subtractive
`ByteView(left_length - right_length)` payload fields. The implemented
derived encode boundary covers those same generated-helper-backed `ByteView`
payload field shapes. The implemented derived codec boundary also covers the
narrow `ReservedBits(9, 0)` plus `UInt8` two-byte prefix helper route, with
successful decode, short-input `NeedMore`, non-consuming reserved-bit
`Invalid` projection, JSON diagnostic projection, and encode helper behavior.
The
implemented hand-written decode boundary also covers a bounded `ByteView` plus
caller-supplied base `ByteOffset` example that returns `Decoded` with a
consumed `ByteCount`, returns non-consuming `NeedMore` for short input, and
reports malformed input at the absolute offset derived from the base offset
plus the local byte position.
The eligible derived codec decode and encode execution boundaries now also
cover same-module recursive closed and extension dispatch payload helpers
already accepted by the generated helper path, and the checked non-HTTP
general generated helper shape with successful decode, short-input
`NeedMore`, successful encode, and helper-projected encode failure. They also
cover generated-helper-backed arithmetic-count and quotient-count repeated
primitive fields, standalone visible `UInt1` through `UInt7` fields,
byte-aligned representation-only `ReservedBits(width, value)` fields through
the derived decode boundary,
visible-only packed two-byte, three-byte, four-byte, five-byte, and six-byte groups, opt-in
visible flag bitset fields, including generated-helper-backed `Flag24be` and
`Flag24le` fields, wide reserved suffix and prefix groups, and schema
mappings that call pure same-module converters with five structural
arguments. The completed
generated-helper-backed codec boundary slices are recorded in
`../reference/implemented-proposals/codec-generated-helper-boundary-slices.md`.
The implemented command-facing diagnostic boundary also covers direct
source-visible `DecodeError`, `DecodeErrorWithReason`, and `EncodeError`
result failures returned by `veln run` entries with the same structured JSON
facts used by their `DecodeStep::Invalid(...)` and `EncodeStep::Invalid(...)`
counterparts. The implemented imported hand-written codec slice covers
written qualified calls to `pub codec` items declared in another module for
both `decode with` and `encode with`, without requiring the importing module
to expose the private helper function or schema. The imported hand-written
decode boundary preserves `NeedMore(NeedEnd)` for source-visible observation
and closed-input command projection. It also keeps private imported codecs
unavailable. The completed slice is recorded in the
[implemented proposal record](../reference/implemented-proposals/codec-imported-hand-written-boundary.md).
The implemented imported derived codec slice covers written qualified calls to
`pub codec` items declared in another module for both `derive decode` and
`derive encode`, including budgeted derived encode calls, using the generated
helper-backed behavior owned by the declaring module without exposing the
private schema or generated helper. It also keeps private imported codecs
unavailable and keeps bare imported codec names from becoming ordinary call
targets. The completed slice is recorded in the
[implemented proposal record](../reference/implemented-proposals/codec-imported-derived-boundary.md).
The completed same-module hand-written encode resume slice is recorded in the
[implemented proposal record](../reference/implemented-proposals/codec-hand-written-encode-resume.md).
The completed same-module hand-written `NeedEnd` readiness preservation and
closed-input projection slice is recorded in the
[implemented proposal record](../reference/implemented-proposals/codec-hand-written-need-end-boundary.md).

Define codec support for:

- remaining schema-driven decoding from `ByteView` plus an explicit input
  position, beyond the implemented hand-written boundary, generated binary
  schema decode-step helper slices, and generated-helper-backed codec slices in
  `../specification/execution.md`
- general encoding into immutable output chunks beyond the implemented
  eligible generated binary schema encode helper, generated-helper-backed
  derived codec encode, same-module and imported public budgeted derived codec
  encode slices in `../specification/execution.md`, and the implemented
  hand-written partial encode preservation and resume slice archived under
  [Codec Hand-Written Encode Resume](../reference/implemented-proposals/codec-hand-written-encode-resume.md)
- consumed byte counts
- incomplete input readiness
- invalid input errors
- decoder and encoder state values
- schema-driven codec functions
- structured diagnostics suitable for tests and agents

## Discussion Result: Incomplete Input

Incomplete input should be a dedicated codec transition outcome, not a
`DecodeError` inside `Result<T, DecodeError>`.

The incremental decode boundary should distinguish at least three outcomes:
decoded value with consumed `ByteCount`, need-more-input with the undecoded
suffix retained by the caller's state, and invalid input with a structured
`DecodeError`. Need-more-input is expected during normal streaming and should
not consume bytes or take the diagnostic failure path while more
`StreamInput` chunks may still arrive.

When end-of-stream arrives while a decoder is waiting for more bytes, the
caller converts that pending readiness into a truncation diagnostic. Closed
byte-string helpers may wrap this behavior in `Result`, but the protocol-core
API needs the transition shape so it can remain restartable after every chunk.

## Discussion Result: Decode Transition Names

The initial source-visible vocabulary is implemented in
`../specification/names-effects.md`: ordinary source can construct and match
`DecodeStep<T>`, `DecodeReadiness`, and `DecodeError`. Remaining codec
execution work should use `DecodeStep<T>` as the incremental decoder return
shape. The public transition variants are `Decoded`, `NeedMore`, and
`Invalid`.

`Decoded` carries the decoded value and the consumed `ByteCount`. It does not
mean the surrounding stream is complete; it only means this decoder accepted
one value from the current buffered input. The caller or decoder state remains
responsible for retaining the undecoded suffix.

`NeedMore` carries a `DecodeReadiness` value and consumes no input.
`DecodeReadiness` should start with `NeedBytes` for decoders that can name the
minimum buffered `ByteCount` required before retrying, and `NeedEnd` for
decoders that cannot decide until the caller supplies an explicit
end-of-stream event. HTTP/2 frame decoding normally uses `NeedBytes`; an
end-of-stream event while `NeedBytes` is pending becomes a truncation
diagnostic at the reporting boundary.

`Invalid` carries a structured `DecodeError`. It is reserved for malformed
input or schema structural failures, not for ordinary streaming backpressure.

This keeps `DecodeStep<T>` distinct from `Result<T, DecodeError>` while using
short variant names that read naturally in `match` expressions. Encoding
should not reuse `DecodeStep`; encode APIs can define their own result shape
because partial output and invalid input have different ownership rules.

## Discussion Result: Absolute Offsets And Bounded Buffers

Incremental decoders should receive a bounded `ByteView` plus an explicit
base `ByteOffset`. The view is the bytes the decoder may inspect on this
step. The offset is the absolute position of the first byte in that view and
is used only for diagnostics, consumed-position accounting, and protocol
state that needs stable byte locations.

Codec functions should report local positions and lengths with `ByteCount`
values. A diagnostic projection combines the caller-supplied base offset with
the local field position to produce the absolute `ByteOffset` reported to
humans, JSON output, fixtures, and agents. Dropping consumed bytes therefore
does not change the location assigned to a previously observed field or
failure.

The decoder must not infer absolute position from the retained buffer length,
and `ByteView` must not carry hidden global cursor state. The caller owns the
parser state that pairs the undecoded suffix with the next base offset. After a
`Decoded` outcome, the caller may drop the consumed prefix and advance the
base offset by the consumed `ByteCount`. After `NeedMore` or `Invalid`, the
caller keeps the same base offset unless it intentionally reports or recovers
through a higher-level protocol rule.

## Discussion Result: Consumed Input Retention

Consumed input should be dropped by the caller that owns parser state, not by
the codec function itself. A successful decode returns the decoded value and a
`ByteCount`; the caller validates that the count is within the supplied
`ByteView`, derives the next pending view by dropping that prefix, and advances
the explicit base `ByteOffset` by the same count.

Values returned from a codec must not depend on an implicit cursor into bytes
that the caller is expected to discard. If the decoded value needs to retain
wire bytes, such as an opaque extension payload or header-block fragment, that
retention must be source-visible as a bounded `ByteView` or `ByteChunk` field
in the returned value. The runtime may preserve the backing bytes by sharing,
pinning, or copying the bounded range, but the source-level rule is that only
the retained value's byte range survives. The rest of the consumed prefix is
not kept alive by the codec boundary.

`NeedMore` consumes no bytes and therefore cannot advance the caller's pending
view or base offset. `Invalid` also consumes no bytes at the codec boundary;
any recovery that skips bytes belongs to a higher-level protocol rule with its
own diagnostic context. Diagnostics remain stable after the caller drops
consumed input because they carry absolute byte offsets and field paths rather
than references to the old buffer prefix.

## Discussion Result: Explicit Codec Directions

The source-surface direction list is implemented in
`../specification/source-surface.md`. Codec declarations name exactly one
schema and list `decode`, `encode`, or both in the declaration head.

The direction list is the source-visible opt-in boundary. For the implemented
`decode with` and eligible `derive decode` slices, a `decode` direction exposes
the codec item as a decoder for values produced from that schema, with
`DecodeStep<T>` readiness and consumed-count behavior. For the implemented
`encode with` slice, an `encode` direction exposes the codec item as a call to
the referenced ordinary encoder function and returns its `EncodeStep<TState>`
unchanged. A declaration that lists both directions may share schema-derived
checks and mapping facts, but each direction still has its own result shape
and diagnostics.

The checker accepts mapped `derive decode` clauses when the generated
decode-step helper can expose the schema mapping target value type, and
accepts mapped `derive encode` clauses when the generated direction can
project that target value back to schema-local encode fields. The checker
rejects other directions that the named schema cannot support with
`codec.derive_helper_unsupported` before the codec item becomes callable. For
example, encoding is unavailable when schema mapping is not total or when a
field can be decoded but cannot be reconstructed from the mapped value without
an explicit encoder body.
Importing or exporting the codec declaration must not silently add directions
that are missing from its head.

## Discussion Result: Codec Body Form

The source-surface body clause forms are implemented in
`../specification/source-surface.md`: `derive decode`, `derive encode`,
`decode with <function>`, and `encode with <function>`. The checker implements
the first hand-written function boundary slices: `decode with` references must
name ordinary same-module functions whose parameters are `ByteView` and
`ByteOffset` and whose return shape is `DecodeStep<T>`, while `encode with`
references must name ordinary same-module functions whose return shape is
`EncodeStep<TState>`. When the referenced schema has one implemented
structural mapping, the checker also requires the `DecodeStep<T>` value type
and the encoder function's first value parameter to match that mapping target
record shape.

`derive` asks the checker to generate that direction from the named schema,
using the schema mapping and validation rules. A derived direction is accepted
only when the schema has enough information for the requested operation. For
example, a derived encoder is rejected if the mapped value cannot reconstruct
a required visible field, length field, or closed dispatch choice without
extra code.

`with` binds a direction to an ordinary source function. The function remains a
normal top-level item with its own visibility, contracts, effects, tests, and
documentation. The implemented decode checker verifies the canonical boundary
shape for hand-written decoders: a bounded `ByteView` plus base `ByteOffset`
and a `DecodeStep<T>` return. For the implemented structural mapping slice, the
decoded `T` must match the mapped target record shape. The implemented encode
checker verifies the hand-written encoder result boundary as
`EncodeStep<TState>` and, for the same mapping slice, verifies that the first
encoder parameter is the mapped target record shape. The implemented
hand-written decode execution boundary exposes the codec item name as an
ordinary source call that forwards `ByteView` and `ByteOffset` to the
referenced function, returns valid `Decoded`, `NeedMore`, and `Invalid`
results unchanged, and projects an oversized consumed count to
`codec.consumed_count_invalid`. The completed same-module hand-written
`NeedEnd` readiness preservation and closed-input projection slice is archived
under
[Codec Hand-Written NeedEnd Boundary](../reference/implemented-proposals/codec-hand-written-need-end-boundary.md).
The implemented hand-written encode execution
boundary exposes the codec item name as an ordinary source call that invokes
the referenced encoder function with that function's parameters and returns
its `EncodeStep<TState>` unchanged, including `Partial` values with emitted
chunks, produced counts, and resumed state preserved as ordinary
source-visible values. The same-module hand-written encode resume slice is
archived under
[Codec Hand-Written Encode Resume](../reference/implemented-proposals/codec-hand-written-encode-resume.md).
The implemented derived decode execution slice exposes the codec item name as an
ordinary source call to the generated `byte_decode_step_<schema>` behavior
when the schema is in the currently implemented generated binary schema
decode-step slice, including same-module nested dispatch payload helper
schemas, public imported nested dispatch payload helper schemas,
repeat-backed schemas, arithmetic-count and quotient-count repeated primitive
fields, supported middle reserved layouts, and the checked non-HTTP general
helper shape, plus additive, subtractive, quotient-sized, and product-sized
`ByteView` payload fields, standalone visible `UInt1` through `UInt7` fields,
byte-aligned representation-only `ReservedBits(width, value)` fields,
visible-only packed two-byte, three-byte, four-byte, five-byte, and six-byte groups, the
narrow `ReservedBits(9, 0)` plus `UInt8` two-byte prefix helper route, and
schema mappings that call pure same-module converters with five structural
arguments.
The implemented
derived encode execution slice exposes
the codec item name as an ordinary source call to the generated
`byte_encode_<schema>` behavior when the schema is in the currently
implemented binary schema encode helper slice, including direct structural
mapped schemas, same-module nested dispatch payload helper schemas, public
imported nested dispatch payload helper schemas, repeat-backed schemas,
arithmetic-count and quotient-count repeated primitive fields, and the checked
non-HTTP general helper shape, plus additive, subtractive, product-sized, and
quotient-sized `ByteView` payload fields, standalone visible `UInt1` through
`UInt7` fields, and visible-only packed two-byte, three-byte, four-byte, five-byte, and six-byte
groups, and the narrow `ReservedBits(9, 0)` plus `UInt8` two-byte prefix
helper route.
Remaining work should extend generated decode and encode execution beyond the
currently implemented helper slices.

The implemented parser rejects a missing implementation clause for a listed
direction, a body clause for a direction absent from the declaration head, and
duplicate implementation clauses. The checker rejects a derived function whose
decoded or encoded value type does not match the schema mapping. Keeping
hand-written logic in ordinary functions avoids nested function syntax inside
`codec` while still giving modules one named codec item for imports, exports,
fixtures, and diagnostics.

## Discussion Result: Codec Names And Imports

The source model now preserves named top-level codec items and their
visibility. The declaration name owns the executable codec boundary for one
schema plus its explicit direction list; it is not derived from the schema
name, and it does not synthesize separate top-level decoder or encoder
functions.

The implemented hand-written encode, hand-written decode, and eligible derived
decode and encode call paths make a private codec usable only in its declaring
module. A `pub codec` is exposed through a written import-qualified module
path, without re-exporting it from the importing module. The imported
hand-written `decode with` path forwards the caller's `ByteView` and
`ByteOffset` to the decoder named in the declaring module. The imported
hand-written `encode with` path forwards the source call arguments to the
encoder named in the declaring module. Imported derived codec calls invoke the
declaring module's generated decode-step or encode helper through the public
codec item, so the schema and helper ownership stay in the declaring module.

Importing a codec imports the codec item only. It does not import the schema as
an ordinary value, expose schema-local field names, or add codec directions that
were not listed in the declaration head. Direction-specific operations are
selected through the codec item and its declared directions, so facade modules
can choose which codec item they publish without creating accidental aliases for
generated decode or encode entry points.

## Discussion Result: Encode Output And Failures

The initial source-visible encode vocabulary is implemented in
`../specification/names-effects.md`: ordinary source can construct and match
`EncodeStep<TState>` and `EncodeError`. Remaining codec execution work should
use this encode-specific result shape, not reuse `DecodeStep<T>`, and not
mutate a caller-owned byte builder.

The implemented public result shape distinguishes `Encoded`, `Partial`, and
`Invalid`. The source-visible shape uses
`Encoded(List<ByteChunk>)`, `Partial(List<ByteChunk>, ByteCount, TState)`, and
`Invalid(EncodeError)`. `Encoded` carries the complete output chunks.
`Partial` carries the chunks that are ready to emit, their produced
`ByteCount`, and an encoder state value that owns the remaining work. It is
used only by APIs where the caller supplied an output budget, chunk-size
limit, or sink backpressure boundary. Unbounded helper APIs may collect all
chunks and expose a simpler `Result<List<ByteChunk>, EncodeError>` wrapper.
The implemented hand-written encode boundary preserves all three outcomes
from the referenced encoder function unchanged. The completed same-module
hand-written encode resume slice is archived under
[Codec Hand-Written Encode Resume](../reference/implemented-proposals/codec-hand-written-encode-resume.md).

`Invalid` carries a structured `EncodeError` for values that cannot be
represented by the codec: out-of-range exact-width fields, failed fixed or
reserved-field requirements, non-total schema mappings, length mismatches, or
unknown values in a closed dispatch. The encoder should validate these
representation facts before it exposes the first output chunk for a value, so
a caller that observes `Partial` can treat the returned chunks as committed
valid output for that encode operation.

The implemented derived encode execution slice in
`../specification/execution.md` covers eligible binary schemas that already
expose `byte_encode_<schema>` helpers. The codec item call accepts the
generated helper's schema-local value record or direct mapping target record,
invokes that helper, returns `EncodeStep<()>`, projects `Ok(ByteChunk)` to
`Encoded(List<ByteChunk>)` with one chunk, and projects `Err(EncodeError)` to
`Invalid(EncodeError)`. The implemented budgeted derived encode boundary,
including the written import-qualified `pub codec` path, accepts the same
value record plus an explicit `ByteCount` output budget. It returns complete
output as `Encoded`, returns oversized output as `Partial` with the committed
prefix, produced count, and a state record carrying `encoded_offset`, resumes
when that state record is passed to the same codec with a later budget, and
preserves helper `Err(EncodeError)` projection to `Invalid` before exposing
any output chunk.

After `Partial`, resuming uses the returned encoder state rather than an
implicit mutable cursor. The state must not borrow from a caller-owned builder
or depend on bytes the caller has already emitted. This keeps output ownership
parallel to decode input ownership: the caller owns emitted chunks and the
encoder state owns only the remaining encode work.

## Non-Goals

- Do not define the schema source syntax itself.
- Do not define protocol state machines.
- Do not require a socket or asynchronous runtime.
- Do not make mutable byte builders part of the source model.

## Completion Criteria

- Remaining proposal work starts after the implemented source-surface
  declaration slice, generated binary schema decode-step helper slice for the
  implemented exact-width, same-module nested dispatch payload, and public
  imported nested dispatch payload boundaries, hand-written plus eligible
  derived codec decode execution boundaries, and hand-written plus eligible
  derived codec encode execution boundaries, including same-module
  hand-written encode resume and same-module plus imported public budgeted
  derived encode over generated helper output,
  selected structural mapping encode cases already accepted by the generated
  helper, same-module recursive closed and extension dispatch payload helpers,
  arithmetic-count and quotient-count repeated primitive fields,
  byte-aligned representation-only `ReservedBits(width, value)` fields through
  the derived decode boundary, standalone visible `UInt1` through `UInt7`
  fields, visible-only packed two-byte,
  three-byte, four-byte, five-byte, and six-byte groups, opt-in visible flag bitset
  fields, wide reserved suffix and prefix groups, generated-helper-backed
  `Flag24be` and `Flag24le` fields, the checked non-HTTP general helper
  shape, the narrow `ReservedBits(9, 0)` plus `UInt8` two-byte prefix helper
  route, and the caller-owned parser-state retention and hand-written bounded
  `ByteView` base-offset `NeedMore` examples, including same-module
  hand-written `NeedEnd` readiness preservation and closed-input projection.
- Remaining examples show decode, encode, consumed byte counts, and
  `NeedMore` behavior beyond the implemented helper slices.
- Codec failures include structured diagnostic data.
- The HTTP/2 design driver can express `decode_step` as a pure state
  transition over byte input events.
