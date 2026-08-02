# HTTP/2 Application Event And Action Boundary

Status: implemented

## Summary

Add one server-side connection-driver boundary that drains immutable HTTP/2
request events from the protocol core, passes them to a pure application
callback, and applies the callback's immutable response actions through the
existing protocol core. The core request-header event and pure drain boundary
are implemented and specified in `../../specification/http2.md`; the remaining
proposal work is the driver, response-action values, and application failure
boundary.

## Dependencies

This proposal depends on:

- [HTTP/2 Duplex Stream Connection Driver](http2-duplex-stream-connection-driver.md), and
- [Lexical Operation Handlers](lexical-operation-handlers.md).

Current HTTP/2 behavior remains specified by
[`http2.md`](../../specification/http2.md). Current duplex-stream effect and
handler behavior remains specified by
[`names-effects.md`](../../specification/names-effects.md).

## Bounded Driver Slice

The remaining bounded slice accepts one server-side request whose completed HEADERS block
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

## Implemented Core Event Value

The core request event is current source-visible behavior:

```veln
pub type Http2ApplicationEvent
	Http2RequestHeaders(stream_id: Int, headers: HeaderList, end_stream: Bool)
end
```

The authoritative source declaration, specification, and executable evidence
are in the standard library, `../../specification/http2.md`, and
`../../../examples/specification/run/http2-core-application-event-drain/`.
The event must not contain `NetStream`, `CoreConnectionState`,
`CoreReceiveConnectionState`, or mutable host state. The core emits
`Http2RequestHeaders` only after HPACK decoding, request-header validation,
and the associated stream transition succeed.

## Implemented Application Values

The implemented response-action value is:

```veln
pub type Http2ApplicationAction
	Http2SendResponseHeaders(stream_id: Int, headers: HeaderList, end_stream: Bool)
	Http2SendResponseData(stream_id: Int, data: ByteChunk, end_stream: Bool)
end
```

The executable source grammar and checked standard-library declarations are
authoritative for final action names. The response-action value must not
contain `NetStream`, `CoreConnectionState`, `CoreReceiveConnectionState`, or
mutable host state.

The callback shape is:

```veln
fn(Http2ApplicationEvent) -> Result<List<Http2ApplicationAction>, String>
```

The callback is pure in this slice. Effect-polymorphic callbacks remain owned
by the separate service proposal.

## Implemented Pure Core Boundary

The pure core receive-state event drain is current behavior. Remaining
proposal sections depend on that implemented boundary instead of redefining
its current behavior here.

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
| Receiving before an application request | Clean accepted EOF | Complete | Return the accepted core state and invoke no callback |
| Driving after one response | A second request event | Failed | Return unsupported-request-count failure and do not invoke the callback again |
| Driving after one response | Clean accepted EOF | Complete | Return the accepted core state without another callback invocation |
| Receiving or driving | EOF with an incomplete frame or header block | Failed | Return the existing incomplete-input connection failure and preserve prior committed writes |
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

## Implemented Core Evidence

The current core event-drain boundary is already covered by focused
standard-library tests and
`../../../examples/specification/run/http2-core-application-event-drain/`. That
evidence records receive-order retention for multiple complete frames in one
chunk, exactly-once drain behavior, deferred emission for incomplete HEADERS
until final CONTINUATION completion, and the public event value boundary.

## Acceptance Evidence

| Case | Required observation | Evidence |
| --- | --- | --- |
| Pure boundary | The application driver exposes only `transport::DuplexStream`; its callback has no effects | `check/http2-connection-application-boundary-effects` |
| One request and response | One accepted headers-only request invokes the callback once and writes accepted response HEADERS and DATA bytes in order | `run/http2-connection-application-one-request` |
| Callback failure | Callback failure writes no response bytes and remains distinct from protocol failure | `run/http2-connection-application-callback-failure-json` |
| Unsupported request | Valid request HEADERS without `END_STREAM` produces the typed unsupported-request outcome without callback invocation | `run/http2-connection-application-unsupported-request-json` |
| Second request | A second request produces the typed unsupported-request-count outcome without a second callback invocation | `run/http2-connection-application-second-request-json` |
| Invalid action order | DATA before HEADERS is rejected before any response write | `run/http2-connection-application-invalid-actions-json` |
| Rejected later action | Earlier accepted bytes remain committed; the rejected and later actions write no bytes | `run/http2-connection-application-rejected-action-json` |
| Value isolation | The public application action type exposes no transport handle or core state | Focused standard-library API and semantic tests |

The relative paths in the table are checked directories below
`examples/specification/`. The run cases use the real `http2::connection`
and `http2::core` public surfaces. A fixture-local replacement core or
send-state model does not meet the acceptance model.

## Activation Handoff

The acceptance evidence has passed and the implemented behavior is promoted:

- application values and driver behavior are specified in the HTTP/2,
  effects, execution, JSON-output, and example routes;
- this proposal record lives under implemented proposal records; and
- [Effect-Polymorphic HTTP/2 Services](../../proposals/effect-polymorphic-http2-services.md)
  records that its application-boundary activation gate is met.

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
