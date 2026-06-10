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
- [Binary Fixture Helpers](binary-fixture-helpers.md): compact binary fixtures
  for executable examples and stable diagnostic assertions.
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

The core should cover:

- connection preface validation
- frame header decode and encode
- SETTINGS
- PING
- GOAWAY
- DATA
- HEADERS as an opaque header-block payload
- CONTINUATION handling only as far as needed to keep header-block boundaries
  valid
- typed protocol errors

HPACK can start as an opaque placeholder or a deliberately small codec. The
proposal should still reserve a boundary for a later HPACK module because
header compression has independent state and security limits.

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
  length: UInt24be where length <= max_frame_size
  kind: UInt8 as FrameKind
  flags: UInt8
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

type DecodeReadiness
  Ready
  NeedMore
end

type DecodeTransition
  Step(state: DecodeState, output: List<FrameEvent>, readiness: DecodeReadiness)
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
	let next_state: DecodeState = { state with pending: next_pending, absolute_position: state.absolute_position + consumed }
	DecodeTransition::Step(next_state, [FrameEvent::Received(frame)], DecodeReadiness::Ready)
end
```

The schema describes the byte-level frame header boundary. The function
contract and implementation handle payload dispatch, input completeness, and
settings-dependent limits. The parser is modeled as a state-transition
function: it receives an immutable state value and one stream-input event, then
returns the next state together with any intermediate output and whether more
input is needed, or returns a structured error. The parser must not assume it
can observe the total length of the stream. It can only inspect the bytes
currently buffered in the state plus the newly received chunk. End-of-stream is
an explicit input event, not a missing length. After a frame is decoded, the
next state keeps only the undecoded suffix so the core can handle long-lived
connections without retaining the entire byte history.

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

- `net: read`
- `net: write`
- `time: read` for deadlines and timeouts
- `concurrency` for per-stream task handling and channels

Veln's channel-first concurrency model is a good fit for HTTP/2 multiplexing.
One connection task can decode frames and route stream events through typed
channels. Stream handlers can return response events to the connection task,
which owns frame ordering, flow control, and transport writes.

## Standard-Library Pressure

This proposal should produce concrete requirements for:

- immutable byte chunks and byte views
- explicit byte positions with checked reads
- immutable output chunks for encoding
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

## Open Questions

- Should binary schema syntax be part of the `schema` declaration, or should
  binary formats use a separate `codec schema` form?
- Should schema declarations generate both decoders and encoders, or should
  encoding require explicit functions?
- Should a schema define the internal Veln type it produces, or should it map
  into an independently declared type?
- How much dependent structure should schema support before the design becomes
  a parser language?
- Should incomplete input be represented as `Result<T, DecodeError>`, a
  separate transition type, or another incremental parsing type?
- What should the canonical names be for immutable byte chunks, byte views,
  input positions, stream-input events, and output chunks?
- How should byte slices interact with value freezing across task and channel
  boundaries?
- Should effect labels distinguish `net: listen`, `net: accept`, `net: read`,
  and `net: write`, or start with a coarser `net` label?
- Should HPACK be treated as a normal library codec, a schema-backed codec, or
  a special case because it has dynamic table state?
- Which HTTP/2 limits should be encoded as contracts, which as runtime
  settings, and which as schema validation rules?
- Should protocol errors use ordinary ADTs only, or should there be a standard
  diagnostic-producing error interface?
- Should the first server example expose application handlers as plain
  functions, stream tasks, or a small service interface?
