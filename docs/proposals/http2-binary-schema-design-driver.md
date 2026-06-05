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
  `Byte`, `Bytes`, slices, cursors, builders, byte-order reads, and checked
  integer conversion.
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

## Non-Goals

- Do not implement TLS, ALPN, socket listeners, or platform networking in this
  proposal.
- Do not require full HPACK support before learning from the frame layer.
- Do not commit to a final binary schema syntax.
- Do not make HTTP/2 a standard-library commitment.
- Do not optimize for production throughput or memory layout.
- Do not encode all protocol state rules inside schema declarations.

## Target Slice

The first target is a sans-I/O core that accepts incoming bytes and emits
outgoing bytes without opening sockets. A host or later network library can
feed it transport chunks.

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

The following examples are design sketches, not accepted Veln syntax.

```veln
type FrameKind =
  | Data
  | Headers
  | Settings
  | Ping
  | Goaway
  | Continuation
  | Unknown(code: Int)

type FrameHeader = {
  length: Int,
  kind: FrameKind,
  flags: Int,
  stream_id: StreamId,
}
```

```veln
schema Http2FrameHeader
  length: UInt24be where length <= max_frame_size
  kind: UInt8 as FrameKind
  flags: UInt8
  stream_id: UInt31be
end
```

```veln
fn decode_frame(input: Bytes, settings: PeerSettings)
  -> Result<{frame: Frame, rest: Bytes}, DecodeError>

  header = codec.decode<Http2FrameHeader>(input)?
  payload = input.slice(frame_header_size, header.length)?
  frame = decode_payload(header, payload, settings)?
  Ok({frame: frame, rest: input.drop(frame_header_size + header.length)})
end
```

The schema describes the byte-level frame header boundary. The function
contract and implementation handle payload dispatch, input completeness, and
settings-dependent limits.

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

- decode from `Bytes` or a cursor
- encode into a builder
- report consumed byte count
- preserve undecoded tail bytes
- distinguish incomplete input from invalid input
- produce structured diagnostics usable by tests and agents

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

- `Bytes` and immutable byte slices
- byte cursors with checked reads
- byte builders for encoding
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
  separate `DecodeStep<T>`, or another incremental parsing type?
- Should `Bytes` be only immutable, or should Veln also expose a mutable
  byte-builder type?
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
