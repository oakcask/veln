# HTTP/2 Application Event And Action Boundary

Status: proposed

## Summary

Add one server-side connection-driver boundary that passes immutable HTTP/2
request events to a pure application callback and applies the callback's
immutable response actions through the existing protocol core. This slice
identifies the values required by the effect-polymorphic service proposal
without adding effect rows, listener ownership, or task ownership.

## Dependencies

This proposal depends on:

- [HTTP/2 Duplex Stream Connection Driver](../reference/implemented-proposals/http2-duplex-stream-connection-driver.md), and
- [Lexical Operation Handlers](../reference/implemented-proposals/lexical-operation-handlers.md).

Current HTTP/2 behavior remains specified by
[`http2.md`](../specification/http2.md). Current duplex-stream effect and
handler behavior remains specified by
[`names-effects.md`](../specification/names-effects.md).

## Bounded First Slice

The first slice accepts one server-side request whose completed HEADERS block
also ends the request stream. The request has no DATA or trailer fields. The
callback may produce one response as an ordered HEADERS action followed by an
optional DATA action.

The driver rejects a request whose completed HEADERS do not end the stream at
the application boundary with a typed unsupported-request failure. The core
must first accept the input as valid HTTP/2. The failure is not a protocol
failure. The driver does not invoke the callback for that request.

This restriction is the stopping condition for the slice. Do not extend the
proposal by adding another fixed body size, chunk count, action count, or
request count. A later streaming boundary must use incremental application
events or an explicit bounded-body policy.

## Application Values

The proposed source-visible values are:

```veln
pub type Http2ApplicationEvent
	Http2RequestHeaders(stream_id: Int, headers: HeaderList, end_stream: Bool)
end

pub type Http2ApplicationAction
	Http2SendResponseHeaders(stream_id: Int, headers: HeaderList, end_stream: Bool)
	Http2SendResponseData(stream_id: Int, data: ByteChunk, end_stream: Bool)
end
```

The executable source grammar and checked standard-library declarations are
authoritative for final names. The values must not contain `NetStream`,
`CoreConnectionState`, `CoreReceiveConnectionState`, or mutable host state.
The core emits `Http2RequestHeaders` only after HPACK decoding, request-header
validation, and the associated stream transition succeed.

The callback shape is:

```veln
fn(Http2ApplicationEvent) -> Result<List<Http2ApplicationAction>, String>
```

The callback is pure in this slice. Effect-polymorphic callbacks remain owned
by the separate service proposal.

## Pure Core Boundary

An accepted connection receive transition must retain every application event
produced by complete frames in that transition. A pure drain operation returns
the retained events in receive order together with a next receive state that
does not retain those events. Draining the next state again returns no events.

This boundary prevents one transport read that contains multiple complete
frames from losing an earlier event when the receive loop processes a later
frame. Incomplete header blocks, response headers, trailers, and non-HEADERS
frames produce no request-header event in this slice.

The event drain must not expose or mutate the caller's core state. The final
standard-library types may extend the existing receive state or use a separate
pure transition value. The observable event order and exactly-once drain
behavior are authoritative; the internal storage shape is not.

## Driver Boundary

Add a server entry point with this source-visible shape:

```veln
pub fn drive_server_application(
	state: CoreConnectionState,
	handler: fn(Http2ApplicationEvent) -> Result<List<Http2ApplicationAction>, String>,
) -> Result<CoreConnectionState, Http2ApplicationConnectionFailure> effects [transport::DuplexStream]
	# Standard-library body omitted.
end
```

The final formatter-approved layout is authoritative. The new failure type
must distinguish the existing connection failures, callback failure,
unsupported request shape, invalid response action sequence, and rejected
core action.

The existing `drive_server` and `drive_client` signatures and behavior do not
change.

## State Transitions

| Current condition | Input or callback outcome | Next state | Required observation |
| --- | --- | --- | --- |
| Open server connection | Accepted incomplete request HEADERS block | Receiving | Retain no application event and invoke no callback |
| Receiving | Accepted complete request HEADERS with `END_STREAM` | Handling | Retain and drain one request-header event, then invoke the callback once |
| Receiving | Accepted complete request HEADERS without `END_STREAM` | Failed | Drain the request-header event, return unsupported-request failure, invoke no callback, and do not classify the input as a protocol failure |
| Driving after one response | A second request event | Failed | Return unsupported-request-count failure and do not invoke the callback again |
| Handling | Callback returns failure | Failed | Return callback failure and write no response action bytes |
| Handling | Callback returns valid response actions | Writing | Apply each action to the accepted immutable core state in list order |
| Writing | Core accepts an action | Writing or driving | Commit the returned state and write its bytes once in action order |
| Writing | Core rejects an action | Failed | Return the focused action failure; preserve earlier committed writes; write no bytes for the rejected or later actions |
| Any driver state | Duplex-stream transport fails | Runtime termination | Preserve the current abrupt transport-failure boundary and prior committed writes |

This table is authoritative for the new driver boundary. Existing core frame,
HPACK, stream, flow-control, and transport-failure rules remain authoritative
for their own decisions.

## Action Sequence Rules

The first response sequence must satisfy one of these rows:

| Actions | Required outcome |
| --- | --- |
| One HEADERS action with `end_stream = true` | Send one bodyless response |
| One HEADERS action with `end_stream = false`, then one DATA action with `end_stream = true` | Send one response with a body |
| Empty list | Reject before writing response bytes |
| DATA before HEADERS | Reject before writing response bytes |
| Any action after an action with `end_stream = true` | Reject before applying the invalid action; preserve earlier committed writes |
| An action whose stream id differs from the request stream id | Reject before applying that action |

The driver applies accepted actions through
`http2::core::send_response_headers(...)` and
`http2::core::send_data(...)`. It does not duplicate HPACK, stream lifecycle,
frame splitting, content-length, or flow-control rules.

## Acceptance Model

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| Pure boundary | The application driver exposes only `transport::DuplexStream`; its callback has no effects | `check/http2-connection-application-boundary-effects` |
| One request and response | One accepted headers-only request invokes the callback once and writes accepted response HEADERS and DATA bytes in order | `run/http2-connection-application-one-request` |
| Event drain | Multiple complete frames in one transport chunk preserve their request events in receive order; a second drain returns no events | Focused HTTP/2 core test and `run/http2-core-application-event-drain` |
| Callback failure | Callback failure writes no response bytes and remains distinct from protocol failure | `run/http2-connection-application-callback-failure-json` |
| Unsupported request | Valid request HEADERS without `END_STREAM` produces the typed unsupported-request outcome without callback invocation | `run/http2-connection-application-unsupported-request-json` |
| Second request | A second request produces the typed unsupported-request-count outcome without a second callback invocation | `run/http2-connection-application-second-request-json` |
| Invalid action order | DATA before HEADERS is rejected before any response write | `run/http2-connection-application-invalid-actions-json` |
| Rejected later action | Earlier accepted bytes remain committed; the rejected and later actions write no bytes | `run/http2-connection-application-rejected-action-json` |
| Value isolation | Public application event and action types expose no transport handle or core state | Focused standard-library API and semantic tests |

The relative paths are planned directories below `examples/specification/`.
The run cases must use the real `http2::connection` and `http2::core` public
surfaces. A fixture-local replacement core or send-state model does not meet
the acceptance model.

## Activation Handoff

After the acceptance evidence passes:

- promote the application values and driver behavior to the HTTP/2, effects,
  execution, and example specification routes;
- move this proposal to implemented proposal records;
- revise
  [Effect-Polymorphic HTTP/2 Services](effect-polymorphic-http2-services.md)
  to use or deliberately adapt the implemented request and action values; and
- record in that proposal that its application-boundary activation gate is
  met.

This proposal does not activate one-shot resumable handlers. The callback
returns an ordinary result and action list, and automatic continuation remains
sufficient.

## Non-Goals

- Do not add effect-row syntax or inference.
- Do not add listener, task, connection-pool, retry, or cleanup ownership.
- Do not change the client driver.
- Do not expose `NetStream` or mutable core state to the callback.
- Do not add request-body buffering without an explicit bounded-body policy.
- Do not add streaming request callbacks in this slice.
- Do not treat valid but unsupported request shapes as protocol failures.
- Do not add explicit resumptions.

## Completion Boundary

This proposal is complete when the real server connection driver passes one
headers-only request event to one pure callback, applies one valid response
action sequence through the existing immutable core, preserves the failure
rows above, and records the implemented event and action values in current
specification and executable evidence.
