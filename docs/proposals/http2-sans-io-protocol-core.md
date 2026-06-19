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

- remaining settings interactions not covered by the implemented
  enable-push, maximum-frame-size, maximum-concurrent-streams,
  initial-window-size, header-table-size, and maximum-header-list-size
  peer-advertised state, unknown-identifier handling, SETTINGS ACK receive
  and outstanding-local SETTINGS tracking, local SETTINGS send-intents for
  header-table-size, enable-push, initial-window-size,
  maximum-concurrent-streams, maximum-frame-size, maximum-header-list-size, a
  two-item local SETTINGS batch, and the narrow outbound SETTINGS ACK
  send-intent slices
- remaining DATA behavior not covered by the implemented receive-window
  accounting, two-open-stream receive-credit boundary, inbound PADDED DATA
  handling, inbound `END_STREAM` closed-by-peer lifecycle, outbound PADDED
  DATA send-intent slice, and half-closed-local inbound DATA receive after
  local `END_STREAM`
- typed protocol errors for the remaining frame and stream rules
- connection settings beyond maximum frame size
- stream identifiers
- remaining stream lifecycle beyond the implemented peer-created stream
  admission, receive-limit, inbound reset slice, DATA and HEADERS
  `END_STREAM` closed-by-peer transitions, outbound `RST_STREAM` local
  reset send-intent slice, outbound HEADERS local closed-stream send-intent
  slice, outbound DATA local closed-stream send-intent slice, GOAWAY
  last-stream-id enforcement for later peer-created HEADERS, outbound
  HEADERS send-intent rejection above a received GOAWAY boundary, and
  half-closed-local inbound DATA receive after local `END_STREAM`
- remaining outbound flow control and broader stream-window interactions
  beyond the implemented outbound DATA send-intent splitting and PADDED DATA
  send-intent slices, outbound `RST_STREAM` reset send intent, inbound DATA,
  stream-level `WINDOW_UPDATE`, outbound `WINDOW_UPDATE` receive-credit
  intent, and
  `SETTINGS_INITIAL_WINDOW_SIZE` two-open-stream receive-window accounting
- graceful shutdown interactions beyond the implemented GOAWAY receive state,
  outbound GOAWAY send-intent state, later peer-created HEADERS rejection,
  and outbound HEADERS send-intent rejection above a received GOAWAY boundary

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
SETTINGS_ENABLE_PUSH, SETTINGS_MAX_FRAME_SIZE,
SETTINGS_MAX_CONCURRENT_STREAMS, SETTINGS_INITIAL_WINDOW_SIZE,
SETTINGS_HEADER_TABLE_SIZE, and SETTINGS_MAX_HEADER_LIST_SIZE must not be
cited as the reason an incoming frame from that same peer violates this
endpoint's inbound limits, because they describe the peer's receive capacity
for frames this endpoint may send.

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
single-frame HEADERS completion when `END_HEADERS` is set alongside
`END_STREAM`, closed-by-peer stream lifecycle after accepted HEADERS
`END_STREAM` completion through both single-frame HEADERS and final
CONTINUATION paths, continuation failures for a different frame kind, a
different stream id, and closed input while a header block remains pending,
one incoming frame-size peer-limit failure, one completed header-list-size
peer-limit failure at the fixture-codec boundary, plus one invalid
idle-stream frame kind and stream id domain failures for zero, even, and
connection-only stream ids. The stream id domain slice rejects HEADERS and
CONTINUATION on the connection stream before opening stream state or changing
header-block continuation state, including a CONTINUATION frame while a
nonzero-stream header block is pending. It keeps
parser state as undecoded suffix bytes plus the next absolute byte offset
after each consumed preface or frame, reuses the implemented frame-header
primitive after the preface gate, checks the active receive maximum frame size
after structural header decode, and projects typed protocol failures into
stable fixture output ids, `protocol_diagnostic` JSON details, and human
related context. The partial and invalid client connection preface projections
and stream id domain projection include bounded protocol-owned byte previews
for the raw bytes inspected by the preface or frame-header check.
It also splits the active receive-limit entry and receive-window credit from
peer-advertised SETTINGS state. The checked example keeps protocol-default,
local-configuration, and local-SETTINGS receive-limit provenance visible in
frame-size failures, stores received `SETTINGS_ENABLE_PUSH`,
`SETTINGS_MAX_FRAME_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`, and
`SETTINGS_INITIAL_WINDOW_SIZE`, `SETTINGS_HEADER_TABLE_SIZE`, and
`SETTINGS_MAX_HEADER_LIST_SIZE` values as peer-advertised state, and confirms
that those peer-advertised values are not used as inbound frame-size or
concurrent-stream receive limits. For `SETTINGS_INITIAL_WINDOW_SIZE`, it
applies the delta from the previous active value to the tracked open stream
receive-window credit, including the two-open-stream fixture boundary, while
keeping that setting out of receive-limit provenance. It ignores unknown
received SETTINGS identifiers for
peer-advertised state and range diagnostics, while still applying or diagnosing
known SETTINGS items in the same frame at their own item byte offsets. It
range-checks received values for constrained settings before updating
peer-advertised state or open-stream receive-window credit and projects
out-of-range values as
`http2.peer_limit.settings_value_out_of_range` with setting identity, observed
value, accepted range, item byte offset, and peer-limit provenance in
executable output, human diagnostics, and JSON details.
It keeps peer-advertised `SETTINGS_MAX_HEADER_LIST_SIZE` separate from the
local receive policy for inbound header blocks: the executable slice accepts a
header block at the local policy boundary, rejects a completed CONTINUATION
block whose fixture-decoded header list size exceeds that local policy, and
projects `http2.peer_limit.header_list_size_exceeded` with observed size,
allowed size, stream reference, receive-limit provenance, and rule
provenance in ordinary output, human diagnostics, and JSON details.
The imported HPACK fixture module also accepts the static indexed
`:authority` with an empty value, `:method: GET`, `:method: POST`,
`:path: /`, `:path: /index.html`,
`:scheme: http`, and `:scheme: https` request pseudo-header bytes and the
static indexed
`:status: 200`, `:status: 204`, `:status: 206`, `:status: 304`,
`:status: 400`, `:status: 404`, and `:status: 500` response pseudo-header
bytes plus the static indexed `accept-charset:`,
`accept-encoding: gzip, deflate`, `accept-language:`, `accept-ranges:`,
`accept:`, `access-control-allow-origin:`, `age:`, `allow:`,
`authorization:`, `cache-control:`, `content-disposition:`,
	`content-encoding:`, `content-language:`, `content-length:`,
	`content-location:`, `content-range:`, `content-type:`, `cookie:`, `date:`,
	`etag:`, `expect:`, `expires:`, `from:`, `host:`, `if-match:`,
	`if-modified-since:`, `if-none-match:`, `if-range:`,
	`if-unmodified-since:`, `last-modified:`, `link:`, `location:`,
	`max-forwards:`, `proxy-authenticate:`, `proxy-authorization:`, `range:`,
	`referer:`, `refresh:`, `retry-after:`, `server:`, `set-cookie:`,
	`strict-transport-security:`, `transfer-encoding:`, `user-agent:`,
	`vary:`, `via:`, and
	`www-authenticate:` header bytes, plus literal-without-indexing and
literal-with-indexing fixtures whose first byte names a supported static-table
header name for `:authority`, `:method`, `:path`, `:scheme`, or `:status`.
Those literal fixtures share the HPACK string literal decoder for short
visible-ASCII raw values and Huffman-marked values decoded by
the HPACK static Huffman table. The same fixture decoder accepts one
continuation byte after a saturated seven-bit string-length prefix for checked
long raw and Huffman-marked values on supported literal names, through both
literal-without-indexing and literal-with-indexing forms. The executable slice
covers a
raw `:authority` value through completed HEADERS and final CONTINUATION paths,
raw `:status` through completed HEADERS, Huffman `:path: test` through
completed HEADERS, Huffman `:method: PUT` through both literal-without-indexing
and literal-with-indexing, Huffman `:status: 200` through completed HEADERS
and final CONTINUATION, raw literal-with-indexing `:authority`, Huffman
literal-with-indexing `:scheme: https`, and raw literal-with-indexing
`:status`. Completed HEADERS and final CONTINUATION paths reach that long
string-length fixture boundary before the local header-list receive limit
rejects the decoded long values. It rejects
non-visible raw bytes, malformed string length including non-terminating
string-length continuations, malformed Huffman padding, and a malformed raw
`:status` literal. Checked bytes include zero-length `:path`
as `0x04 0x80`, `:path: test` as `0x04 0x83 0x49 0x50 0x9f`,
`:scheme: https` as `0x06 0x84 0x9d 0x29 0xad 0x1f`,
`:status: 200` as `0x08 0x82 0x10 0x01`, and
literal-without-indexing `:authority: www.example.com` as
`0x01 0x8c 0xf1 0xe3 0xc2 0xe5 0xf2 0x3a 0x6b 0xa0 0xab 0x90 0xf4 0xff`.
In completed HEADERS or final CONTINUATION frames, the fixture returns
ordinary header-list data through the same accessors as the deterministic
fixture-label blocks, advances immutable fixture state, and also covers one
dynamic-table receive slice: a
literal-with-indexing `:path: /target` block inserts that entry into the next
immutable HPACK state carried by the HTTP/2 decode state, a later `0xbe`
indexed representation decodes through that carried state, and the same
indexed representation without prior state stays unsupported. A later
literal-with-indexing `:method: PUT` block is inserted as the newest bounded
fixture dynamic-table entry while the older `:path: /target` entry remains
addressable when the table has room. The fixture carries that state through
both completed HEADERS and final CONTINUATION paths, decodes the newest entry
through `0xbe`, decodes the older retained entry through `0xbf`, and keeps
dynamic entries evicted by a reduced fixture table size on the unsupported
fixture path. Reducing the fixture table size to `42` keeps the newest
supported `:method: PUT` entry and evicts the older `:path: /target` entry;
reducing the fixture table size to `30` evicts both supported entries. The
fixture also
accepts dynamic table-size update bytes `0x3e`, `0x3f`, one-byte
continuations such as `0x3f 0x01`, and the fixture-boundary slice of general
multi-byte HPACK integer continuations with the table-size update prefix,
including `0x3f 0x0b`, `0x3f 0x80 0x01`, `0x3f 0x81 0x01`, and
`0x3f 0x82 0x02`, returns
next immutable HPACK states whose checked table sizes include `30`, `31`,
`32`, `42`, `159`, `160`, and `289`, and carries those states through completed
HEADERS and final CONTINUATION paths before later header blocks are decoded.
Unsupported HPACK bytes, including malformed non-terminating table-size
updates, table-size updates with trailing bytes after a complete integer, and
malformed Huffman padding, Huffman EOS, or Huffman strings whose decoded bytes
are not visible ASCII, remain on `hpack.fixture.unsupported_header_block`.
It accepts zero-length SETTINGS ACK frames on the connection stream without
updating peer-advertised SETTINGS state, rejects nonzero-length SETTINGS ACK
frames as `http2.protocol.invalid_payload_length`, and keeps SETTINGS ACK on
nonzero streams on the existing `http2.protocol.invalid_stream_id` path.
It also records one outstanding local SETTINGS batch when the fixture emits
local `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_INITIAL_WINDOW_SIZE`,
`SETTINGS_ENABLE_PUSH`, `SETTINGS_MAX_CONCURRENT_STREAMS`,
`SETTINGS_MAX_FRAME_SIZE`, or `SETTINGS_MAX_HEADER_LIST_SIZE` items, including
a two-item batch. Those local SETTINGS send-intents emit one
frame-header-plus-payload chunk with length `6 * item_count`, kind `4`, flags
`0`, stream id `0`, and the selected identifier and four-byte unsigned value
pairs in order. The local `SETTINGS_ENABLE_PUSH` send-intent accepts values
`0` and `1`, and rejects other values before output bytes are emitted using
the SETTINGS value range failure shape, including when the invalid value
appears in a batch. A valid SETTINGS ACK clears that state, including a
multi-item batch, and a valid SETTINGS ACK with no outstanding local SETTINGS
is rejected as
`http2.protocol.unexpected_settings_ack` in ordinary output, human diagnostics,
and JSON details.
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
domain, endpoint role, active state, rule provenance, and a bounded
frame-header byte preview. Representation failures for the generated
`UInt31be` helper remain schema or codec failures instead of protocol
diagnostics.
The same executable example now includes the outbound frame-header encode
slice. Ordinary source builds record-shaped frame descriptions with `length`,
`kind`, `flags`, and `stream_id`, invokes the generated binary schema encode
helper for the HTTP/2 wire header layout, and checks one nine-byte output
chunk for a SETTINGS header on the connection stream, a DATA header on a
nonzero stream, and the maximum valid `UInt31be` stream id. It also keeps the
generated helper's `codec.encode_value_unrepresentable` error visible for an
out-of-range stream id instead of projecting that representation failure into
a protocol diagnostic.
It also includes the outbound SETTINGS ACK send-intent slice. After a valid
non-ACK SETTINGS receive, ordinary source constructs exactly one immutable
nine-byte output chunk through the same frame-header encode path, with length
`0`, kind `4`, flags `1`, and stream id `0`. The send intent does not update
peer-advertised SETTINGS state or local receive-limit state.
The implemented slice also includes outbound DATA send-intent flow control,
frame-size splitting, PADDED DATA encoding, and output. Ordinary source tracks
outbound connection and stream credit separately from inbound receive windows,
uses received `SETTINGS_MAX_FRAME_SIZE` as the peer-owned maximum DATA frame
size for frames this endpoint sends, and uses received
`SETTINGS_INITIAL_WINDOW_SIZE` as the peer-owned stream-window credit.
Accepted DATA intents whose full encoded payload fits available outbound
connection and stream credit emit one immutable output chunk containing one
or more DATA frame-header-plus-payload frames, each no larger than the
peer-advertised maximum frame size. PADDED DATA send-intents encode the
PADDED flag, one pad-length byte per emitted frame, application bytes, and
requested zero padding bytes. The frame-size and outbound credit checks count
the pad-length byte and padding as part of each encoded DATA payload.
`END_STREAM` appears only on the final DATA frame when requested, and
accepted DATA consumes outbound connection and stream credit by the full
encoded DATA payload length after all split frames encode. DATA intents
larger than available outbound connection credit or available outbound stream
credit, and PADDED DATA intents whose padding cannot fit in the selected
frame payload, are rejected in source-level fixture output before output
bytes or credit changes. Accepted DATA with `END_STREAM` records local
closed-stream state so later outbound DATA, outbound HEADERS, and
stream-level outbound `WINDOW_UPDATE` for that stream use the existing closed
stream-state rejection boundary. The receive core records that local
`END_STREAM` as half-closed-local for inbound processing: later inbound DATA
on that stream consumes connection and stream receive-window credit, PADDED
DATA keeps exposing only application bytes, invalid padding and stream-window
failures report the half-closed-local active state, connection-window failures
remain connection-flow-control failures, and accepted inbound DATA with peer
`END_STREAM` transitions the stream to closed-by-peer. Generated frame-header
representation failures stay on the `codec.encode_value_unrepresentable`
encode-error path.
The implemented outbound HEADERS send-intent slice also observes received
GOAWAY graceful-shutdown state. It accepts an open stream at the recorded
last-stream-id boundary, rejects an open stream above that boundary with
`http2.protocol.stream_after_goaway` before frame splitting or encode checks, and
keeps stream id zero plus closed-stream failures on their narrower existing
paths.
The implemented slice also includes the outbound `WINDOW_UPDATE`
receive-credit intent. Ordinary source accepts connection-level and
currently open stream-level increments, emits exactly one immutable frame
with length `4`, kind `8`, flags `0`, the selected stream id, and a
four-byte unsigned increment payload, and rejects zero, negative,
out-of-range, current-window overflow, stream id zero, idle-stream,
closed-stream, reset-stream, and mismatched-stream intents before output
bytes. Generated frame-header and increment-payload representation failures
remain `codec.encode_value_unrepresentable` encode errors instead of
protocol diagnostics.
It now also handles structurally decoded PING and GOAWAY frames. PING is
accepted only on the connection stream with an eight-byte payload, and the
observable output preserves the ACK flag distinction. GOAWAY is accepted only
on the connection stream with the fixed eight-byte prefix needed to expose the
last stream id and error code, then transitions the decode state into graceful
shutdown. Stream-targeted PING and GOAWAY frames are stream id domain
failures, while wrong-length PING and GOAWAY payloads use
`http2.protocol.invalid_payload_length` in ordinary output, human diagnostics,
and JSON `protocol_diagnostic` details.
The implemented slice also includes the narrow outbound PING ACK send-intent.
After a valid inbound non-ACK PING frame, ordinary source encodes a nine-byte
header with length `8`, kind `6`, ACK flag `1`, and stream id `0`, appends
the original eight-byte opaque payload, and emits exactly one immutable output
chunk. Received PING ACK frames remain observable as received ACKs and emit no
response chunk.
The implemented slice also accepts DATA frames on an already-open stream and
decrements both connection and stream receive-window credit by the payload
length. PADDED DATA consumes receive-window credit for the full DATA payload,
including the pad-length byte and padding bytes, while exposing only
application data bytes as DATA content. A PADDED DATA pad length greater than
the remaining payload uses `http2.protocol.invalid_data_padding` in ordinary
output, human diagnostics, and JSON `protocol_diagnostic` details. DATA on the
connection stream is a stream id domain failure, DATA on an idle stream
remains `http2.protocol.invalid_frame_kind`, and DATA payloads that exceed the
available stream or connection receive-window credit use
`http2.peer_limit.flow_control_window_exceeded` with byte offset, stream
reference, observed payload length, allowed window credit, active state, and
rule provenance in executable output, human diagnostics, and JSON
`protocol_diagnostic` details.
When accepted inbound DATA carries `END_STREAM`, the same receive-window
accounting is applied before the tracked peer-created stream transitions to a
closed-by-peer state. Later DATA and stream-level `WINDOW_UPDATE` frames for
that stream use the existing stream-state
`http2.protocol.invalid_frame_kind` failure shape with closed-by-peer active
state and rule provenance.
After this endpoint sends DATA with `END_STREAM`, the receive core records the
stream as half-closed-local rather than fully closed. Inbound DATA on that
stream stays on the receive-window accounting path, including PADDED DATA
content projection, invalid padding diagnostics, stream-window failures, and
connection-window failures. Inbound DATA with peer `END_STREAM` moves that
stream to the closed-by-peer state.
When accepted inbound HEADERS carries `END_STREAM`, the stream transitions to
the same closed-by-peer state after the header block completes, HPACK fixture
decoding succeeds, and the local header-list receive-limit check passes. The
single-frame HEADERS `END_HEADERS | END_STREAM` path and the HEADERS
`END_STREAM` plus final CONTINUATION path both reject later DATA and
stream-level `WINDOW_UPDATE` through the same closed-by-peer failure shape.
The implemented slice also receives `WINDOW_UPDATE` frames. Connection-level
`WINDOW_UPDATE` increases connection receive-window credit, and
stream-level `WINDOW_UPDATE` increases the currently open stream's
receive-window credit. Wrong-length `WINDOW_UPDATE` payloads use
`http2.protocol.invalid_payload_length`, idle or unknown stream-targeted
`WINDOW_UPDATE` remains the existing stream-state
`http2.protocol.invalid_frame_kind` shape, and zero or overflowing increments
use `http2.peer_limit.flow_control_window_exceeded` without changing receive
window state.
The implemented slice also includes the narrow outbound `RST_STREAM`
send-intent. Ordinary source accepts a nonzero currently open stream, encodes
a nine-byte header with length `4`, kind `3`, flags `0`, and the selected
stream id, appends the four-byte error-code payload, and records outbound
reset state so a later stream-level `WINDOW_UPDATE` for that stream uses the
same reset stream-state rejection boundary. It rejects stream id `0`, missing
or non-open streams, already reset streams, and generated encode-helper
representation failures for the stream id or error-code payload before
accepted bytes are produced.
The implemented slice also includes the narrow outbound PRIORITY send-intent.
Ordinary source accepts a nonzero currently open stream, encodes a nine-byte
header with length `5`, kind `2`, flags `0`, and the selected stream id,
appends the five-byte priority payload with exclusive flag, dependency stream
id, and weight, and leaves outbound receive credit unchanged. It rejects
stream id `0`, missing or non-open streams, already closed or reset streams,
mismatched open streams, and self-dependency before accepted bytes are
produced. Generated encode-helper representation failures for the frame
stream id or dependency payload remain `codec.encode_value_unrepresentable`
encode errors instead of protocol diagnostics.
The implemented slice also includes the narrow outbound HEADERS send-intent.
Ordinary source accepts a nonzero currently open stream and an already-encoded
opaque header-block `ByteChunk`, or builds that chunk from fixture-owned
ordinary header-list values through the HPACK fixture encoder before entering
the same send-intent path. Header blocks within the peer-advertised maximum
frame size encode as one HEADERS frame with kind `1`, `END_HEADERS`, optional
`END_STREAM`, and the selected stream id. Larger header blocks encode as one
HEADERS frame followed by CONTINUATION frames on the same stream; every
payload chunk respects the peer-advertised maximum frame size, `END_HEADERS`
is set only on the final frame, and optional `END_STREAM` stays on the first
HEADERS frame. Accepted `END_STREAM` records local closed-stream state so a
later stream-level `WINDOW_UPDATE` for that stream uses the same closed
stream-state rejection boundary. It rejects stream id `0`, missing or non-open
streams, already closed or reset streams, and generated frame-header
representation failures before accepted bytes are produced. Unsupported
fixture header-list values return typed HPACK fixture encode failures instead
of HTTP/2 protocol diagnostics.
The implemented slice also includes the narrow server-side outbound
`PUSH_PROMISE` send-intent. Ordinary source accepts a nonzero currently open
client-created associated stream, a nonzero server-initiated promised stream
id, and an already-encoded opaque header-block `ByteChunk`, or builds those
bytes from a fixture-owned header list through the same HPACK fixture encoder.
It encodes frame kind `5` on the associated stream, writes the promised stream
id through the generated `UInt31be` payload helper, then appends the
header-block bytes. When the promised-id payload plus header-block bytes
exceed the
peer-advertised maximum frame size, the output uses one `PUSH_PROMISE` frame
followed by CONTINUATION frames on the same associated stream, with
`END_HEADERS` only on the final frame. It rejects stream id `0`, missing,
closed, reset, mismatched, or server-created associated streams, promised
stream id `0`, and representable client-initiated promised stream ids before
accepted bytes are produced. Generated payload representation failures, such
as out-of-range promised stream ids, remain
`codec.encode_value_unrepresentable` encode errors instead of HTTP/2
protocol diagnostics.
The implemented slice also includes the outbound GOAWAY send-intent.
Ordinary source validates the selected last stream id and error code through
the schema-declared GOAWAY payload record, encodes a nine-byte header with
length `8`, kind `7`, flags `0`, and stream id `0`, appends the eight-byte
GOAWAY payload, and records local graceful-shutdown state. A later
peer-created HEADERS stream greater than the sent last stream id uses the
same post-GOAWAY stream rejection boundary as received GOAWAY state.
Generated schema encode-helper representation failures for the last stream id
or error-code payload are preserved before accepted bytes
are produced.
The implemented slice also applies received `SETTINGS_INITIAL_WINDOW_SIZE`
values to tracked open stream receive-window credit by the delta between the
previous active peer setting and the new value. The checked two-open-stream
boundary records streams `1` and `3` separately, applies the same SETTINGS
delta to both open stream windows, and keeps later DATA and stream-level
`WINDOW_UPDATE` accounting on the targeted stream's own adjusted credit. The
adjusted stream credit can become negative, in which case later DATA remains
blocked by `http2.peer_limit.flow_control_window_exceeded` until stream-level
`WINDOW_UPDATE` restores enough credit on that stream.
The implemented slice also admits peer-created streams narrowly. HEADERS
frames on idle, nonzero streams open tracked peer-created streams when the
active concurrent-stream receive limit allows them. A HEADERS frame that
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
The implemented slice also receives PRIORITY frames on nonzero
client-initiated stream ids. It decodes the five-byte payload into
source-visible dependency stream id, exclusive flag, and weight facts. On the
currently tracked open stream, it records those facts and lets a later
accepted PRIORITY frame for that stream replace the tracked dependency,
exclusive flag, and weight. On an idle stream when no peer-created stream is
tracked, it exposes those facts while leaving the stream idle and leaving the
concurrent-stream receive count unchanged. PRIORITY on closed-by-peer, reset,
or mismatched streams uses the existing stream-state failure boundary rather
than opening or retargeting stream state. PRIORITY on stream id zero uses the
existing stream id domain failure, wrong-length PRIORITY payloads use
`http2.protocol.invalid_payload_length`, and PRIORITY self-dependency uses
`http2.protocol.invalid_priority_dependency` in ordinary output, human
diagnostics, and JSON `protocol_diagnostic` details with a bounded preview of
the inspected PRIORITY payload bytes.
The implemented slice also recognizes `PUSH_PROMISE` as a known HTTP/2 frame
kind before unknown extension-frame fallback. In the server-side receive core,
`PUSH_PROMISE` on a nonzero client-initiated stream is rejected through the
existing `http2.protocol.invalid_frame_kind` projection with server receive
state and rule provenance. `PUSH_PROMISE` on stream id zero follows the
existing stream id domain failure route before frame-kind state validation.

The remaining scope below is still planned work for the full protocol core.

## Non-Goals

- Do not implement TLS, ALPN, socket listeners, or platform networking.
- Do not require complete HPACK support.
- Do not optimize for production throughput.
- Do not encode all protocol state rules inside schema declarations.

Completed HPACK fixture behavior is current behavior under
`../specification/` and the implemented-proposal records under
`../reference/implemented-proposals/`, including
`../reference/implemented-proposals/http2-hpack-authority-static-indexed-fixture.md`
and
`../reference/implemented-proposals/http2-hpack-string-literal-fixture.md`.
The remaining HPACK work in this proposal starts after that fixture boundary:
full HPACK compression, unbounded dynamic-table behavior, general eviction
policy beyond the checked fixture-boundary entry-size calculation and
table-size update slice, Huffman encoding, broader dynamic-table string
encoding policy, and production header validation beyond the fixture request
and response checks.
The completed request-header and response-header validation slices are
current behavior under `../specification/` and
`../reference/implemented-proposals/http2-request-header-validation.md` plus
`../reference/implemented-proposals/http2-response-header-validation.md`: the
HTTP/2 core validates fixture-marked request and response header lists after
HPACK fixture decode on both completed HEADERS and final CONTINUATION paths.
Request validation rejects duplicate request pseudo-headers, request
pseudo-headers after regular headers, missing `:method`, `:scheme`, or
`:path`, and response-only `:status` through
`http2.protocol.invalid_request_header_list`. Response validation rejects
missing or duplicate `:status`, request-only `:authority`, `:method`,
`:scheme`, or `:path`, and response pseudo-headers after regular headers
through `http2.protocol.invalid_response_header_list`.
The completed outbound HPACK fixture encoder slice is current behavior under
`../specification/` and
`../reference/implemented-proposals/http2-outbound-hpack-fixture-encoder.md`.
It supports fixture-owned static-indexed header lists, raw short literal
header-list encoding for supported static-table names, the checked request and
response pseudo-header fixture lists needed by outbound send-intents, and one
unsupported-header failure path that remains an HPACK fixture result.

The remaining scope below is still planned work for the full protocol core and
full HPACK behavior.

## Completion Criteria

- Examples show valid and invalid frame fixtures for the target slice.
- A pure decode state transition handles chunk arrival and end-of-stream.
- Protocol-state failures are typed and diagnostically structured.
- The core keeps only undecoded suffix bytes after frame consumption.
- Full HPACK compression, unbounded dynamic table behavior, general eviction
  policy, table-size policy beyond fixture-boundary HPACK integer updates,
  Huffman encoding, broader dynamic-table string encoding policy, and
  production header validation remain later work beyond the implemented
  fixture boundary.
- The design driver can use the core to evaluate schema, byte, codec,
  diagnostic, and standard-library decisions.
