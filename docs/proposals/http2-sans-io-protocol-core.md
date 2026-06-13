# HTTP/2 Sans-I/O Protocol Core

Status: proposed

This proposal defines the actual HTTP/2 protocol-core slice used by the binary
schema design driver. It depends on schema declarations, binary schema
primitives, byte vocabulary, codec execution, diagnostics, and binary fixture
helpers.

## Problem

The design driver needs a concrete protocol target, but the target should not
be a production Web server. The useful slice is a sans-I/O core that accepts
byte input events, returns outgoing byte chunks, and models protocol state with
ordinary Veln values.

## Scope

Define the remaining HTTP/2 core behavior beyond the implemented
ordinary-source decode-state slices. Planned coverage still includes:

- remaining SETTINGS values and settings interactions beyond the implemented
  maximum-frame-size and initial-window-size peer-advertised state and
  SETTINGS ACK receive handling
- remaining DATA behavior beyond the implemented receive-window accounting
- typed protocol errors for the remaining frame and stream rules
- connection settings beyond maximum frame size
- stream identifiers
- remaining stream lifecycle beyond the implemented peer-created stream
  admission, receive-limit, inbound reset slice, and GOAWAY last-stream-id
  enforcement for later peer-created HEADERS
- initial-window-size changes, outbound flow control, and broader
  stream-window interactions beyond the implemented inbound DATA and
  `WINDOW_UPDATE` receive-window accounting
- graceful shutdown interactions beyond the implemented GOAWAY receive state
  and later peer-created HEADERS rejection

## Discussion Result: Limit Placement

The core should treat schema validation, runtime settings, and contracts as
separate limit owners.

Schema validation owns representation-local limits that can be checked from the
current bytes and fields already decoded in the same schema, such as exact
field widths, reserved bits, fixed payload lengths, and payload length
multiples.

Runtime settings own negotiated or configured peer limits such as maximum frame
size, initial flow-control window size, maximum concurrent streams, and
header-list policy. Incoming frames that violate these limits should produce
typed peer protocol errors with diagnostic context for the active setting or
configured limit.

Contracts own Veln implementation invariants: state constructors keep counters
and stream identifiers well formed, state transitions preserve lifecycle and
flow-control invariants, and encoding helpers do not produce frames that
violate active limits. Contracts should not replace peer protocol errors for
invalid incoming frames.

## Discussion Result: Protocol Error Reporting

The HTTP/2 core should model protocol errors as ordinary ADTs and expose an
explicit conversion into diagnostic data for reporting surfaces.

Pure transition functions return typed protocol errors so callers can decide
whether to send GOAWAY, reset a stream, continue processing other streams, or
close the connection. Fixture runners, command output, and agent-facing tests
use the diagnostic projection to get stable ids, byte offsets, focused primary
messages, and related context for stream id, frame kind, current state, active
settings, configured limits, and rule provenance.

## Discussion Result: HPACK Boundary

The first core should treat HPACK as an explicit library codec boundary with
its own immutable state values. Frame schemas and frame-level decoding keep
HEADERS and CONTINUATION payloads as opaque header-block bytes, then pass those
bytes to an HPACK codec when the example needs header decoding.

This keeps dynamic table contents, table size updates, and header-list limits
out of the schema language. The initial slice may use opaque header blocks or
a deliberately small fixture codec, but it should not introduce a
schema-backed HPACK special case.

## Discussion Result: Protocol Numeric Domain Types

Stream identifiers and flow-control counters should be protocol-domain values
backed by ordinary `Int`, not new source-visible unsigned integer widths.
Binary schemas decode the external `UInt31be` representation into an `Int`.
The protocol core then validates and wraps that value at the state-machine
boundary.

Use a nonzero `StreamId` domain value for real streams and a separate
`StreamRef` shape when a frame may target either the connection or a stream.
`StreamRef` distinguishes the connection control stream from a stream id, so
stream id zero does not accidentally pass through APIs that require an actual
stream. Client-initiated and server-initiated parity rules are checked by
connection state constructors or transition functions because validity depends
on endpoint role and lifecycle state.

Flow-control values should be distinct domain values even though they share an
`Int` representation. The core should at least separate current window credit,
configured initial window size, and incoming window-update increments. Current
stream windows may become negative after a SETTINGS initial-window-size
reduction, while connection windows and advertised limits keep their own
bounds. Constructors and transition contracts own those range checks; schemas
only read the bytes.

This keeps wire layout in schema primitives, protocol meaning in ordinary Veln
types, and diagnostics precise: schema failures report malformed encoded
fields, while protocol failures report invalid stream ids, parity mismatches,
window overflow, negative-credit blocking, or limit violations with the active
connection state as related context.

## Discussion Result: Header-Block Continuation State

Header-block continuation should be represented as connection decode state, not
as schema state and not as HPACK state.

The decode state should carry an optional pending header-block assembly value.
That value records the stream id that owns the block, the frame kind that
started it, the absolute byte offset of the starting frame, the accumulated
opaque header-block byte chunks, and the accumulated `ByteCount`. The
accumulated bytes must be owned or frozen chunks because the main decode state
still drops consumed input after each frame.

When no header block is pending, a HEADERS frame with `END_HEADERS` produces a
complete opaque header-block event immediately. A HEADERS frame without
`END_HEADERS` creates the pending assembly and emits no complete header event
yet. When a block is pending, the next frame must be a CONTINUATION frame for
the same stream. A CONTINUATION without `END_HEADERS` appends its payload to
the assembly. A CONTINUATION with `END_HEADERS` completes the assembly and
hands the combined opaque chunks to the HPACK boundary or fixture codec.

Any different frame kind, different stream id, or connection end while an
assembly is pending is a peer protocol-state failure. The diagnostic should
point at the incoming frame or stream end that violated the continuation rule,
with related context for the pending stream id, starting frame kind, starting
byte offset, accumulated byte count, and rule provenance. This keeps
continuation ordering out of schemas while still making the state machine
repairable in fixtures and agent-facing output.

## Discussion Result: Peer Limit Diagnostic Ids

HTTP/2 peer protocol limit failures should use protocol-owned diagnostic ids
under the `http2.peer_limit.*` namespace. These ids name the peer-visible
limit fact that failed, while related notes carry the active setting,
configured local limit, frame kind, stream reference, current state, and rule
provenance.

The first canonical ids are:

- `http2.peer_limit.frame_size_exceeded` for an incoming frame whose payload
  length is greater than the active maximum frame size
- `http2.peer_limit.header_list_size_exceeded` for a decoded header block
  whose list size is greater than the active header-list policy
- `http2.peer_limit.concurrent_streams_exceeded` for a peer-created stream
  that would exceed the current concurrent-stream limit
- `http2.peer_limit.flow_control_window_exceeded` for DATA or WINDOW_UPDATE
  behavior that would make the receiving flow-control window exceed its
  allowed range
- `http2.peer_limit.settings_value_out_of_range` for a peer SETTINGS value
  outside the range accepted by the HTTP/2 core

These ids are not schema or codec ids. A frame with a malformed reserved bit,
truncated payload, or closed-dispatch tag failure still reports the narrower
`schema.*` or `codec.*` failure before the protocol core can evaluate peer
limits. Once the frame is structurally decoded, negotiated settings and stream
state decide whether a peer limit was violated.

The human primary message should state only the failed limit fact, such as a
frame size exceeding the active maximum. The diagnostic projection should put
the observed value, allowed value, setting name or configured limit, and peer
blame in structured details or related notes so fixtures can assert the stable
id without parsing display text.

## Discussion Result: SETTINGS State And Provenance

Decode state should split locally enforced receive limits from settings the
peer advertised for outbound frames.

The receive-limit state owns the limits used to validate incoming frames:
maximum frame size, inbound flow-control bounds, header-list policy, and any
other local setting that constrains what the peer may send. Each receive limit
stores both the active value and a small provenance value: protocol default,
local configuration, or a local SETTINGS item emitted by the core. The peer
settings state stores SETTINGS values received from the peer, with their own
frame and item offsets, but those values affect outbound encoding decisions
instead of inbound frame-size validation.

When an incoming frame payload length exceeds the active receive maximum, the
protocol error should capture a snapshot of the receive-limit entry used for
the check. The diagnostic projection for
`http2.peer_limit.frame_size_exceeded` reports the offending frame offset as
the primary location, includes observed and allowed lengths in structured
details, and attaches the receive-limit provenance as related context. If the
active value came from a local SETTINGS item, the related context points to
that item; if it came from a default or configuration value, the related
context names that source instead.

Received SETTINGS frames still update peer-advertised settings after their
payload is structurally decoded and each value passes protocol range checks.
Invalid SETTINGS values use `http2.peer_limit.settings_value_out_of_range` at
the offending setting item. Received peer-advertised SETTINGS such as
SETTINGS_MAX_FRAME_SIZE and SETTINGS_INITIAL_WINDOW_SIZE must not be cited as
the reason an incoming frame from that same peer violates this endpoint's
inbound limits, because they describe the peer's receive capacity for frames
this endpoint may send.

## Required Design Decisions

All design decisions listed for this proposal have discussion results above.
Later implementation may split new follow-up proposals if source syntax or
public API questions appear.

## Implemented Slice

The first ordinary-source executable slice is current behavior under
`../specification/` and `../../examples/specification/run/http2-protocol-core/`,
with command-facing diagnostic projection fixtures beside that case. It covers
chunk arrival, client connection preface validation before frame-header
decode, partial preface input that waits for more bytes, end-of-stream with a
partial preface, mismatched preface bytes, incomplete frame input that waits
for more bytes, end-of-stream truncation with pending frame bytes,
continuation header-block assembly through a valid final CONTINUATION frame,
the combined opaque header-block payload bytes from that completed block,
single-frame HEADERS completion when END_HEADERS is set alongside another
flag, continuation failures for a different frame kind, a different stream id,
and closed input while a header block remains pending, and one incoming
frame-size peer-limit failure, plus one invalid idle-stream frame kind and
stream id domain failures for zero, even, and connection-only stream ids. It keeps
parser state as undecoded suffix bytes plus the next absolute byte offset
after each consumed preface or frame, reuses the implemented frame-header
primitive after the preface gate, checks the active receive maximum frame size
after structural header decode, and projects typed protocol failures into
stable fixture output ids, `protocol_diagnostic` JSON details, and human
related context. The partial and invalid client connection preface projections
also include bounded protocol-owned byte previews for the raw bytes inspected
by the preface check.
It also splits the active receive-limit entry and receive-window credit from
peer-advertised SETTINGS state. The checked example keeps protocol-default,
local-configuration, and local-SETTINGS receive-limit provenance visible in
frame-size failures, stores received `SETTINGS_MAX_FRAME_SIZE` and
`SETTINGS_INITIAL_WINDOW_SIZE` values as peer-advertised state, and confirms
that those peer-advertised values are not used as inbound limits for later
incoming frame-size or DATA receive-window checks. It range-checks received
values for both settings before updating peer-advertised state and projects
out-of-range values as
`http2.peer_limit.settings_value_out_of_range` with setting identity, observed
value, accepted range, item byte offset, and peer-limit provenance in
executable output, human diagnostics, and JSON details.
It accepts zero-length SETTINGS ACK frames on the connection stream without
updating peer-advertised SETTINGS state, rejects nonzero-length SETTINGS ACK
frames as `http2.protocol.invalid_payload_length`, and keeps SETTINGS ACK on
nonzero streams on the existing `http2.protocol.invalid_stream_id` path.
It also accepts structurally complete unknown extension frames after the
client preface gate as ordinary `UnknownFrame` values preserving frame type,
flags, stream id, and bounded payload bytes, and keeps active continuation
ownership by rejecting an unknown frame with the existing continuation
protocol-state failure when CONTINUATION is required next.
It validates the stream id domain for received frame headers after structural
decode and before frame-specific state updates. In the server-side fixture
core, SETTINGS, PING, and GOAWAY require stream id zero, while HEADERS, DATA,
`RST_STREAM`, CONTINUATION, and stream-level `WINDOW_UPDATE` require a nonzero
client-initiated stream id. Domain failures use
`http2.protocol.invalid_stream_id` with frame kind, stream id, required
domain, endpoint role, active state, and rule provenance. Representation
failures for the generated `UInt31be` helper remain schema or codec failures
instead of protocol diagnostics.
The same executable example now includes the outbound frame-header encode
slice. Ordinary source builds record-shaped frame descriptions with `length`,
`kind`, `flags`, and `stream_id`, invokes the generated binary schema encode
helper for the HTTP/2 wire header layout, and checks one nine-byte output
chunk for a SETTINGS header on the connection stream, a DATA header on a
nonzero stream, and the maximum valid `UInt31be` stream id. It also keeps the
generated helper's `codec.encode_value_unrepresentable` error visible for an
out-of-range stream id instead of projecting that representation failure into
a protocol diagnostic.
It now also handles structurally decoded PING and GOAWAY frames. PING is
accepted only on the connection stream with an eight-byte payload, and the
observable output preserves the ACK flag distinction. GOAWAY is accepted only
on the connection stream with the fixed eight-byte prefix needed to expose the
last stream id and error code, then transitions the decode state into graceful
shutdown. Stream-targeted PING and GOAWAY frames are stream id domain
failures, while wrong-length PING and GOAWAY payloads use
`http2.protocol.invalid_payload_length` in ordinary output, human diagnostics,
and JSON `protocol_diagnostic` details.
The implemented slice also accepts DATA frames on an already-open stream and
decrements both connection and stream receive-window credit by the payload
length. DATA on the connection stream is a stream id domain failure, DATA on
an idle stream remains `http2.protocol.invalid_frame_kind`, and DATA payloads
that exceed the
available stream or connection receive-window credit use
`http2.peer_limit.flow_control_window_exceeded` with byte offset, stream
reference, observed payload length, allowed window credit, active state, and
rule provenance in executable output, human diagnostics, and JSON
`protocol_diagnostic` details.
The implemented slice also receives `WINDOW_UPDATE` frames. Connection-level
`WINDOW_UPDATE` increases connection receive-window credit, and
stream-level `WINDOW_UPDATE` increases the currently open stream's
receive-window credit. Wrong-length `WINDOW_UPDATE` payloads use
`http2.protocol.invalid_payload_length`, idle or unknown stream-targeted
`WINDOW_UPDATE` remains the existing stream-state
`http2.protocol.invalid_frame_kind` shape, and zero or overflowing increments
use `http2.peer_limit.flow_control_window_exceeded` without changing receive
window state.
The implemented slice also admits peer-created streams narrowly. A HEADERS
frame on an idle, nonzero stream opens the tracked peer-created stream when
the active concurrent-stream receive limit allows it. A HEADERS frame that
would open another peer-created stream beyond that receive limit fails as
`http2.peer_limit.concurrent_streams_exceeded`, with byte offset, stream
reference, attempted and allowed concurrent-stream counts, active protocol
state, receive-limit provenance, and rule provenance in ordinary output, human
diagnostics, and JSON `protocol_diagnostic` details. Non-HEADERS frames on
idle streams keep using the existing invalid frame-kind failure.
The implemented slice also receives `RST_STREAM` frames on the tracked open
peer-created stream. It decodes the four-byte error-code payload into
source-visible reset state, clears the open stream, and rejects later DATA or
stream-level `WINDOW_UPDATE` frames for that reset stream through the existing
`http2.protocol.invalid_frame_kind` path. `RST_STREAM` on stream id zero uses
the existing stream id domain failure, wrong-length `RST_STREAM` payloads use
`http2.protocol.invalid_payload_length`, and idle or unknown-stream
`RST_STREAM` frames remain stream-state invalid frame-kind failures.

The remaining scope below is still planned work for the full protocol core.

## Non-Goals

- Do not implement TLS, ALPN, socket listeners, or platform networking.
- Do not require complete HPACK support.
- Do not optimize for production throughput.
- Do not encode all protocol state rules inside schema declarations.

## Completion Criteria

- Examples show valid and invalid frame fixtures for the target slice.
- A pure decode state transition handles chunk arrival and end-of-stream.
- Protocol-state failures are typed and diagnostically structured.
- The core keeps only undecoded suffix bytes after frame consumption.
- HPACK has a reserved boundary for later work.
- The design driver can use the core to evaluate schema, byte, codec,
  diagnostic, and standard-library decisions.
