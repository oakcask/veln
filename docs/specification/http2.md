# HTTP/2 Standard Modules

HTTP/2 support is opt-in. Source files import the required public module from
the toolchain-owned `std` package; no HTTP/2 function is part of the implicit
prelude.

```veln
use http2::frame from "std"
use http2::diagnostic from "std"
use http2::hpack from "std"
use http2::hpack::diagnostic from "std"
use http2::core from "std"
use http2::connection from "std"
```

The public routes are:

- `http2::frame`: frame decoding and validated frame-header encoding.
- `http2::diagnostic`: protocol and peer-limit diagnostic constructors.
- `http2::hpack`: prefixed-integer and HPACK Huffman codecs, static entries,
  immutable dynamic-table state, table-size updates, indexed and literal
  header-field encoding, and complete header-block encoding and decoding.
- `http2::hpack::diagnostic`: HPACK diagnostic constructors.
- `http2::core`: connection and role-specific stream-id domains, immutable
  connection-preface and initial-peer-SETTINGS transitions, pure frame
  payload-length validation, immutable pending header-block sequencing,
  immutable local SETTINGS send transitions, pure peer SETTINGS item
  validation, immutable peer SETTINGS state application, immutable SETTINGS
  acknowledgement state, peer-created stream admission high-water tracking,
  pure public HPACK header-list validation for request, response, and trailer
  rules, pure flow-control numeric domains, stream lifecycle projection and
  frame-admission predicates, pure PING request and ACK response transitions,
  immutable receive-frame dispatch for DATA, HEADERS, CONTINUATION,
  PUSH_PROMISE, WINDOW_UPDATE, RST_STREAM, SETTINGS, and GOAWAY payload
  application, immutable chunked receive state for server preface, initial
  peer SETTINGS, frame buffering, inbound SETTINGS ACK output, inbound PING
  ACK output, and inbound PRIORITY offset application,
  immutable GOAWAY, RST_STREAM, PRIORITY, DATA, WINDOW_UPDATE, HEADERS, and
  PUSH_PROMISE send transitions, an immutable output buffer for ordering
  accepted send bytes, and an immutable aggregate connection state that
  composes those migrated components with the public HPACK dynamic table and
  an immutable stream collection.
- `http2::connection`: the `drive_server` duplex-stream connection driver and
  typed protocol-owned connection failures for one caller-owned stream.

Nested implementation modules below `http2::hpack` and `http2::core` are not
package exports.
The JVM adapter keeps its intrinsic link names private; source code calls only
the module-qualified API. Diagnostic ids, human rendering, and
`details.protocol_diagnostic` projections remain stable.

`http2::core` exposes flow-control domain types for connection window credit,
stream window credit, configured initial window size, and received
`WINDOW_UPDATE` increments. Connection credit and configured initial window
sizes accept `0..2147483647`; stream credit accepts `-2147483647..2147483647`
so peer initial-window reductions can be represented before later refill; and
`WINDOW_UPDATE` increments accept `1..2147483647`. Out-of-range construction
returns a `FlowControlDomainFailure` with the domain label, observed value, and
accepted bounds.

`debit_connection_window(...)`, `debit_stream_window(...)`,
`refill_connection_window(...)`, and `refill_stream_window(...)` return
accepted next-credit decisions or the same typed domain failure without
changing the caller's input credit or increment value.

`http2::core::validate_frame_payload_length(...)` performs pure inbound frame
shape validation. The caller supplies the active-state label associated with
its protocol projection. The function returns either a typed success or a
`FramePayloadLengthFailure` containing the offset, frame kind, stream id,
observed and expected lengths, active-state label, rule provenance, and exact
supplied payload preview. Rejection exposes no partial success and does not
alter the caller's immutable preview.

PING is exactly 8 payload octets; GOAWAY is at least 8; WINDOW_UPDATE and
RST_STREAM are exactly 4; and PRIORITY is exactly 5. A SETTINGS frame with ACK
is empty, while a SETTINGS frame without ACK has a length divisible by 6.
HEADERS requires the PADDED and PRIORITY prefixes selected by its flags.
PUSH_PROMISE requires its promised-stream prefix and the optional PADDED
prefix. DATA, CONTINUATION, and unknown kinds have no additional fixed or
minimum length constraint in this validator. Padding content, frame-header
decoding, and maximum-frame-size validation remain separate responsibilities.

`http2::core::send_ping(payload)` accepts exactly eight opaque payload octets
and emits one complete frame byte chunk with a length-`8`, kind-`6`,
flags-`0`, stream-`0` header followed by the unchanged payload. Seven, nine,
or otherwise invalid payload lengths return the shared
`FramePayloadLengthFailure` from `validate_frame_payload_length(...)`, expose
no bytes, and leave the caller's payload unchanged. Encoding failure is a
distinct transition shape, although the public constants are representable.

`http2::core::respond_to_ping(flags, payload)` emits one ACK frame byte chunk
for a validated non-ACK PING by preserving the eight-octet payload and using a
length-`8`, kind-`6`, flags-`1`, stream-`0` header. A received PING ACK returns
an explicit no-response action with no bytes, preventing an ACK loop.

`http2::core::server_connection_state(starting_offset)` and
`client_connection_state(starting_offset)` create immutable aggregate
connection state for the endpoint role. The state starts with the matching
connection-preface offset, role-specific initial peer SETTINGS gate, idle
pending header-block state, an empty production HPACK dynamic table with
capacity `4096`, peer SETTINGS protocol defaults, empty SETTINGS ACK state,
empty peer stream admission state, empty stream collection, connection receive
credit `65535`, empty local SETTINGS policy, and an open lifecycle.

Projection functions expose each component without mutating the aggregate.
`connection_state_with_*` functions return new aggregate states for individual
component replacement, including next offset, preface, initial peer SETTINGS
gate, pending header block, HPACK table, peer SETTINGS state, SETTINGS ACK
state, peer stream admission state, stream collection, connection receive
credit, local SETTINGS policy, and lifecycle. The lifecycle projection
distinguishes `open`, `draining`, and `closed` with last-stream-id and
error-code fields for GOAWAY integration.

`http2::core::empty_stream_collection()` creates immutable standard-owned
stream state with no entries. `stream_entry(...)` validates receive and send
stream-window credits through the public flow-control domains, then records a
stream id, lifecycle, receive credit, send credit, and content-length expected
and observed counters. Collection updates add or replace one stream id,
leaving earlier collection values unchanged; focused update helpers replace
lifecycle, receive credit, send credit, or content-length accounting only for
an existing stream. Updates that target an absent stream leave the collection
and its existing entries unchanged. Projections expose stream count, active
stream count, lookup, stream ids, lifecycle labels and reset error codes,
receive and send credits, and content-length counters.

Stream lifecycle values distinguish open, client-push-associated,
reserved-by-peer, reserved-local, half-closed-local, half-closed-remote,
closed, and reset states. Public predicates expose whether a lifecycle is
active, retains receive-window credit, projects as an open stream, and accepts
DATA, RST_STREAM, WINDOW_UPDATE, or PRIORITY in the current receive
transition. Public projection helpers also expose the active-state label and
rejection-rule label used by later protocol failures. Receive and send
transitions consume the stream collection through the aggregate
`CoreConnectionState` boundary.

`http2::core::apply_remote_end_stream_lifecycle(state, offset, frame_kind,
stream_id, end_stream, preview)` is the standard-owned immutable lifecycle
update for a successfully admitted inbound DATA or completed header-block
transition. When `end_stream` is false, the accepted decision returns the
input aggregate state unchanged. When `end_stream` is true, open and
client-push-associated streams become half-closed-remote, and a
half-closed-local stream becomes closed. The transition preserves stream
credits, content-length counters, and all unrelated aggregate components.

`http2::core::apply_rst_stream_lifecycle(state, offset, stream_id, error_code,
preview)` records an inbound RST_STREAM as a reset stream lifecycle with the
supplied error code. Missing-stream failures expose the stable
`http2.protocol.invalid_stream_reference` id, offset, frame kind, stream id,
idle-stream active-state label, `idle_streams_require_headers` provenance,
and supplied frame-header preview, without returning a next state or mutating
the input aggregate. Accepted and rejected accessor helpers reject the wrong
decision variant.

`http2::core::apply_receive_frame(state, frame_bytes)` is the standard-owned
immutable receive-frame dispatcher for the migrated SETTINGS, DATA, HEADERS,
PUSH_PROMISE, CONTINUATION, WINDOW_UPDATE, RST_STREAM, and GOAWAY
payload-application boundary. It decodes one complete HTTP/2 frame, validates
the payload length,
applies stream-frame admission, parses SETTINGS payload items, SETTINGS ACKs,
DATA padding length, HEADERS padding and priority prefixes, CONTINUATION
fragments, WINDOW_UPDATE increments, RST_STREAM error codes, and GOAWAY
last-stream-id plus error-code fields. It then composes the existing
aggregate peer SETTINGS validation and state application, SETTINGS ACK
bookkeeping, DATA flow-control, production HPACK header-block decoding,
request, response, and trailer header-list validation, pending header-block
sequencing, stream and connection WINDOW_UPDATE flow-control, remote
END_STREAM lifecycle, RST_STREAM lifecycle, and GOAWAY shutdown transitions.
Accepted decisions expose the next aggregate state, frame kind, stream id,
payload length, and applied transition label. Rejected decisions expose a
focused failure source, stable failure id, offset, frame kind, stream id,
decode, payload-read, HPACK, or header-list reason where applicable, and
preserved preview without returning a next state or mutating the input
aggregate. Payload-length rejections expose their rule provenance through the
public failure reason and rule-provenance projections. The dispatcher starts
from one complete frame; connection-preface
consumption, the initial peer SETTINGS gate, chunk buffering, inbound PING
output integration, and inbound PRIORITY state application are handled by the
chunked receive boundary. Inbound DATA on a stream with an accepted `content-length` updates
only the DATA application-octet count. It rejects over-length DATA and
END_STREAM shortfalls as `http2.protocol.content_length_mismatch` with the
expected and observed lengths, active-state label, `rfc9113_content_length_body`
provenance, and preserved DATA preview before changing flow-control credit,
stream lifecycle, or the input aggregate.

`http2::core::receive_connection_state(...)` creates an immutable chunked
receive state from caller-owned aggregate connection, buffered input, and
output-buffer state. `receive_connection_chunk(...)` appends the supplied input
chunk, consumes the server connection preface when required, buffers partial
frame bytes, enforces the initial peer SETTINGS gate before the first accepted
frame, then applies each complete buffered frame in receive order through
`apply_receive_frame(...)` until the buffer is empty, only a partial frame
remains, or a rejection occurs.
Accepted non-ACK peer SETTINGS frames with a recorded pending ACK append an
outbound SETTINGS ACK through the output buffer in receive order and clear
only the pending peer-ACK state.
Accepted non-ACK PING frames append the exact PING ACK bytes after any earlier
output, while received PING ACKs append no bytes. Accepted PRIORITY frames
advance the aggregate offset and preserve stream and output state. Accepted
PUSH_PROMISE frames reserve the promised stream through the aggregate receive
dispatcher without appending output bytes or mutating caller-owned output.
Rejections from the preface gate, initial SETTINGS gate, frame decode, or frame
dispatcher, including after an earlier complete frame in the same input chunk
has advanced HPACK, continuation, DATA flow-control and content-length, or
shutdown state, expose a focused failure source and do not expose a next
chunked receive state, preserving the caller-owned connection, buffered input,
and output values, including any output chunks supplied by the caller.

The adjacent
[`core_test.veln`](../../crates/veln-stdlib/veln/http2/core_test.veln) checks
preface plus initial SETTINGS composition, partial PING buffering, complete
frames followed by a partial suffix, SETTINGS ACK and PING ACK byte ordering
across split and same-chunk receive, PRIORITY offset application,
PUSH_PROMISE reservation without output side effects, initial-gate rejection
context, later-frame rejection after locally advanced receive state, and
input/output preservation on rejection.
The focused
[`http2-core-receive-connection-boundary`](../../examples/specification/run/http2-core-receive-connection-boundary/)
case records the public decision, state, failure, and emitted-byte
projections.

`http2::core::finalize_receive_connection_eof(...)` finalizes a chunked
receive state after clean transport end. It accepts only when the connection
preface is complete, the initial peer SETTINGS gate has accepted a complete
SETTINGS frame, no partial frame bytes remain, and no pending header block is
active. Accepted EOF returns the immutable aggregate connection state.
Rejected EOF returns a typed incomplete-input failure that identifies the
pending source: connection preface, initial peer SETTINGS, frame bytes, or
pending header block. The failure exposes offset, pending count, expected
count when applicable, frame kind, stream id, reason, and preview facts.

`http2::connection::drive_server(state)` drives one server-side HTTP/2
connection through `transport::DuplexStream`. If the supplied core lifecycle is
closed, the driver returns that state without reading, writing, sending local
SETTINGS, or requesting a protocol transition. Otherwise the driver first
sends the initial empty server SETTINGS through the existing local SETTINGS
send transition, commits the returned core state, and writes those bytes
before reading peer input.

For each `Some(chunk)` read, the driver delegates to
`http2::core::receive_connection_chunk(...)`. Accepted transitions commit the
returned receive state and write each newly accepted output chunk exactly once
in output-buffer order. Rejected transitions return
`Http2ConnectionProtocolFailure` with the focused core failure facts and do
not write output from the rejected transition; bytes from earlier accepted
driver iterations remain committed. On clean end, the driver delegates to
`finalize_receive_connection_eof(...)` and returns either the accepted final
core state or `Http2ConnectionIncompleteInput`.

The executable connection-driver evidence lives under
`examples/specification/run/http2-connection-server-split-preface/`,
`examples/specification/run/http2-connection-settings-ack/`,
`examples/specification/run/http2-connection-partial-frame/`,
`examples/specification/run/http2-connection-clean-end/`,
`examples/specification/run/http2-connection-truncated-end-json/`,
`examples/specification/run/http2-connection-protocol-failure-json/`, and
`examples/specification/run/http2-connection-closed-entry/`.

`http2::core::apply_goaway_receive_shutdown(state, offset, payload, preview)`
applies a validated inbound GOAWAY payload to the aggregate connection
lifecycle. An open connection becomes draining with the decoded last-stream-id
and error code. A later GOAWAY may keep or tighten the existing boundary; it
may not raise the last-stream-id after shutdown has begun. Boundary-raising
failures expose `http2.protocol.stream_after_goaway`, the proposed stream id,
the existing last-stream-id, shutdown-state label, endpoint role, provenance,
and preserved preview without returning a next state or mutating the input.
Truncated payloads reject with `http2.frame.payload_read_failure`, payload-read
provenance, the underlying read reason, and the supplied preview while keeping
the input connection state unchanged.
`complete_connection_shutdown_drain(state)` closes a draining lifecycle when no
active stream remains at or below the GOAWAY boundary.
The focused
[`http2-core-connection-shutdown`](../../examples/specification/run/http2-core-connection-shutdown/)
case projects accepted, tightened, drained, boundary-rejected, and
payload-read-rejected shutdown decisions through the public facade.

`http2::core::send_goaway_shutdown(state, offset, last_stream_id, error_code,
debug_data)` emits one GOAWAY frame from explicit caller-owned aggregate state
and returns the next lifecycle state, the accepted last-stream-id and error
code, and exact bytes. The payload is the last-stream-id, error code, and
caller-supplied debug data. An open connection becomes draining, or closed
immediately when no active stream remains at or below the boundary. A later
outbound GOAWAY may keep or tighten the existing boundary; it may not raise
the last-stream-id after shutdown has begun. Boundary-raising failures expose
`http2.protocol.stream_after_goaway`, the proposed stream id, the existing
last-stream-id, shutdown-state label, endpoint role, provenance, and preserved
empty output without returning a next state or mutating the input. Integer or
frame encoding failures also expose no bytes and no next state.
The focused
[`http2-core-goaway-send`](../../examples/specification/run/http2-core-goaway-send/)
case records accepted bytes, drain completion, boundary rejection, empty
failure output, encode failure output, and input-state preservation through
the public facade.

`http2::core::send_data(state, offset, stream_id, data, end_stream)` emits a
kind-`0` DATA frame from an existing outbound-data-capable stream. The
accepted transition debits the stream send credit by the DATA payload length,
updates the stream content-length observed counter when an expected
content-length is present, and applies local END_STREAM by moving an open
stream to half-closed-local or closing a half-closed-remote stream. DATA
stream-zero, idle, closed, reset, exhausted send-window, over-length, short
END_STREAM content-length, and active GOAWAY boundary cases reject with typed
failures, no bytes, and no next state. GOAWAY boundary failures expose
`http2.protocol.stream_after_goaway`, the attempted stream id, retained
last-stream-id, shutdown-state label, endpoint role, and
`goaway_last_stream_id` provenance.

`http2::core::send_window_update(state, offset, stream_id, increment)` emits a
kind-`8`, flags-`0` WINDOW_UPDATE frame. Stream-level sends refill the target
stream receive credit when the stream has a receive window. Connection-level
sends refill the aggregate connection receive credit. Connection-level
WINDOW_UPDATE remains valid after GOAWAY, while stream-level WINDOW_UPDATE
honors the retained GOAWAY last-stream-id boundary. Invalid increments, window
overflow, idle stream references, streams beyond the active GOAWAY boundary,
and streams without receive-window ownership reject with typed failures, no
bytes, and no next state.
The focused
[`http2-core-outbound-data-flow`](../../examples/specification/run/http2-core-outbound-data-flow/)
case records accepted DATA, stream WINDOW_UPDATE, and connection
WINDOW_UPDATE bytes, accepted state projections, content-length shortfall,
invalid-increment failures, DATA and stream WINDOW_UPDATE GOAWAY boundary
failures, empty failure output, and public failure fields.

`http2::core::send_request_headers(state, offset, stream_id, headers,
end_stream)` validates a request header list, encodes it through the public
production HPACK header-block encoder, and emits one kind-`1` HEADERS frame
with END_HEADERS set and END_STREAM selected by the caller. A new outbound
request stream is created with peer-advertised initial window credit, accepted
`content-length`, and either open or half-closed-local lifecycle.
`send_response_headers(...)` and `send_trailers(...)` apply the same immutable
encoding boundary to existing streams using response or trailer validation.
Accepted response HEADERS update the existing stream's `content-length`
metadata without changing the observed body count, and accepted trailers
preserve that content-length metadata while applying local END_STREAM
lifecycle.
Header-list failures, stream-id or lifecycle failures, GOAWAY boundary
failures, peer maximum-header-list-size failures, and HPACK encode failures
expose typed failures with no bytes and no next state. A nonzero peer
`SETTINGS_MAX_HEADER_LIST_SIZE` bounds the encoded HPACK header block before a
HEADERS frame is emitted.

`http2::core::send_push_promise(state, offset, stream_id,
promised_stream_id, headers)` is the server outbound PUSH_PROMISE transition.
It requires an associated open stream, a nonzero server-initiated promised
stream id above the retained promised-stream high-water, peer push still
enabled when advertised, and no active GOAWAY boundary for the associated
stream. It validates the promised request headers, encodes the header block
through the public production HPACK encoder, applies the peer maximum header
list size to the encoded block, applies the peer maximum frame size to the
PUSH_PROMISE payload, emits one kind-`5` frame with END_HEADERS set, records
the promised stream as reserved-local with peer-advertised initial window
credit, and advances the promised-stream high-water only in the returned
state. Endpoint, stream, ordering, disabled-push, GOAWAY, frame-size,
header-list-size, header-list, and HPACK failures expose no bytes and no next
state.
The focused
[`http2-core-outbound-headers`](../../examples/specification/run/http2-core-outbound-headers/)
case records accepted HEADERS and PUSH_PROMISE bytes, created and reserved
stream projections, response-header content-length updates, trailer closure
with preserved content-length state, PUSH_PROMISE high-water projection,
header-list failure fields, GOAWAY boundary rejection, promised-stream
ordering rejection, disabled-push rejection, peer frame-size rejection,
peer header-list-size rejection, endpoint rejection, empty failure output,
and public failure accessors.

`http2::core::send_rst_stream(state, offset, stream_id, error_code)` emits a
kind-`3`, flags-`0` RST_STREAM frame from an existing open outbound stream and
records the target stream as reset in the returned aggregate state. Zero,
idle, closed, and already-reset streams reject with typed protocol failures,
no bytes, and no next state. Integer and frame encoding failures also expose
no bytes and no next state.

`http2::core::send_priority(state, offset, stream_id, dependency_stream_id,
exclusive, weight)` emits a kind-`2`, flags-`0` PRIORITY frame from an existing
open outbound stream while preserving the aggregate state. It rejects stream
zero, idle, closed, reset, self-dependent, and GOAWAY-forbidden streams with
typed protocol failures, no bytes, and no next state. Integer and frame
encoding failures also expose no bytes and no next state.
The focused
[`http2-core-outbound-stream-control`](../../examples/specification/run/http2-core-outbound-stream-control/)
case records accepted RST_STREAM and PRIORITY bytes, reset-state projection,
dependency and GOAWAY failures, empty failure output, and public failure
fields.

Wider outbound ordering is covered by the output-buffer and outbound
transition cases, especially
[`http2-core-output-buffer`](../../examples/specification/run/http2-core-output-buffer/),
[`http2-core-outbound-data-flow`](../../examples/specification/run/http2-core-outbound-data-flow/),
[`http2-core-outbound-headers`](../../examples/specification/run/http2-core-outbound-headers/),
and
[`http2-core-outbound-stream-control`](../../examples/specification/run/http2-core-outbound-stream-control/).

`http2::core::send_local_settings_batch(...)` accepts a caller-ordered batch
of the supported local SETTINGS items:
`SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_ENABLE_PUSH`,
`SETTINGS_MAX_CONCURRENT_STREAMS`, `SETTINGS_INITIAL_WINDOW_SIZE`,
`SETTINGS_MAX_FRAME_SIZE`, `SETTINGS_MAX_HEADER_LIST_SIZE`, and
`SETTINGS_ENABLE_CONNECT_PROTOCOL`. Accepted batches emit exactly one
length-`6 * item_count`, kind-`4`, flags-`0`, stream-`0` SETTINGS frame,
preserve item order in the payload, update the local sent policy for
`SETTINGS_ENABLE_PUSH` and `SETTINGS_ENABLE_CONNECT_PROTOCOL` with the
six-byte item offset, and record exactly one outstanding local SETTINGS batch
in the caller-supplied `SettingsAckState`. An empty batch emits the zero-length
non-ACK SETTINGS frame, records one outstanding empty batch, and leaves local
policy unchanged.

Local `SETTINGS_ENABLE_PUSH` and `SETTINGS_ENABLE_CONNECT_PROTOCOL` accept
only `0..1`; `SETTINGS_INITIAL_WINDOW_SIZE` accepts `0..2147483647`;
`SETTINGS_MAX_FRAME_SIZE` accepts `16384..16777215`; and the remaining
four-byte SETTINGS values accept `0..4294967295`. A client endpoint cannot
send `SETTINGS_ENABLE_CONNECT_PROTOCOL`. Validation and encoding failures
return a typed decision with no output bytes and without exposing a next
policy or next ACK state.

`http2::core::validate_peer_settings_payload(...)` validates a complete
already frame-shaped non-ACK peer SETTINGS payload without applying accepted
values to caller-owned state. It recognizes
`SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_ENABLE_PUSH`,
`SETTINGS_MAX_CONCURRENT_STREAMS`, `SETTINGS_INITIAL_WINDOW_SIZE`,
`SETTINGS_MAX_FRAME_SIZE`, `SETTINGS_MAX_HEADER_LIST_SIZE`, and
`SETTINGS_ENABLE_CONNECT_PROTOCOL`; unknown identifiers are ignored. Known
items, including duplicates, are inspected in wire order and the first invalid
item is reported at its exact absolute item offset with its six-octet preview.
`SETTINGS_ENABLE_PUSH` and `SETTINGS_ENABLE_CONNECT_PROTOCOL` accept only
`0..1`; `SETTINGS_INITIAL_WINDOW_SIZE` accepts `0..2147483647`;
`SETTINGS_MAX_FRAME_SIZE` accepts `16384..16777215`; and the remaining
supported four-byte values accept the full unsigned wire range. A client
endpoint rejects peer `SETTINGS_ENABLE_PUSH`, and a server endpoint rejects
peer `SETTINGS_ENABLE_CONNECT_PROTOCOL`. Rejection returns typed value-range
or endpoint-role failures with stable diagnostic ids, setting metadata,
provenance, and no partial success.

`http2::core::empty_peer_settings_state()` creates immutable peer advertised
SETTINGS state with protocol defaults for active values: maximum frame size
`16384`, initial stream window `65535`, maximum concurrent streams
`2147483647`, and HPACK header-table size `4096`. Explicit projections return
`0` for absent advertised values and offsets.

`http2::core::apply_peer_settings_payload(state, payload_offset, payload)`
applies a complete, already validated non-ACK peer SETTINGS payload to caller
state. Unknown identifiers are ignored. Known duplicate items are applied in
wire order, so the last known item leaves the active advertised value.
Recorded item offsets are absolute payload offsets, including independent
offsets for `SETTINGS_ENABLE_PUSH` and
`SETTINGS_ENABLE_CONNECT_PROTOCOL` when both are present. A payload whose byte
count is not divisible by six leaves the input state unchanged. The
peer-created stream high-water projection is updated only through
`peer_settings_with_highest_peer_created_stream_id(...)`, keeping SETTINGS
application separate from stream admission.

`http2::core::empty_peer_stream_admission()` creates immutable peer-created
stream admission state with no recorded high-water stream id.
`record_new_headers_peer_stream(kind, stream_id, completed_is_trailer,
peer_stream_is_known, state)` records only new non-trailer HEADERS streams
that are not already tracked by the caller's stream state. It leaves trailers,
known streams, and non-HEADERS frames unchanged. `validate_new_peer_stream(...)`
accepts a candidate stream id only when no previous peer-created stream exists
or the candidate is greater than the recorded high-water id. Rejection returns
a `CorePeerStreamAdmissionFailure` containing the offset, candidate stream id,
previous stream id, endpoint role, active-state label, rule provenance, and
exact caller-supplied preview without exposing a next state.

`http2::core::empty_settings_ack_state()` creates immutable state with no
outstanding local SETTINGS batch and no pending peer SETTINGS ACK. Local
SETTINGS senders record each already validated and emitted batch through
`record_local_settings_batch(...)`, which keeps the first setting identifier
and item count for FIFO acknowledgement projections. `accept_settings_ack(...)`
accepts a validated peer SETTINGS ACK only when a local batch is outstanding;
it removes exactly the oldest batch. Without an outstanding batch, it returns
`SettingsAckFailure` with the stable
`http2.protocol.unexpected_settings_ack` id, offset, active-state label, rule
provenance, and caller-supplied preview without exposing a next state.

`settings_ack_after_peer_frame(...)` records one pending outbound ACK after a
validated non-ACK peer SETTINGS frame with payload items and coalesces later
peer SETTINGS frames into the same pending intent. `send_pending_settings_ack`
returns a no-pending action with no bytes when there is no intent; otherwise
it emits exactly `000000040100000000` and clears only the peer-ACK side of the
state. The local outstanding queue, peer advertised SETTINGS values, HPACK
state, flow-control state, stream state, and shutdown state remain caller-owned
and separate from this acknowledgement state.

The adjacent
[`core_test.veln`](../../crates/veln-stdlib/veln/http2/core_test.veln) checks
one-below, exact, and one-above boundaries where distinct, the complete
HEADERS and PUSH_PROMISE flag matrix, unconstrained kinds, exact failure data,
preview preservation, and absence of success output on rejection. Focused
payload-length human and JSON cases project the active-state label and the
failure's stored preview through
`http2::diagnostic`; focused receive-frame cases retain wider decode ordering.
The same adjacent test checks flow-control domain boundaries, immutable debit
and refill transitions, negative stream-credit preservation, overflow
failures, and input preservation. The focused
[`http2-core-flow-control-domains`](../../examples/specification/run/http2-core-flow-control-domains/)
case imports `http2::core` from `std` and records public boundary, failure,
debit, refill, and input-preservation projections.
The same adjacent test checks exact PING request bytes, seven- and nine-octet
rejection, failure preview and output preservation, exact ACK bytes, payload
preservation, and received-ACK no-response behavior. The focused
[`http2-core-ping-transitions`](../../examples/specification/run/http2-core-ping-transitions/)
case imports `http2::core` from `std` and records the public request, ACK,
no-response, representative failure projections, and emitted bytes.
The adjacent test also checks SETTINGS ACK FIFO acknowledgement, unexpected ACK
failure context, independent local and peer ACK state, peer ACK coalescing,
no-pending behavior, exact emitted bytes, encode-failure output preservation,
and failure/input preservation. The focused
[`http2-core-settings-ack-state`](../../examples/specification/run/http2-core-settings-ack-state/)
case imports `http2::core` from `std` and records public success, no-pending,
representative failure, FIFO, coalescing, and exact-byte projections.
The same adjacent test checks every supported local SETTINGS item as one
bounded set, exact accepted bytes, ordered multi-item batches, local policy
offsets, empty-batch ACK tracking, ACK queue integration, endpoint role
rejection, and immutable failure/output behavior. The focused
[`http2-core-local-settings-send`](../../examples/specification/run/http2-core-local-settings-send/)
case imports `http2::core` from `std` and records public result and
output-chunk projections for accepted and rejected local SETTINGS sends.
The adjacent test also checks peer SETTINGS supported-setting boundaries,
unknown items, duplicate ordering, first-failure precedence, exact failure
context, endpoint-role failures, and immutable failure input. The focused
`http2-protocol-core-settings-value-{human,json}` and
`http2-protocol-core-settings-enable-push-role-{human,json}` cases obtain the
public typed failures through `http2::core` before projecting them through
`http2::diagnostic`.
The adjacent test also checks peer SETTINGS state defaults, known item
application, unknown item preservation, duplicate last-value behavior, partial
payload state preservation, and peer-created stream high-water immutability.
The focused
[`http2-core-peer-settings-state`](../../examples/specification/run/http2-core-peer-settings-state/)
case imports `http2::core` from `std` and records the public state
projections.
The adjacent test also checks peer-created stream admission recording,
monotonic high-water updates, ignored trailer and known-stream cases, empty
and higher-id acceptance without advancing caller-owned high-water state,
non-increasing stream-id rejection, exact failure data, preview preservation,
and immutable input state. The focused
[`http2-core-peer-stream-admission`](../../examples/specification/run/http2-core-peer-stream-admission/)
case imports `http2::core` from `std` and records public state, decision, and
failure projections.
The adjacent test also checks aggregate connection state defaults for server
and client roles, empty stream collection defaults, immutable component
replacement, and stream collection replacement. The focused
[`http2-core-connection-state`](../../examples/specification/run/http2-core-connection-state/)
case imports `http2::core` from `std` and records public aggregate state,
HPACK table, SETTINGS ACK, peer stream admission, stream collection,
flow-control credit, and lifecycle projections.
The focused
[`http2-core-stream-collection`](../../examples/specification/run/http2-core-stream-collection/)
case records stream collection add, replace, lookup, missing-update,
active-count, lifecycle, credit, content-length, immutable input, and
entry-construction failure projections. It also records public lifecycle
active-state, receive-window, open-projection, frame-admission, rejection-rule,
and reset-error-code projections without depending on the aggregate connection
case.
The adjacent test also checks every missing-stream update helper as a no-op
and verifies that embedding the resulting collection in `CoreConnectionState`
preserves the caller-owned stream data.

`http2::core::validate_inbound_frame_kind(offset, frame_kind, stream_id,
streams, preview)` is the standard-owned pure admission boundary for the
receive dispatcher before payload-specific state is applied. It accepts
connection-level SETTINGS, PING, GOAWAY, WINDOW_UPDATE, and unknown extension
frames on stream zero; rejects other known frame kinds on stream zero with the
`connection_frames_require_settings` context; accepts HEADERS before stream
collection lookup; applies DATA, RST_STREAM, WINDOW_UPDATE, PRIORITY, and
unknown extension admission to the current stream lifecycle; accepts PRIORITY
for idle streams; and accepts PUSH_PROMISE only on client-push-associated
streams. Rejections expose the stable `http2.protocol.invalid_frame_kind` id,
offset, actual kind, stream id, expected kind, active-state label, rule
provenance, and supplied frame-header preview, without returning a next state
or mutating the input collection.

The adjacent test checks connection-control acceptance, unknown extension
handling, idle stream rejection, lifecycle-specific DATA, WINDOW_UPDATE,
PRIORITY, and PUSH_PROMISE admission, exact failure context, immutable stream
collection preservation, and preview preservation. The focused
[`http2-core-stream-frame-admission`](../../examples/specification/run/http2-core-stream-frame-admission/)
case imports `http2::core` from `std` and records public decision and failure
projections.

The adjacent test checks remote END_STREAM transitions from open and
half-closed-local streams, non-END_STREAM preservation, RST_STREAM reset-code
projection, immutable input-state preservation, and missing-stream failure
context. The focused
[`http2-core-stream-lifecycle-transitions`](../../examples/specification/run/http2-core-stream-lifecycle-transitions/)
case imports `http2::core` from `std` and records public state, failure, and
wrong-variant accessor projections.

`http2::core::apply_data_receive_flow_control(state, offset, stream_id,
data_length, preview)` is the standard-owned immutable DATA receive
flow-control transition over `CoreConnectionState`. It looks up the target
stream collection entry and, on success, returns a new aggregate state with
both connection receive credit and that stream's receive credit debited by the
DATA length. Connection-window failure, stream-window failure, and missing
stream failure expose stable public ids, offset, stream id, DATA length,
domain, observed credit, original credit values, and supplied frame-header
preview. Rejections expose no next state and leave the input aggregate,
stream collection, and preview unchanged.

The adjacent test checks immutable success, connection-window failure,
stream-window failure, missing-stream failure, exact failure context, original
credit preservation, and preview preservation. The focused
[`http2-core-data-receive-flow-control`](../../examples/specification/run/http2-core-data-receive-flow-control/)
case imports `http2::core` from `std` and records the public result-state and
failure projections.

`http2::core::apply_stream_window_update_flow_control(state, offset,
stream_id, increment, preview)` is the standard-owned immutable stream-level
WINDOW_UPDATE receive transition over `CoreConnectionState`. It looks up the
target stream collection entry and, on success, returns a new aggregate state
with that stream's send credit refilled by the checked WINDOW_UPDATE
increment. Invalid zero or oversized increments, stream-window overflow, and
missing stream failure expose stable public ids, offset, stream id, increment,
domain, observed value, original stream credit, and supplied frame-header
preview. Rejections expose no next state and leave the input aggregate,
stream collection, send credit, and preview unchanged.

The adjacent test checks immutable success, invalid increment,
stream-window overflow, missing-stream failure, exact failure context,
original send-credit preservation, and preview preservation. The focused
[`http2-core-stream-window-update-flow-control`](../../examples/specification/run/http2-core-stream-window-update-flow-control/)
case imports `http2::core` from `std` and records the public result-state and
failure projections. Full receive-frame WINDOW_UPDATE integration is covered by
[`http2-core-receive-frame-dispatch`](../../examples/specification/run/http2-core-receive-frame-dispatch/).

`http2::core::apply_connection_window_update_flow_control(state, offset,
increment, preview)` is the standard-owned immutable connection-level
WINDOW_UPDATE receive transition over `CoreConnectionState`. On success it
returns a new aggregate state with the connection receive credit refilled by
the checked WINDOW_UPDATE increment. Invalid zero or oversized increments and
connection-window overflow expose stable public ids, offset, increment,
domain, observed value, original connection credit, and supplied frame-header
preview. Rejections expose no next state and leave the input aggregate,
stream collection, connection credit, and preview unchanged.

The adjacent test checks immutable success, invalid increment,
connection-window overflow, exact failure context, original connection-credit
preservation, stream collection preservation, and preview preservation. The
focused
[`http2-core-connection-window-update-flow-control`](../../examples/specification/run/http2-core-connection-window-update-flow-control/)
case imports `http2::core` from `std` and records the public result-state and
failure projections.

`http2::core::validate_request_header_list(headers, enable_connect_protocol)`,
`validate_response_header_list(headers, completed_end_stream)`, and
`validate_trailer_header_list(headers, active_state)` validate a completed
public `http2::hpack::HeaderList` without changing HPACK table state, stream
state, or frame input. Accepted request and response lists return the accepted
`content-length` value, or `-1` when no accepted content length is present.
The request validator receives the active `SETTINGS_ENABLE_CONNECT_PROTOCOL`
value used for extended CONNECT negotiation. Failures expose a stable failed
fact, the selected header name, and the request, response, or trailer
active-state label.

The pure boundary covers duplicate pseudo-headers, pseudo-headers after
regular headers, request-only and response-only pseudo-headers, ordinary
request required pseudo-headers, CONNECT and extended CONNECT request shape,
status value and informational END_STREAM response rules, trailer
pseudo-header rejection, ordinary header lowercase and token rules,
connection-specific header rejection, `te` value validation, and
`content-length` invalid or mismatched values. HPACK-carrying receive
transitions are covered by `apply_receive_frame(...)`, outbound
content-length send accounting is covered by the outbound send transitions,
and diagnostic rendering remains in focused human and JSON cases. The adjacent
test checks pure
success and failure facts for request, response, and trailer lists. The
focused executable
[`http2-core-header-list-validation`](../../examples/specification/run/http2-core-header-list-validation/)
case imports `http2::core` and `http2::hpack` from `std` and records accepted
content-length values plus representative public failure facts, selected
header names, and active-state labels. The focused
[`http2-core-header-list-validation`](../../examples/specification/check/http2-core-header-list-validation/)
case imports `http2::core` from `std` and checks that the public validation
surface is available to external packages.

The adjacent test also checks that `apply_receive_frame(...)` composes frame
decode, payload-length validation, stream-frame admission, DATA END_STREAM
lifecycle application, HEADERS and CONTINUATION header-block completion,
PUSH_PROMISE promised-stream reservation, production HPACK decode,
header-list validation, DATA receive-credit debit, inbound DATA
content-length body accounting,
stream and connection WINDOW_UPDATE credit refill, RST_STREAM reset-code
application, pending-header-block sequence rejection, wrong-variant accessors,
and immutable failure-state preservation. The focused
[`http2-core-receive-frame-dispatch`](../../examples/specification/run/http2-core-receive-frame-dispatch/)
case imports `http2::core` from `std` and records public success, state,
failure-source, failure-id, HPACK failure, header-list failure,
content-length mismatch, PUSH_PROMISE reserved-stream and high-water
projections, stream-admission active-state and rule provenance, and accessor
projections. Outbound content-length send accounting and emitted-byte ordering
are covered by the focused outbound and output-buffer cases named below.

`http2::core::empty_connection_preface(starting_offset)` creates immutable
state for the 24-octet client connection preface.
`accept_connection_preface(state, input)` accepts a complete preface in one
chunk or retains its matched prefix across arbitrary chunk boundaries. A
successful transition reports completion and exposes every trailing input
octet through `connection_preface_suffix(...)` for the later initial-SETTINGS
transition.

A mismatch failure reports its absolute offset, expected and actual octets,
matched count, and preview input. `close_connection_preface(state)` reports a
distinct partial-preface failure with the original starting offset, pending
count, and preview. Failures expose neither a next state nor a consumable
suffix, and the input state remains unchanged. The adjacent
[`core_test.veln`](../../crates/veln-stdlib/veln/http2/core_test.veln) checks
complete, byte-by-byte, unevenly chunked, trailing-input, first, middle, and
final mismatch, partial-close, and immutable failure-state behavior. The
focused `http2-protocol-core-preface-{invalid,partial}-{human,json}` cases
obtain these public failures and project them through `http2::diagnostic`;
initial-SETTINGS integration is covered by
[`http2-initial-peer-settings-gate-human`](../../examples/specification/run/http2-initial-peer-settings-gate-human/)
and
[`http2-initial-peer-settings-gate-json`](../../examples/specification/run/http2-initial-peer-settings-gate-json/).

`http2::core::server_initial_peer_settings_gate()` and
`client_initial_peer_settings_gate()` create immutable role-specific state for
the first complete peer frame. `accept_initial_peer_settings(...)` accepts
only a non-ACK SETTINGS frame. A successful transition exposes the accepted
next state and retains the endpoint role.

A non-SETTINGS frame or initial SETTINGS ACK returns an
`InitialPeerSettingsFailure` with the stable diagnostic id, offset, frame kind,
flags, stream id, endpoint role, active-state label, rule provenance, and exact
supplied frame-header preview. Rejection exposes no transition or next state
and leaves the input state and preview unchanged. Frame-header completeness,
maximum-frame-size validation, stream-id and SETTINGS payload validation, and
SETTINGS value application remain separate transition stages.

The adjacent
[`core_test.veln`](../../crates/veln-stdlib/veln/http2/core_test.veln) checks
accepted server and client roles, acceptance without mutating the input state
or preview, non-SETTINGS and ACK rejection, exact failure context, and
immutable failure state and preview preservation. The focused
`http2-initial-peer-settings-gate-{human,json}` cases obtain the typed public
failure and project it through `http2::diagnostic`. Frame-size, stream-id,
payload, SETTINGS-value, and state integration are covered by focused
`http2-core-*` and `http2-protocol-core-*` cases.

`http2::core::empty_pending_header_block()` constructs idle continuation
state. `start_header_block(...)` accepts an already validated HEADERS or
PUSH_PROMISE fragment. END_HEADERS completes the block immediately; otherwise
the returned immutable state retains the initiating stream, frame kind,
offset, flags, trailer classification, promised stream id, and accumulated
octets.

`continue_header_block(...)` accepts only CONTINUATION on the initiating
stream, appends fragments in wire order, and exposes a completed block only
after END_HEADERS. Completion preserves END_STREAM and trailer status from
HEADERS or the promised stream id from PUSH_PROMISE. Non-final transitions
expose no completed block. `close_pending_header_block(...)` accepts idle
input and rejects closure while a block remains active.

Typed failures distinguish a different frame kind, a different stream, and
closed input. They expose the current offset and frame coordinates, initiating
coordinates, expected stream, accumulated byte count, rule provenance, and
preview octets without exposing a next state or completed block. The input
state remains unchanged. Adjacent
[`core_test.veln`](../../crates/veln-stdlib/veln/http2/core_test.veln) coverage
checks immediate, multi-frame HEADERS, multi-frame PUSH_PROMISE, non-final,
wrong-kind, wrong-stream, active and idle closed-input paths, and exact
diagnostic-input preservation. The focused
`http2-protocol-core-continuation-*` cases project the public failures through
the stable human and JSON diagnostics. Frame decoding, HPACK decoding,
stream-lifecycle, and output integration are covered by focused
`http2-core-*` cases.

`http2::hpack::encode_integer(value, prefix_bits, representation_bits)` accepts
a non-negative `Int` and a prefix width from one through eight. It preserves
the caller-supplied high representation bits in the first octet and returns
the finite HPACK continuation encoding as a `ByteChunk`.
`http2::hpack::decode_integer(input, prefix_bits)` uses the same width contract
and reports the decoded value plus the consumed octet count. Empty input,
invalid widths, incomplete continuations, and encodings beyond the `Int` range
are rejected. The canonical multi-octet encoding and representation-bit
behavior are checked by
`../../examples/specification/run/hpack-prefixed-integer-codec/`.

`http2::hpack::encode_huffman(bytes)` encodes arbitrary `ByteChunk` octets
with the HPACK static Huffman table and the required EOS-prefix padding.
`decode_huffman(input)` returns the exact decoded `ByteChunk`, including
non-visible octets. It rejects EOS as a payload symbol, invalid or overlong
padding, and truncated or invalid code sequences. A failure returns no partial
decoded value. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks canonical vectors, every single octet, recursive input, padding
boundaries, and rejection paths. The focused
[`hpack-huffman-codec`](../../examples/specification/run/hpack-huffman-codec/)
case records the public facade's encoded and decoded octets and representative
failures.

`http2::hpack::static_entry(index)` exposes every one-based HPACK static-table
entry from 1 through 61; `static_entry_name(entry)` and
`static_entry_value(entry)` project its exact fields.
`static_entry_index(name, value)` returns the exact entry index, while
`static_name_index(name)` returns the first index with the exact name. Indices
outside the table and unknown names or values return `None`. The complete
forward and reverse contract is checked by the adjacent standard-library
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln).

`http2::hpack::empty_dynamic_table(capacity)` creates an immutable empty table
and rejects a negative capacity. `insert_dynamic_table_entry(table, name,
value)` inserts at index one, keeps entries in newest-to-oldest order, and
accounts for each entry as the name octet count plus the value octet count plus
32. It evicts the oldest entries until the result fits; an entry larger than
the active capacity clears the result table. Header values are `ByteChunk`
values and preserve arbitrary octets.

`dynamic_table_with_capacity(table, capacity)` returns a new table, evicting
after a shrink and retaining entries after a grow, and rejects a negative
capacity. Successful and failed transitions leave the input table unchanged.
The capacity, current size, and entry count have dedicated projections.
`dynamic_table_entry(table, index)` performs one-based lookup and returns
`None` for non-positive or unavailable indices; `dynamic_entry_name` and
`dynamic_entry_value` project a found entry. The adjacent standard-library
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks exact size accounting, insertion order, eviction, capacity changes,
lookup boundaries, arbitrary-octet values, and input-state preservation. The
focused
[`hpack-dynamic-table-state`](../../examples/specification/run/hpack-dynamic-table-state/)
case checks the same facade from an external package and records its projected
state and octet values through command output.

`header_field(name, value)`, `empty_header_list()`, and
`prepend_header_field(header, remaining)` construct an ordered encode input
while preserving every value as an exact `ByteChunk`.
`encode_indexed_header_field(header, index, table)` validates that the selected
static or newest-first dynamic entry exactly matches the field before emitting
the full seven-bit-prefixed indexed representation.
`encode_literal_header_field(header, representation, name_index,
huffman_name, huffman_value, table)` emits one explicitly selected literal.
Representation `0` means incremental indexing, `1` means without indexing, and
`2` means never indexed. Name index zero emits the direct name; other indices
must resolve to the field's exact static or dynamic name. The two Boolean
selectors independently choose raw or HPACK Huffman encoding for a direct name
and the value.

`encode_header_block(headers, table, active_capacity)` recursively encodes any
finite `HeaderList` in order. Its deterministic policy uses an exact static
entry first, then an exact dynamic entry; otherwise it emits an
incrementally-indexed literal with a static name, dynamic name, or direct name
in that order. Each string uses Huffman only when the complete Huffman literal
is shorter than its raw literal, so ties remain raw. A successful insertion is
available to later fields in the same block. When the supplied table capacity
exceeds `active_capacity`, the block starts with the required table-size update
and applies immutable oldest-first eviction before encoding fields.

Header encode transitions expose only complete bytes and the next immutable
table. Typed failures distinguish invalid representations or names, zero and
unavailable indices, indexed-field mismatches, integer or string encoding, and
table transitions. Block failures add the zero-based field position and active
capacity selection. A failure exposes no partial bytes or next state and
leaves the input list and table unchanged. Invalid representations and names,
zero indices, unavailable static and dynamic indices, indexed-field
mismatches, invalid active capacity, and nested field failures are reachable
through public encoder calls. The integer, string, and table failure variants
are defensive mappings for private codec failures that valid public values
cannot produce. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks exact static and multi-octet dynamic bytes, the complete literal-form by
name-source matrix, raw and Huffman strings, empty and non-visible values,
in-block reuse, capacity eviction, list boundaries, decode-after-encode
behavior, every reachable failure, and input preservation. The focused
[`hpack-header-block-encoding`](../../examples/specification/run/hpack-header-block-encoding/)
case records public encoded bytes, ordered decoded values, next-state
projections, and representative typed failures.

`http2::hpack::decode_table_size_update(input, table, peer_maximum)` decodes
one `001xxxxx` dynamic table-size update with the five-bit-prefixed integer
codec. The transition reports the requested capacity, consumed octet count,
and next immutable table. Shrinking evicts oldest entries through the ordinary
capacity transition, while growing retains entries.

The typed `TableSizeUpdateFailure` distinguishes a different representation,
a malformed integer, an incomplete integer, and a capacity above the explicit
peer-advertised maximum. Capacity-limit projections report both the requested
capacity and the peer maximum. Every failure contains no next table and leaves
the input table unchanged. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks boundary and multi-octet values, shrink and growth transitions, every
failure class, and state preservation. The focused
[`hpack-table-size-update`](../../examples/specification/run/hpack-table-size-update/)
case records public transition values and representative typed failures.

`http2::hpack::decode_indexed_header_field(input, table)` decodes one HPACK
indexed header-field representation with the full seven-bit-prefixed integer.
Indices 1 through 61 resolve through the static table. Larger indices resolve
through the supplied immutable dynamic table, where index 62 selects its
newest entry. The transition reports the consumed octet count, decoded name
and exact value `ByteChunk`, and the unchanged dynamic table.

The typed `IndexedDecodeFailure` distinguishes malformed and incomplete
integers, index zero, unavailable static entries, and unavailable dynamic
entries. Failure projections expose a stable failure kind and the requested
table coordinates where applicable; a failure contains neither a decoded
header nor a next table. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks every static entry, single- and multi-octet dynamic indices, arbitrary
value octets, all reachable failure classes, and state preservation. The
focused
[`hpack-indexed-header-field`](../../examples/specification/run/hpack-indexed-header-field/)
case checks the public facade from an external package.

`http2::hpack::decode_literal_header_field(input, table)` decodes one literal
header field with incremental indexing, without indexing, or marked never
indexed. A zero name index reads a raw or Huffman string name; a nonzero name
index resolves through the static or immutable dynamic table with the
representation's full prefixed integer. The value is another raw or Huffman
string and is returned as an exact `ByteChunk`, including non-visible octets.
The transition identifies the representation and reports its decoded field,
consumed octet count, and next table. Incremental indexing inserts the field
into the next table; the other representations return the unchanged table.

The typed `LiteralDecodeFailure` distinguishes malformed and incomplete name
indices, unavailable indexed names, malformed and incomplete name or value
lengths, truncated raw name or value octets, invalid name octets, and name or
value Huffman failures. Truncation projections report the expected and
available octet counts, while unavailable dynamic-name projections report the
requested table coordinates. Every failure contains neither a decoded field
nor a next table and leaves the input table unchanged. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks all representations, direct and indexed names, raw and Huffman strings,
multi-octet indices and lengths, exact value octets, insertion, and failure
preservation. The focused
[`hpack-literal-header-field`](../../examples/specification/run/hpack-literal-header-field/)
case records public raw result values and representative typed failures.

`http2::hpack::decode_header_block(input, table, peer_maximum)` recursively
decodes a complete ordered block of indexed and literal fields. `HeaderList`
keeps wire order, and every `HeaderField` retains its value as an exact
`ByteChunk`. The transition reports the full list, total consumed octets, and
next immutable table, so an incrementally indexed field is available to later
fields in the same block and to a later decode.

One or more bounded table-size updates may lead the block. An update after the
first field is a focused misplaced-update failure. Indexed, literal, and
table-size-update codec failures remain available as their existing typed
families beneath the block failure. A failure exposes neither a partial list
nor a next table and leaves the caller's input table unchanged. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks empty and mixed blocks, field order, exact non-visible octets, dynamic
transitions across fields and decodes, every literal representation, list
boundaries, update-only and leading-update blocks, nested failures, and
failure-state preservation.
The focused
[`hpack-header-block-decoding`](../../examples/specification/run/hpack-header-block-decoding/)
case records public result values and representative failure kinds.

Additional executable evidence lives in the adjacent standard-library
`*_test.veln` files and in the focused HTTP/2 cases under
`../../examples/specification/`.
`http2::core::empty_output_buffer()` creates immutable output state for
sans-I/O write ordering. Public append helpers accept send decisions for PING,
peer SETTINGS ACK, local SETTINGS, GOAWAY, DATA, WINDOW_UPDATE, HEADERS,
PUSH_PROMISE, RST_STREAM, and PRIORITY. Accepted decisions append exactly one
emitted byte chunk at the end of the buffer. Rejected, encode-failed,
no-pending, and no-response decisions return the caller-owned buffer
unchanged. Projection helpers expose chunk count, zero-based chunk lookup, and
concatenated bytes. The focused
[`http2-core-output-buffer`](../../examples/specification/run/http2-core-output-buffer/)
case records ordered chunks, combined bytes, and failure/no-response
non-append behavior through the public facade.
The broad `http2-protocol-core` implementation is retired and is not current
HTTP/2 behavior. Focused `http2-core-*` cases cover state transitions,
emitted bytes, and failure atomicity through the public core. Focused
`http2-protocol-core-*` cases remain only where their human and JSON
diagnostic projections are current observable behavior. Standard-package
`frame_test.veln`, `diagnostic_test.veln`, `hpack_test.veln`, and
`core_test.veln` cover the public modules without reading the retired fixture
or a historical migration manifest. The migration-only inventories,
generator, checker, and generated retirement tests were removed after this
independent coverage passed. The completion boundary is recorded in
[`http2-standard-library-completion-and-fixture-retirement.md`](../reference/implemented-proposals/http2-standard-library-completion-and-fixture-retirement.md).
