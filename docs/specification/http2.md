---
role: specification
authority: normative
specification-coverage: usage=#public-modules; behavior=#core-domains-and-transitions; limits=#shutdown
update-when: The HTTP/2 standard-module API, protocol behavior, diagnostics, or checked HTTP/2 evidence changes.
---

# HTTP/2 Standard Modules

## Public modules

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
- `http2::connection`: the `drive_server`, `drive_client`, and
  `drive_server_application` duplex-stream connection drivers,
  `serve_connection` and `serve_tcp` server service boundaries,
  `request_endpoint_sequence` client service boundary, typed protocol-owned
  connection failures, immutable application response actions, typed
  application-boundary failures for one caller-owned stream, immutable client
  request and response observations, and typed service failures for
  service-owned listeners, client streams, tasks, and streams.

Nested implementation modules below `http2::hpack` and `http2::core` are not
package exports.
The JVM adapter keeps its intrinsic link names private; source code calls only
the module-qualified API. Diagnostic ids, human rendering, and
`details.protocol_diagnostic` projections remain stable.

## Core domains and transitions

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
output-buffer state. The receive state also retains immutable application
events produced by accepted complete request-header blocks.
`receive_connection_chunk(...)` appends the supplied input chunk, consumes the
server connection preface when required, buffers partial frame bytes, enforces
the initial peer SETTINGS gate before the first accepted frame, then applies
each complete buffered frame in receive order through `apply_receive_frame(...)`
until the buffer is empty, only a partial frame remains, or a rejection occurs.
Accepted non-ACK peer SETTINGS frames with a recorded pending ACK append an
outbound SETTINGS ACK through the output buffer in receive order and clear
only the pending peer-ACK state.
Accepted non-ACK PING frames append the exact PING ACK bytes after any earlier
output, while received PING ACKs append no bytes. Accepted PRIORITY frames
advance the aggregate offset and preserve stream and output state. Accepted
PUSH_PROMISE frames reserve the promised stream through the aggregate receive
dispatcher without appending output bytes or mutating caller-owned output.
When a complete request HEADERS block is accepted, decoded, validated as a
request header list, and committed through the stream transition, the receive
state retains one `Http2RequestHeaders(stream_id, headers, end_stream)` event.
A request HEADERS block without `END_STREAM` still produces the event.
Incomplete HEADERS blocks retain no event until the final CONTINUATION
completes the block and the core accepts it. Response headers, trailers,
PUSH_PROMISE, DATA, PING, and other unrelated frames retain no request-header
event. Rejected HPACK, header-list, and stream-transition decisions return the
existing focused core failure and do not expose an event from the rejected
block.
`http2::core::drain_application_events(state)` returns the retained events in
receive order with a next receive state that retains none of those returned
events. Draining that next state again returns an empty event list. The input
state remains usable as an immutable value.
Rejections from the preface gate, initial SETTINGS gate, frame decode, or frame
dispatcher, including after an earlier complete frame in the same input chunk
has advanced HPACK, continuation, DATA flow-control and content-length, or
shutdown state, expose a focused failure source and do not expose a next
chunked receive state, preserving the caller-owned connection, buffered input,
and output values, including any output chunks supplied by the caller.

`http2::core::finalize_receive_connection_eof(...)` finalizes a chunked
receive state after clean transport end. It accepts only when the connection
preface is complete, the initial peer SETTINGS gate has accepted a complete
SETTINGS frame, no partial frame bytes remain, and no pending header block is
active. Accepted EOF returns the immutable aggregate connection state.
Rejected EOF returns a typed incomplete-input failure that identifies the
pending source: connection preface, initial peer SETTINGS, frame bytes, or
pending header block. The failure exposes offset, pending count, expected
count when applicable, frame kind, stream id, reason, and preview facts.

## Connection drivers and services

`http2::connection::drive_server(state)` drives one server-side HTTP/2
connection through `transport::DuplexStream`. If the supplied core lifecycle is
closed, the driver returns that state without reading, writing, sending local
SETTINGS, or requesting a protocol transition. Otherwise the driver first
sends the initial empty server SETTINGS through the existing local SETTINGS
send transition, commits the returned core state, and writes those bytes
before reading peer input.

`http2::connection::drive_client(state)` drives one client-side HTTP/2
connection through `transport::DuplexStream`. If the supplied core lifecycle is
closed, the driver returns that state without reading, writing, sending local
SETTINGS, or requesting a protocol transition. Otherwise the driver first
writes the HTTP/2 client connection preface, then writes the initial client
SETTINGS bytes produced by the existing local SETTINGS send transition. It
commits the returned core state before reading peer input.

For each `Some(chunk)` read, the driver delegates to
`http2::core::receive_connection_chunk(...)`. Accepted transitions commit the
returned receive state and write each newly accepted output chunk exactly once
in output-buffer order. Rejected transitions return
`Http2ConnectionProtocolFailure` with the focused core failure facts and do
not write output from the rejected transition; bytes from earlier accepted
driver iterations remain committed. On clean end, the driver delegates to
`finalize_receive_connection_eof(...)` and returns either the accepted final
core state or `Http2ConnectionIncompleteInput`.

`http2::connection::Http2ApplicationAction` is the public immutable response
action value for the server application boundary. It has
`Http2SendResponseHeaders(stream_id, headers, end_stream)` and
`Http2SendResponseData(stream_id, data, end_stream)` constructors. These
values carry only response data: request stream id, HPACK header list,
payload bytes, and END_STREAM state. They do not carry transport handles,
mutable host state, or HTTP/2 core state.

`http2::connection::drive_server_application(state, handler)` drives the same
server-side duplex-stream transport as `drive_server` and does not change the
existing server or client driver contracts. The handler has shape
`fn(Http2ApplicationEvent) -> Result<List<Http2ApplicationAction>, String>
effects [...E]`, and the driver has the public effect boundary
`[std::transport::DuplexStream, ...E]`. A pure callback therefore leaves only
the duplex-stream effect on the driver expression. An effectful callback adds
its row to the driver expression. Handling the driver with
`transport::net::net_stream` replaces only the duplex-stream effect with
`net`; callback effects remain declared by the caller.
The driver drains core application events exactly once. In this bounded slice
it accepts at most one request whose completed HEADERS also set END_STREAM.
It invokes the callback exactly once for that request. A body-bearing request
and a second request return distinct application-boundary failures and are
not classified as core protocol failures.

Before writing response bytes, `drive_server_application` validates the
complete callback action list against the bounded response grammar. The list
must be either one final HEADERS action for the request stream id, or one
non-final HEADERS action followed by one final DATA action for the same
stream id. Empty lists, DATA before HEADERS, mismatched stream ids, missing
final DATA, non-final DATA, and actions after the final action are invalid
application action sequences.

Accepted response actions are applied in list order through
`http2::core::send_response_headers(...)` and
`http2::core::send_data(...)`. Each accepted core action is committed through
the duplex stream before the next core action is attempted. If a later core
action is rejected, earlier committed writes remain visible and the rejected
action and later actions write no bytes. Callback failures write no response
action bytes. Clean EOF and incomplete EOF preserve the same accepted and
incomplete-input behavior as the connection driver. Abrupt runtime transport
failures remain runtime failures rather than typed application failures.

`http2::connection::Http2ServiceFailure` is the public failure value for the
server service boundary. It exposes state creation failure, application
boundary failure with accepted-connection index, and task join failure with
accepted-connection index. Projection helpers expose service failure kind, id,
reason, accepted index, and stream id.

`http2::connection::serve_connection(handler)` creates a server connection
state and drives one caller-owned duplex stream through
`drive_server_application`. Its public effect boundary is
`[std::transport::DuplexStream, ...E]` for the callback row.

`http2::connection::serve_tcp(listener, handler)` owns the supplied listener,
accepted streams, and connection tasks. Each spawned connection task receives
its `NetStream` as explicit context and installs
`transport::net::net_stream` inside the task. Lexical handlers installed
outside `serve_tcp` do not satisfy the spawned connection task boundary. Its
public effect boundary is `[net, concurrency, ...E]`.

For each accepted stream, `serve_tcp` spawns one connection task with that
stream as explicit context, joins the task, and closes the stream once. The
service accepts the next stream only after the current task succeeds. On clean
listener end, `serve_tcp` closes the listener once. On the first ordinary
callback, protocol, or join failure, it preserves that first failure, closes
the failed stream once, closes the listener once, and does not accept or write
a later connection response. Abrupt runtime transport failures, including
transport failures raised inside a spawned connection task, remain runtime
failures rather than `Http2ServiceJoinFailure` values. Later source cleanup
after an abrupt runtime transport failure is not specified.

`http2::connection::Http2ClientRequest` is the immutable public client request
value. The implemented request shape is
`Http2ClientBodylessRequest(headers)`, created by
`client_bodyless_request(headers)`. It carries only a validated HPACK header
list for one bodyless request and exposes no `NetStream`, handler context, or
mutable core state.

`http2::connection::Http2ClientResponse` is the immutable public client
response observation for one final response. Projection helpers expose the
stream id, whether the retained connection was reused for the request, the
connection lifecycle label, the active stream count, and the peer stream
capacity that the service uses for the next reuse decision.

`http2::connection::request_endpoint_sequence(endpoint, requests, handler)`
owns each connected `NetStream` and each spawned request task for a finite
request list to one endpoint. A request task receives the stream, immutable
core state, reuse flag, and request value as explicit task context, then
installs `transport::net::net_stream` inside that task. The application
handler receives only `Http2ClientResponse` values. It does not receive
transport handles, handler context, or mutable core state.

After a successful request, the service may retain the connection for the next
request only when `client_connection_reusable(state)` is true. That projection
is true only for an open connection whose active stream count is less than the
peer stream capacity reported by `http2::core`. Draining and closed
connections are not reusable. An open connection with exhausted peer stream
capacity is not reusable. When the retained connection is not reusable, the
service closes that stream once before connecting a new stream for the next
request.

On the first ordinary request-send, protocol, callback, or join failure,
`request_endpoint_sequence` returns `Http2ClientServiceFailure` with the
completed request count and closes the retained stream once. Abrupt runtime
transport failures remain runtime failures, and later source cleanup after
that failure is not specified. A pure response handler leaves only `net` and
`concurrency` on the service expression. A handler with callback effects adds
that callback row to the same expression.

## Shutdown

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

`http2::core::apply_connection_window_update_flow_control(state, offset,
increment, preview)` is the standard-owned immutable connection-level
WINDOW_UPDATE receive transition over `CoreConnectionState`. On success it
returns a new aggregate state with the connection receive credit refilled by
the checked WINDOW_UPDATE increment. Invalid zero or oversized increments and
connection-window overflow expose stable public ids, offset, increment,
domain, observed value, original connection credit, and supplied frame-header
preview. Rejections expose no next state and leave the input aggregate,
stream collection, connection credit, and preview unchanged.

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

Request validation requires lowercase token names, pseudo-headers before
ordinary headers, one occurrence of each request pseudo-header, and
`te: trailers` when `te` is present. Ordinary requests require `:method`,
`:scheme`, and `:path`; `:authority` is required for `CONNECT`. Ordinary
`CONNECT` rejects `:scheme` and `:path`. When
`SETTINGS_ENABLE_CONNECT_PROTOCOL` is enabled, extended CONNECT requires
`:method: CONNECT`, `:protocol`, `:scheme`, `:path`, and non-empty
`:authority`; duplicate or mixed pseudo-header forms fail. Response validation
requires one three-digit `:status`; informational responses cannot end the
stream and status `101` is rejected. Trailer validation rejects all
pseudo-headers. Header-list size and accepted `content-length` values are
checked by the consuming frame transition.

`http2::core::empty_connection_preface(starting_offset)` creates immutable
state for the 24-octet client connection preface.
`accept_connection_preface(state, input)` accepts a complete preface in one
chunk or retains its matched prefix across arbitrary chunk boundaries. A
successful transition reports completion and exposes every trailing input
octet through `connection_preface_suffix(...)` for the later initial-SETTINGS
transition.

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

### HPACK codecs

`http2::hpack::encode_integer(value, prefix_bits, representation_bits)` accepts
a non-negative `Int` and a prefix width from **1 through 8**. The third
argument is the high representation bits; it must be in `0..=255` and must
have no bits set in the selected prefix. The function emits the minimal HPACK
continuation encoding as a `ByteChunk`, using prefix maximum
`(1 << prefix_bits) - 1`. Negative values, an out-of-range prefix width, or
representation bits that do not fit return an error.

`http2::hpack::decode_integer(input, prefix_bits)` accepts a `ByteView`, uses
the same **1 through 8** width contract, and returns
`Result<{value: Int, consumed: Int}, String>`. Empty input, incomplete
continuations, and continuation values beyond the `Int` range are rejected;
no partial integer is returned. `decoded_integer_value` and
`decoded_integer_consumed` project the two result fields. The continuation
decoder rejects values beyond its supported range rather than wrapping them.

`http2::hpack::encode_huffman(bytes)` and
`http2::hpack::decode_huffman(input)` implement the HPACK static Huffman code.
Encoding accepts a `ByteChunk`; decoding accepts a `ByteView`. Both return
`Result<ByteChunk, String>` and preserve arbitrary octets. Decoding rejects EOS as a
payload symbol, invalid or overlong EOS-prefix padding, and truncated or
invalid code sequences. Invalid input returns no partial decoded value.

`static_entry(index)` exposes one-based entries 1 through 61;
`static_entry_name` and `static_entry_value` project the exact fields.
`static_entry_index(name, value)` returns the exact matching index and
`static_name_index(name)` returns the first index with that name. Unknown
names and values do not select an entry. Dynamic table lookup is one-based and
newest-first.
Capacity and entry-size accounting use name octets plus value octets plus 32;
shrinking evicts oldest entries, and an entry larger than capacity clears the
table. Every transition returns a new immutable table or a typed failure and
leaves its input table unchanged.

`http2::hpack::empty_dynamic_table(capacity)` creates an immutable empty table
and rejects a negative capacity. `insert_dynamic_table_entry(table, name,
value)` inserts at index one, keeps entries in newest-to-oldest order, and
accounts for each entry as the name octet count plus the value octet count plus
32. It evicts the oldest entries until the result fits; an entry larger than
the active capacity clears the result table. Header values are `ByteChunk`
values and preserve arbitrary octets.

`dynamic_table_with_capacity(table, capacity)` returns a new table with the
requested non-negative capacity. A shrink evicts the oldest entries until the
remaining newest entries fit; a grow retains all entries. The public
projections are `dynamic_table_capacity`, `dynamic_table_size`,
`dynamic_table_entry_count`, `dynamic_table_entry`, `dynamic_entry_name`, and
`dynamic_entry_value`. Dynamic entry indices are one-based, with index `1`
newest; non-positive and unavailable indices return `None`. A failed capacity
or insertion operation leaves the caller's immutable table unchanged.

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

The single-field encoder returns `HeaderEncodeTransition(bytes, table)` only
after complete encoding. Its `HeaderEncodeFailure` variants are
`InvalidEncodeRepresentation`, `InvalidEncodeName`, `ZeroEncodeIndex`,
`UnavailableEncodeStaticIndex(index)`,
`UnavailableEncodeDynamicIndex(index, dynamic_index, entry_count)`,
`EncodeIndexedFieldMismatch(index)`, `EncodeIntegerFailure`,
`EncodeStringFailure`, and `EncodeTableFailure`. Static and dynamic index
projections are one-based; dynamic failures additionally expose the requested
dynamic coordinate and current entry count. A failure produces no bytes or
next table and leaves the input field and table unchanged.

The block encoder returns `HeaderBlockEncodeTransition(bytes, table)`. Its
failures are `InvalidActiveCapacity(capacity)`,
`HeaderBlockFieldEncodeFailure(position, failure)`,
`HeaderBlockIntegerEncodeFailure`, and `HeaderBlockTableEncodeFailure`;
`position` is the zero-based field position and nested field failures retain
their `HeaderEncodeFailure`. Block failures produce no partial bytes or next
table and leave the input list and table unchanged.

`http2::hpack::decode_table_size_update(input, table, peer_maximum)` decodes
one `001xxxxx` dynamic table-size update with the five-bit-prefixed integer
codec. The transition reports the requested capacity, consumed octet count,
and next immutable table. Shrinking evicts oldest entries through the ordinary
capacity transition, while growing retains entries.

The `TableSizeUpdateFailure` family is `NotTableSizeUpdate`,
`MalformedTableSizeUpdateInteger`, `IncompleteTableSizeUpdateInteger`, or
`TableSizeUpdateCapacityLimit(capacity, peer_maximum)`. The capacity-limit
projections expose both requested values. Every failure has no next table and
leaves the input table unchanged.

`http2::hpack::decode_indexed_header_field(input, table)` decodes one HPACK
indexed header-field representation with the full seven-bit-prefixed integer.
Indices 1 through 61 resolve through the static table. Larger indices resolve
through the supplied immutable dynamic table, where index 62 selects its
newest entry. The transition reports the consumed octet count, decoded name
and exact value `ByteChunk`, and the unchanged dynamic table.

`IndexedDecodeFailure` is one of `MalformedIndexedInteger`,
`IncompleteIndexedInteger`, `ZeroIndexedHeader`,
`UnavailableStaticEntry(index)`, or
`UnavailableDynamicEntry(index, dynamic_index, entry_count)`. The failure
kind, requested index, dynamic index, and dynamic entry count have public
projections where applicable. A failed decode returns neither a header nor a
next table and leaves the caller-owned input table unchanged.

`http2::hpack::decode_literal_header_field(input, table)` decodes one literal
header field with incremental indexing, without indexing, or marked never
indexed. A zero name index reads a raw or Huffman string name; a nonzero name
index resolves through the static or immutable dynamic table with the
representation's full prefixed integer. The value is another raw or Huffman
string and is returned as an exact `ByteChunk`, including non-visible octets.
The transition identifies the representation and reports its decoded field,
consumed octet count, and next table. Incremental indexing inserts the field
into the next table; the other representations return the unchanged table.

`LiteralDecodeFailure` distinguishes malformed or incomplete name indices,
unavailable static or dynamic names, malformed or incomplete name and value
lengths, truncated raw names and values, name and value Huffman failures, and
invalid name octets. Its truncation constructors are
`TruncatedLiteralRawName(expected, available)` and
`TruncatedLiteralRawValue(expected, available)`; unavailable dynamic names
retain `(index, dynamic_index, entry_count)`. These coordinates and the
expected/available truncation counts are projected by the public facade.
Every failure returns neither a decoded field nor a next table and preserves
the input table.

`http2::hpack::decode_header_block(input, table, peer_maximum)` recursively
decodes a complete ordered block of indexed and literal fields. `HeaderList`
keeps wire order, and every `HeaderField` retains its value as an exact
`ByteChunk`. The transition reports the full list, total consumed octets, and
next immutable table, so an incrementally indexed field is available to later
fields in the same block and to a later decode.

One or more bounded table-size updates may lead the block. A table-size update
after the first field is `MisplacedTableSizeUpdate`. Unsupported first-octet
representations produce `UnsupportedHeaderRepresentation(first_octet)`;
indexed, literal, and table-size-update failures are retained as
`HeaderBlockIndexedFailure`, `HeaderBlockLiteralFailure`, and
`HeaderBlockTableSizeUpdateFailure`. A failed block exposes neither a partial
header list nor a next table and leaves the input table unchanged.

## Frame and output boundaries

The frame module decodes complete frame headers and encodes validated headers.
Header fields expose length, kind, flags, and stream id; reserved-bit,
stream-id, maximum-size, and truncated-header failures retain offset and
preview context through the diagnostic module. Protocol and peer-limit
constructors project stable diagnostic ids and the `details.protocol_diagnostic`
shape used by command output.

`http2::core::empty_output_buffer()` creates immutable output state. Append
helpers accept successful send decisions for SETTINGS acknowledgements, PING,
GOAWAY, DATA, WINDOW_UPDATE, HEADERS, PUSH_PROMISE, RST_STREAM, and PRIORITY.
Each accepted decision appends exactly one byte chunk in order. Rejected,
encode-failed, no-pending, and no-response decisions preserve the input buffer.
Projections expose chunk count, indexed chunks, and concatenated bytes.

## Verification references

The public implementations are under `crates/veln-stdlib/veln/http2/`; adjacent module tests exercise source-visible return values and failure projections. Runtime adapter and protocol diagnostic projections are covered by the JVM backend and CLI protocol suites. Representative frame, connection, and HPACK inputs live under `examples/specification/`.
