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
helper, and derived encode calls for schemas that are already eligible for the
generated binary schema encode helper in `../specification/execution.md`.

Define codec support for:

- remaining general decoding from `ByteView` plus an explicit input position,
  beyond the implemented generated exact-width binary schema decode-step
  helper slice in `../specification/execution.md`
- general encoding into immutable output chunks beyond the implemented
  eligible generated binary schema encode helper and derived codec encode
  slices in `../specification/execution.md`
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
rejects mapped `derive encode` clauses when the generated direction cannot
accept that target value type. Remaining checker work should reject other
directions that the named schema cannot support. For example, encoding is
unavailable when schema mapping is not total or when a field can be decoded
but cannot be reconstructed from the mapped value without an explicit encoder
body.
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
example, a derived encoder is rejected if the mapped value cannot reconstruct a
required field, fixed field, reserved field, length field, or closed dispatch
choice without extra code.

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
referenced function and returns its `DecodeStep<T>` unchanged. The implemented
hand-written encode execution boundary exposes the codec item name as an
ordinary source call that invokes the referenced encoder function with that
function's parameters and returns its `EncodeStep<TState>` unchanged. The
implemented derived decode execution slice exposes the codec item name as an
ordinary source call to the generated `byte_decode_step_<schema>` behavior
when the schema is in the currently implemented exact-width binary schema
decode-step slice. The implemented derived encode execution slice exposes the
codec item name as an ordinary source call to the generated
`byte_encode_<schema>` behavior when the schema is in the currently
implemented binary schema encode helper slice. Remaining work should extend
generated decode and encode execution beyond the currently implemented helper
slices.

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
path, without re-exporting it from the importing module.

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

The public result shape should distinguish `Encoded`, `Partial`, and
`Invalid`. The implemented source-visible shape uses
`Encoded(List<ByteChunk>)`, `Partial(List<ByteChunk>, ByteCount, TState)`, and
`Invalid(EncodeError)`. `Encoded` carries the complete output chunks.
`Partial` carries the chunks that are ready to emit, their produced
`ByteCount`, and an encoder state value that owns the remaining work. It is
used only by APIs where the caller supplied an output budget, chunk-size
limit, or sink backpressure boundary. Unbounded helper APIs may collect all
chunks and expose a simpler `Result<List<ByteChunk>, EncodeError>` wrapper.

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
generated helper's schema-local value record, invokes that helper, returns
`EncodeStep<()>`, projects `Ok(ByteChunk)` to `Encoded(List<ByteChunk>)` with
one chunk, and projects `Err(EncodeError)` to `Invalid(EncodeError)`.

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
  declaration slice, generated exact-width binary schema decode-step helper
  slice, hand-written plus eligible derived codec decode execution boundaries,
  and hand-written codec encode execution boundary.
- Examples show decode, encode, consumed byte counts, and `NeedMore` behavior.
- Codec failures include structured diagnostic data.
- Incremental examples keep only undecoded suffix bytes in parser state.
- The HTTP/2 design driver can express `decode_step` as a pure state
  transition over byte input events.
