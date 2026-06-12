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

Define a small HTTP/2 core covering:

- connection preface validation
- frame header decode and encode
- SETTINGS
- PING
- GOAWAY
- DATA
- HEADERS with opaque header-block payloads
- CONTINUATION handling only as needed to keep header-block boundaries valid
- typed protocol errors
- connection settings
- stream identifiers
- stream lifecycle
- inbound and outbound flow-control windows
- graceful shutdown state

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

## Discussion Result: Unknown Frame Handling

The first core should decode unknown HTTP/2 frame types into an explicit
unknown-frame value instead of dropping them at the codec boundary. That value
preserves the numeric frame type, flags, stream id, and bounded payload bytes
after the normal frame-header and payload-length checks succeed.

Unknown frames have no built-in protocol semantics in the initial core. The
state transition may emit an ignored-unknown event or otherwise make the value
available to the caller, but it must not treat the unknown type as a schema
dispatch failure. If current connection state imposes a rule that only a
specific known frame can appear next, that rule remains a protocol-state check
with a typed protocol error rather than a schema error.

This separates extension tolerance from state-machine ownership. Fixtures can
assert that unknown frame bytes round-trip through decoding, while long-lived
connections can discard the preserved payload as soon as the caller decides it
does not need the extension frame.

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
the offending setting item. A received SETTINGS_MAX_FRAME_SIZE value must not
be cited as the reason an incoming frame from that same peer is too large,
because it describes the peer's receive capacity for frames this endpoint may
send.

## Required Design Decisions

All design decisions listed for this proposal have discussion results above.
Later implementation may split new follow-up proposals if source syntax or
public API questions appear.

## Implemented Slice

The first ordinary-source executable slice is current behavior under
`../specification/` and `../../examples/specification/run/http2-protocol-core/`,
with command-facing diagnostic projection fixtures beside that case. It covers
chunk arrival, incomplete input that waits for more bytes, end-of-stream
truncation with pending bytes, continuation header-block assembly through a
valid final CONTINUATION frame, one continuation ordering failure, and one
incoming frame-size peer-limit failure, plus one invalid connection-state frame
kind. It keeps parser state as undecoded suffix bytes plus the next absolute
byte offset after each consumed frame, reuses the implemented frame-header
primitive, checks the active receive maximum frame size after structural header
decode, and projects typed protocol failures into stable fixture output ids,
`protocol_diagnostic` JSON details, and human related context.

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
