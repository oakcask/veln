# HTTP/2 Binary Schema Design Driver

Status: proposed

This proposal uses a minimal HTTP/2 server core as a practical design driver
for Veln schema boundaries, binary data handling, codec support, and network
effects. It does not make HTTP/2 support current language behavior.

## Problem

Veln needs a concrete external protocol target before standard-library and
schema decisions become useful. JSON-like examples show how a schema validates
named fields, but they do not stress byte order, bit-width fields,
length-prefixed payloads, tagged binary variants, or incremental decoding from
partial input.

HTTP/2 is a good pressure test because it combines:

- fixed binary frame headers
- frame-type-specific payloads
- length-dependent payload decoding
- per-connection and per-stream state machines
- flow-control counters
- concurrent streams over one transport
- structured protocol errors
- header compression through HPACK as a separate codec layer

The target is not a production Web server. The target is a small enough
protocol core that exposes which language and library boundaries Veln needs.

## Design Principle

A Veln `schema` is a boundary contract between an external representation and
a Veln value. The external representation may be bytes, text, JSON, TOML,
database rows, HTTP requests, or another format supplied outside the program.

Schemas are not aliases for internal Veln types. They define how untrusted
external data is decoded, validated, diagnosed, documented, and converted into
typed Veln values.

This proposal keeps the responsibilities separate:

- Types describe Veln values after validation.
- Schemas describe external representation boundaries.
- Contracts describe semantic conditions on values and functions.
- Effects describe interaction with external systems or concurrency.

For HTTP/2, this means frame header layout belongs at the schema and codec
boundary, while stream-state rules belong in typed state machines and
contracts.

## Discussion Result: Schema-To-Type Mapping

Schemas should map into independently declared Veln types. A schema should not
define the internal type that its decoder returns.

This keeps schema syntax focused on external layout facts such as byte widths,
reserved bits, tags, field paths, and local structural validation. It also
keeps ordinary records and ADTs as the source of truth for internal protocol
values, contracts, pattern matching, and public APIs. Multiple external
representations can map into the same Veln type, and one external schema can
also map into different internal views through explicit codec functions when a
proposal later accepts that surface.

The schema may still define schema-local field names and intermediate decoded
values for validation, diagnostics, dispatch, and mapping. Those names are not
exported as ordinary Veln type declarations unless an independent `type`
declaration says so.

## Discussion Result: Codec Direction

Schema declarations should not implicitly export both decoders and encoders.
The codec surface should make direction explicit: a codec can opt into
decoding, encoding, or both for a named schema.

This keeps schema declarations as external boundary contracts while making
executable behavior visible at the module API boundary. HTTP/2 frame headers
need both directions, so their codec can derive both from the same schema when
the mapping is total and canonical. Other boundaries may need decoding only,
or may need hand-written encoding because output construction depends on
state, canonicalization, omitted fields, reserved bits, or values that are not
present in the internal type.

The proposal should still allow schema-driven implementation of explicit codec
functions. The important boundary is source visibility: importing a schema
does not silently import every possible executable codec, and exporting a
codec says which directions callers may use.

## Discussion Result: Incomplete Input Boundary

Incomplete byte input should be represented as a dedicated codec transition,
not as `Result<T, DecodeError>`.

Incomplete input is a normal streaming state while more chunks may arrive. It
should not be reported through the same error path as malformed bytes,
unknown invalid tags, failed reserved-bit checks, or protocol-state failures.
The codec boundary should expose a transition shape with successful decode,
need-more-input, and invalid-input outcomes. A successful decode reports the
decoded value and consumed `ByteCount`; a need-more-input outcome reports the
readiness fact without consuming the undecoded suffix; an invalid-input
outcome carries a structured `DecodeError`.

End-of-stream is where a pending need-more-input state becomes a truncation
failure. That failure should use the normal diagnostic path with byte offset,
field path, expected width or length, and actual available byte count. This
keeps ordinary chunk arrival pure and restartable while still giving agents a
repairable diagnostic once the stream can no longer satisfy the schema.

One-shot helpers may adapt the transition into `Result<T, DecodeError>` for
closed byte strings, but the incremental codec API used by protocol cores must
keep incomplete input distinct from invalid input.

## Discussion Result: Binary Schema Declaration Form

Binary formats should use the normal top-level `schema` declaration, not a
specialized `codec schema` declaration form.

This keeps `schema` as the single source construct for external
representation boundaries. A binary schema differs from a JSON, TOML, row, or
request schema by an explicit `format binary` clause that selects its field
vocabulary: exact-width integers, byte-order annotations, reserved bits,
length-dependent fields, and tag dispatch. Those are schema-local
representation facts, not a different kind of top-level declaration.

The `codec` surface remains the place where executable decoding and encoding
are named, imported, exported, and made directional. Using `codec schema`
would blur that boundary by making the schema declaration look like executable
codec API rather than a boundary contract. The format selector belongs inside
the schema body so it can make representation primitives available without
importing ordinary values or implying executable codec APIs.

## Discussion Result: Schema Dependent Structure

Schema-dependent structure should be limited to representation-local
dependencies over fields decoded earlier in the same schema.

The schema language may use a prior field to size a later byte range, select a
tagged payload schema, validate a reserved or fixed field, enforce a local
payload multiple, or map schema-local fields into an independently declared
record or ADT. These dependencies are still external layout facts: they can be
checked from the current buffered bytes and the values already decoded from
those bytes.

Schema declarations should not gain general loops, recursion through runtime
values, arbitrary function calls, connection or stream state access,
negotiated settings access, mutation, or protocol recovery behavior. Those
belong in explicit codec functions, library codec state, or the HTTP/2
protocol core. The boundary keeps schemas useful for length-prefixed payloads
and dispatch without turning them into a second parser language.

The implemented first repeated-structure slice uses a bounded schema primitive
whose count comes from a prior field, with diagnostics still reported against
field paths and byte offsets. Repetition beyond that bounded primitive,
semantic lookahead, and stateful recovery should require ordinary Veln code at
the codec boundary.

## Discussion Result: Core Byte Vocabulary Names

The source-visible byte vocabulary should use a small shared name set:
`Byte`, `ByteChunk`, `ByteView`, `ByteOffset`, `ByteCount`, and `StreamInput`.

`ByteChunk` names an immutable owned sequence of bytes. The same type should
represent incoming chunks and outgoing chunks; output APIs can return a
`ByteChunk` or a list of `ByteChunk` values when output is segmented. A
separate `OutputChunk` name would imply a different value model without adding
protocol or diagnostic precision.

`ByteView` names a bounded immutable view over byte storage. It is the right
type for pending parser input and length-dependent payload windows. It remains
a normal immutable Veln value at source level; crossing a task or channel
boundary uses the ordinary value-freezing rule.

`ByteOffset` names a zero-based absolute byte offset used in diagnostics and
parser state. `ByteCount` names lengths, consumed counts, and bounded buffer
sizes. Avoid `BytePosition` as a public name for now; the offset/count split is
more precise for protocols that must report absolute locations while dropping
consumed input from the retained buffer.

`StreamInput` names the incremental input event ADT. The required variants are
`Chunk(bytes: ByteChunk)` for byte arrival and `End` for explicit
end-of-stream. The important public boundary is that end-of-stream is an
explicit event, not an absent chunk or a special zero-length `ByteChunk`.

## Discussion Result: Exact-Width Integer Boundary

HTTP/2 exact-width unsigned fields should be represented by binary schema
primitives, not by adding `UInt24` or `UInt31` as ordinary Veln numeric types.

The schema owns wire facts such as width, byte order, reserved bits, and field
paths. Decoding maps those fields into `Int` by default or into independently
declared protocol-domain types when a later mapping rule asks for that shape.
This keeps the 24-bit frame length and 31-bit stream identifier visible at the
binary boundary without making non-standard integer widths part of the general
source type system.

## Discussion Result: HTTP/2 Limit Placement

HTTP/2 limits should be split by the authority that owns the fact.

Schema validation should cover representation-local facts that are true
without consulting connection state: exact field widths, byte order, reserved
bits, fixed payload lengths, payload length multiples, tag dispatch shape, and
whether the currently buffered input contains the declared byte range. Schema
validation may refer to fields decoded earlier in the same schema, but it
should not read negotiated settings or stream state.

Runtime settings should carry peer-negotiated or locally configured limits:
maximum frame size, initial flow-control window size, maximum concurrent
streams, header-list size policy, and other SETTINGS-derived values. Incoming
frames that exceed those values are peer protocol errors reported by the
protocol core, with diagnostics pointing at the active setting or configured
limit.

Contracts should protect Veln-owned invariants and implementation obligations:
typed state constructors keep counters in range, state-transition functions
preserve stream lifecycle invariants, flow-control arithmetic uses checked
conversion, and encoding helpers do not emit frames that violate the active
local or peer limits. Contracts should not be the primary rejection path for
untrusted peer input, because those failures need protocol-error blame rather
than implementation-contract blame.

## Discussion Result: Protocol Error Diagnostics

Protocol errors should be ordinary Veln ADTs with a standard diagnostic
projection at the boundary where they are reported.

The ADT remains the source of truth for protocol logic, matching, tests, and
state transitions. For HTTP/2, variants can distinguish peer protocol errors,
connection errors, stream errors, local configuration failures, and internal
contract failures without forcing all protocols into one shared hierarchy.

The standard boundary should define how a protocol error value is converted
into diagnostic data: stable diagnostic id, primary byte offset or source
span when available, human primary message for the failed fact, and related
notes for stream id, frame kind, current state, active settings, configured
limits, and rule provenance. This keeps agent-facing diagnostics stable while
letting each protocol keep its domain-specific error vocabulary.

The diagnostic projection should be explicit rather than automatic. Returning
a protocol ADT from pure core functions should not by itself emit a diagnostic;
the caller decides whether to recover, send a protocol response, close a
connection, or report the error to a command, fixture, or test harness.

## Discussion Result: Network Effect Labels

The first transport integration should use the existing coarse `net` effect
label rather than introduce access-mode labels such as network listen, accept,
read, and write.

The current source surface exposes effect labels as simple names, and `net` is
already a reserved public boundary label. Splitting it before socket APIs exist
would commit the language to effect taxonomy and syntax before there are enough
standard-library calls, diagnostics, or examples to justify the distinction.
The HTTP/2 design driver should stay pure for the sans-I/O slice, and later
transport code can distinguish listen, accept, read, and write through function
names, typed capabilities, and diagnostics while still carrying the single
`net` effect.

A later proposal may revisit finer-grained network effects if the runtime needs
static permission separation between network operations. That work should come
with concrete APIs and compatibility rules instead of being decided by the
HTTP/2 binary schema slice.

## Discussion Result: HPACK Boundary

HPACK should be treated as a normal library codec with explicit state, not as a
schema-backed codec or a language-level special case.

The frame schema should stop at the HEADERS and CONTINUATION payload boundary:
it can validate frame header layout, payload length, flags, stream id rules
that are local to the frame representation, and header-block byte ranges. It
should pass the header block as a `ByteView` or `ByteChunk` to an HPACK codec
rather than try to model the HPACK dynamic table inside schema declarations.

HPACK has protocol-owned mutable state in the HTTP/2 sense: dynamic table
contents, table size updates, header-list limits, and decoding context depend
on earlier header blocks on the same connection. In Veln source that state
should still be represented immutably as codec state values returned from
decode and encode transitions. That keeps it aligned with the codec execution
boundary while avoiding a schema language that can express arbitrary
state-machine behavior.

The first HTTP/2 core uses a deliberately small HPACK fixture codec after
completed HEADERS or final CONTINUATION assembly. The implemented fixture
boundary is an imported ordinary source module with explicit immutable state,
ordinary header-list data, and a stable `hpack.fixture.*` diagnostic path.
Future HPACK work may use binary schema primitives for stateless substructures
if useful, but schema support is not required to make HPACK part of the HTTP/2
design driver.

## Discussion Result: First Server Handler Shape

The first server example should expose application behavior as plain handler
functions, not as stream tasks or a service interface.

A plain function keeps the design driver focused on the protocol and codec
boundaries: the core can deliver a decoded request-like stream event plus
explicit application state, and the handler can return response actions or a
next application state. That shape is easy to run in fixture tests, does not
require sockets, and does not commit the language to a server framework before
transport APIs exist.

Stream tasks remain an implementation strategy for a future transport adapter.
The adapter may spawn one task per active stream, route events through
channels, and call the plain handler from those tasks. That keeps concurrency
ownership in the adapter that already owns ordering, flow-control backpressure,
and transport writes.

A small service interface is also deferred. It may become useful once routing,
middleware, deadlines, cancellation, and per-connection resources have concrete
standard-library shapes, but the HTTP/2 binary schema slice should not define
that interface first. If a later proposal introduces a service abstraction, it
should adapt the plain handler shape instead of replacing the protocol core.

## Discussion Result: Unknown Frame Preservation

HTTP/2 unknown frame types should be preserved by decoding and made available
to the protocol core as explicit unknown-frame values. The value should include
the numeric frame type, flags, stream id, and the bounded payload bytes selected
by the frame length. The schema dispatch is therefore extension-tolerant for
frame types, while still reporting structural failures for truncated frames,
invalid reserved fields, or payload ranges that cannot be read.

The protocol core may ignore an unknown frame after decoding because the first
slice does not define extension semantics. Ignoring is a state-transition
choice, not a codec behavior: fixture tests and later extension work should be
able to observe the unknown frame before it is discarded. If the connection is
in a state where only a specific known frame is legal, that violation is a
typed protocol-state failure with related context, not an unknown-tag schema
failure.

This result keeps extensible binary dispatch general enough for protocols
beyond HTTP/2. Closed dispatch remains available for formats where an unknown
tag is invalid, but HTTP/2 frame dispatch should opt into preservation.

## Goals

- Define the minimum binary data vocabulary needed for protocol work:
  `Byte`, immutable byte chunks and views, input positions, output chunks,
  stream-input events, byte-order reads, and checked integer conversion.
- Explore binary schema support for bit widths, endian-aware integers,
  length-prefixed payloads, tagged dispatch, reserved bits, and validation
  diagnostics.
- Separate schema declarations from codec execution so a schema can drive
  decoding, encoding, documentation, diagnostics, and test data generation.
- Model HTTP/2 connection and stream behavior with ordinary Veln ADTs,
  records, `Result`, `Option`, `match`, contracts, and channel-first
  concurrency.
- Identify which effect labels are needed for a future network runtime, while
  keeping the first protocol core sans-I/O.

## Required Proposal Elements

The design driver is not directly implementable as one code change. It depends
on smaller proposals that define the language and library elements the driver
needs:

- [Schema Declaration Surface](schema-declaration-surface.md): top-level
  schema syntax, ownership, imports, validation clauses, and mapping into Veln
  values.
- [Binary Data Standard Library](binary-data-standard-library.md):
  source-visible byte chunks, views, offsets, counts, checked reads, writes,
  and conversions.
- [Binary Schema Primitives And Dispatch](binary-schema-primitives-and-dispatch.md):
  exact-width fields, endian-aware fields, reserved bits, length-dependent
  payloads, tag dispatch, and unknown tag preservation.
- [Codec Execution Boundary](codec-execution-boundary.md): decode and encode
  APIs, consumed byte counts, incremental readiness, and immutable codec state.
- [Schema And Protocol Diagnostics](schema-and-protocol-diagnostics.md):
  byte offsets, field paths, incomplete-input reports, invalid-input reports,
  and protocol-state context.
- [Binary Fixture Helpers](../reference/implemented-proposals/binary-fixture-helpers.md):
  compact binary fixtures for executable examples and stable diagnostic
  assertions.
- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): the
  concrete frame and state-machine slice that exercises the preceding design
  elements.
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  the later route from the pure protocol core to host transport effects,
  deadlines, channels, and stream tasks.

## Non-Goals

- Do not implement TLS, ALPN, socket listeners, or platform networking in this
  proposal.
- Do not require full HPACK support before learning from the frame layer.
- Do not commit to a final binary schema syntax.
- Do not make HTTP/2 a standard-library commitment.
- Do not optimize for production throughput or memory layout.
- Do not encode all protocol state rules inside schema declarations.

## Target Slice

The first target is a sans-I/O core that accepts an open-ended stream of
incoming byte chunks and emits outgoing byte chunks without opening sockets. A
host or later network library can feed it transport chunks from a socket or
another source whose total length is not known in advance.

The core already covers client connection preface validation, frame header
decode and encode, SETTINGS maximum-frame-size state, PING, GOAWAY, DATA
receive-window accounting, HEADERS opaque header-block payload preservation,
CONTINUATION handling needed to keep header-block boundaries valid, and typed
protocol errors for those slices in the ordinary-source protocol-core example.
Remaining target coverage includes broader SETTINGS, stream lifecycle,
outbound flow control, graceful shutdown interactions, and typed protocol
errors beyond the implemented frame and stream rules.

HPACK starts outside the frame schema at a deliberately small library codec
boundary. The reserved boundary is an explicit codec module because header
compression has independent state and security limits.

## Illustrative Shape

The following examples are design sketches. The Veln code blocks use current
source syntax where they model ordinary Veln values and functions. The schema
block remains proposed schema notation because `schema` is not accepted source
syntax.

```veln
type FrameKind
	Data
	Headers
	Settings
	Ping
	Goaway
	Continuation
	Unknown(code: Int)
end
```

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

```text
type DecodeState
  phase: DecodePhase
  pending: ByteView
  absolute_position: ByteOffset
  settings: PeerSettings
end

type StreamInput
  Chunk(bytes: ByteChunk)
  End
end

type DecodeTransition
  Step(state: DecodeState, output: List<FrameEvent>)
  NeedMore(state: DecodeState)
  Fail(error: DecodeError)
end
```

```text
fn decode_step(state: DecodeState, input: StreamInput) -> DecodeTransition
	let pending: ByteView = append_chunk(state.pending, input)?
	let header: FrameHeader = decode_http2_frame_header(pending, 0)?
	let payload: ByteView = bytes_view(pending, frame_header_size, header.length)?
	let frame: Frame = decode_payload(header, payload, state.settings)?
	let consumed: ByteCount = frame_header_size + header.length
	let next_pending: ByteView = bytes_drop(pending, consumed)
	let next_state: DecodeState = {
		state with
		pending: next_pending,
		absolute_position: state.absolute_position + consumed
	}
	DecodeTransition::Step(next_state, [FrameEvent::Received(frame)])
end
```

The schema describes the byte-level frame header boundary. The function
contract and implementation handle payload dispatch, input completeness, and
settings-dependent limits. The parser is modeled as a state-transition
function: it receives an immutable state value and one stream-input event, then
returns the next state together with any intermediate output, returns a
need-more-input transition, or returns a structured error. The parser must not
assume it can observe the total length of the stream. It can only inspect the
bytes currently buffered in the state plus the newly received chunk.
End-of-stream is an explicit input event, not a missing length. After a frame
is decoded, the next state keeps only the undecoded suffix so the core can
handle long-lived connections without retaining the entire byte history.

## Schema Boundary

Binary schema support should be evaluated against these HTTP/2 requirements:

- exact-width unsigned integers such as 24-bit length fields
- endian-aware numeric reads and writes
- reserved bits that are consumed but not exposed as ordinary data
- flags that may decode into a bitset or frame-specific ADT
- payload fields whose length comes from a preceding field
- dispatch from a tag field to frame-specific payload schemas
- validation errors with byte offsets and field paths
- unknown frame types that preserve raw payload bytes when allowed

The schema layer should report local structural failures, such as a truncated
frame header or invalid fixed field. It should not decide whether a DATA frame
is legal for the current stream state.

## Codec Boundary

The proposal should distinguish the declaration from the executable codec.

`schema` provides the contract. `codec` performs decoding and encoding against
bytes. This keeps the source model small while leaving room for streaming,
partial input, and format-specific libraries.

The codec layer needs at least:

- decode from immutable byte chunks held in parser state plus an explicit input
  position
- encode into immutable output chunks
- report consumed byte count
- preserve undecoded buffered bytes across calls
- distinguish incomplete input from invalid input
- produce structured diagnostics usable by tests and agents

Cursor-like behavior should be vocabulary for state values, not a mutable data
structure. Advancing input means returning a new state with a later byte offset.
Likewise, encoding should return output chunks or an updated encoder state
rather than mutate a byte builder in place.

Because stream length is unknown, codecs should separate absolute byte offsets
used for diagnostics from the bounded buffer of undecoded bytes. Consumed input
can be dropped from the next state once no pending schema or protocol rule
needs it.

## Protocol State

Frame-level decoding is not enough. HTTP/2 behavior depends on connection and
stream state.

The protocol core should model:

- connection settings
- stream identifiers
- stream lifecycle
- inbound and outbound flow-control windows
- header-block continuation state
- graceful shutdown state

These rules should be expressed with Veln types and contracts rather than
schema declarations. For example, a schema can decode a frame with stream id
zero, but the connection state machine decides which frame kinds may use it.

## Effects And Concurrency

The first target is sans-I/O, so the core decoding and state-transition
functions should be pure unless they use concurrency for internal scheduling.

Future transport integration likely needs effect labels such as:

- `net` for socket listen, accept, read, and write operations
- `time` for deadlines and timeouts
- `concurrency` for per-stream task handling and channels

Veln's channel-first concurrency model is a good fit for HTTP/2 multiplexing.
One connection task can decode frames and route stream events through typed
channels. Stream handlers can return response events to the connection task,
which owns frame ordering, flow control, and transport writes.

## Standard-Library Pressure

This proposal should produce concrete requirements for:

- immutable byte chunks and byte views
- explicit byte positions with checked reads
- checked reads and writes for schema-owned exact-width unsigned fields
- immutable byte chunks for encoding output
- stream-input events for chunk arrival and end-of-stream
- integer conversions with overflow diagnostics
- bounded buffers for flow control
- parser or codec combinators
- structured protocol errors
- binary fixture helpers for tests

Any standard-library additions should be justified by the HTTP/2 slice and
kept general enough for other binary protocols.

## Diagnostics

Diagnostics should make boundary failures repairable by agents.

For schema and codec failures, diagnostics should include:

- byte offset
- schema field path
- expected width or length
- actual available byte count when input is incomplete
- decoded tag value for unknown or invalid dispatch
- related setting or limit when a configured limit is violated

For protocol-state failures, diagnostics should include:

- current connection or stream state
- received frame kind
- stream id
- violated contract or state-transition rule
- whether the failure is a peer protocol error or an implementation contract
  failure

## Completion Criteria

This proposal is complete enough to promote only when:

- a specification page defines the accepted schema boundary role
- examples show binary schema decoding into Veln values
- examples show HTTP/2 frame fixtures for valid and invalid input
- the standard-library surface needed by the examples is specified
- diagnostics for schema and protocol-state failures are covered
- open questions that affect source syntax or public APIs are resolved or
  split into follow-up proposals
