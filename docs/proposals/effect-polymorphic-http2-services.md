# Effect-Polymorphic HTTP/2 Services

Status: proposed

## Summary

Add high-level HTTP/2 server and client service boundaries after the
single-connection duplex-stream driver is implemented. The service APIs own
listener or connection tasks while preserving the effects of application
callbacks instead of fixing those callbacks to one concrete effect set.
Standard task creation already preserves job effects; this proposal now covers
only the remaining HTTP/2 service surfaces and ownership behavior.

## Activation Gate

This proposal depends on:

- [Lexical Operation Handlers](../reference/implemented-proposals/lexical-operation-handlers.md), and
- [HTTP/2 Duplex Stream Connection Driver](../reference/implemented-proposals/http2-duplex-stream-connection-driver.md), and
- [HTTP/2 Application Event And Action Boundary](../reference/implemented-proposals/http2-application-event-action-boundary.md).

The application-boundary activation gate is met by
`Http2ApplicationEvent`, `Http2ApplicationAction`, and
`drive_server_application`, with typed failures represented by
`Http2ApplicationBoundaryFailure`. This service proposal must use those
implemented values or explicitly adapt them before adding effect rows. The
values support one request and one response without exposing `NetStream` or
mutable core state.

This proposal selects the current abrupt runtime boundary for transport
failure. Source cleanup after that failure is not required. A separate
value-returning transport or scoped-cleanup proposal is required before a
service can guarantee that service-owned cleanup continues in source.

[One-Shot Resumable Effect Handlers](one-shot-resumable-effect-handlers.md) is
not a dependency unless a checked service case meets its separate activation
gate.

## Effect-Preserving Boundaries

The effect-row language foundation is implemented and current. The
source grammar, effect diagnostics, assignment compatibility, checked-core and
typed-IR preservation, and executable evidence are specified by
`../specification/source-surface.md`, `../specification/names-effects.md`,
`../specification/types.md`,
`../../examples/specification/check/effect-row-syntax-diagnostics/`, and
`../../examples/specification/check/http2-service-effect-row/`.

The current specification also includes effect-preserving task creation:
`task::spawn<T, effect E>(job: fn() -> T effects [...E])` and
`task::spawn_with<T, C, effect E>(job: fn(C) -> T effects [...E], context: C)`
infer `concurrency` plus the concrete job effects. The checked
`../../examples/specification/check/http2-service-task-effect-row/` case fixes
the current task boundary for pure jobs, effectful jobs, and a context job
whose lexical transport handler replaces `transport::DuplexStream` with `net`
while preserving an application `db` effect.

The remaining service work uses the implemented effect-row and task
boundaries to preserve callback effects through proposed HTTP/2 service
surfaces:

```veln
fn(Request) -> Result<Response, String> effects [...E]
```

```veln
pub fn serve_connection<effect E>(handler: fn(Request) -> Result<Response, String> effects [...E]) -> Result<(), Http2ServiceFailure> effects [transport::DuplexStream, ...E]
	# Standard-library body omitted.
end
```

`serve_connection` is the task-entry connection logic. It does not create or
join a task. A separate TCP service surface owns listener and task operations:

```veln
pub fn serve_tcp<effect E>(listener: NetListener, handler: fn(Request) -> Result<Response, String> effects [...E]) -> Result<(), Http2ServiceFailure> effects [net, concurrency, ...E]
	# Standard-library body omitted.
end
```

At these service boundaries, a caller must not erase an application effect by
passing the callback through the service API. Handling
`transport::DuplexStream` removes only that nominal effect and must not remove
effects supplied by the callback row.

When a proposed service creates a connection task, the connection job installs
the TCP handler inside the task. The job substitutes `net` together with the
application callback effects into the current task boundary. The spawn
expression therefore exposes `concurrency`, `net`, and those callback effects.
Installed lexical handlers still do not cross the task boundary.

The following table is authoritative for the service effect boundaries:

| Boundary | Callback or job effects | Required expression effects |
| --- | --- | --- |
| Abstract connection with pure callback | `[]` | `[transport::DuplexStream]` |
| Abstract connection with database callback | `[db]` | `[transport::DuplexStream, db]` |
| TCP handler around database connection job | `[transport::DuplexStream, db]` | `[net, db]` |
| TCP service with pure callback | `[]` | `[net, concurrency]` |

## Service Ownership

The server surface owns a listener loop. Each accepted stream has one
connection task and one lexical duplex-stream handler installed inside that
task. The client surface owns connected-stream tasks and reuses a connection
only while the existing HTTP/2 core state permits it.

The first service slice is bounded to:

- one listener;
- a finite configured connection count in executable cases;
- one application callback invocation for each completed request event;
- one response action sequence for each successful callback result;
- explicit join and close of every owned task and stream on success and on
  ordinary callback, join, or protocol failure;
- fail-fast cleanup after the first ordinary callback, join, or protocol
  failure;
- no source-cleanup guarantee after an abrupt runtime transport failure.

The service must not propagate an installed handler implicitly through
`task::spawn_with`. The task entry installs its own handler from the explicit
`NetStream` context.

## Service State Transitions

| State | Event | Next state | Required action |
| --- | --- | --- | --- |
| Listening | Accepted stream | Listening with pending task | Spawn one task with explicit stream context |
| Listening | Clean listener end | Draining | Stop accepting and retain pending tasks |
| Pending task | Complete request | Pending task | Invoke the application callback once |
| Pending task | Callback success | Pending task | Convert the response into ordered protocol actions |
| Pending task | Callback failure | Failing | Emit no later response bytes for that stream |
| Draining | Task success | Draining or complete | Join, write accepted output, and close its stream once |
| Draining | Non-transport join failure or ordinary protocol failure | Failing | Cancel and join later tasks; suppress their writes; close retained streams once |
| Failing | Cleanup complete | Failed | Return the first service failure with cleanup context |
| Complete | No pending work | Complete | Close the listener once and return success |
| Any active state | Abrupt runtime transport failure | Runtime termination | Preserve the transport failure and prior committed writes; require no later source write, cancel, join, stream close, or listener close |

This table extends the current adapter-owned fail-fast task and stream cleanup
shape. It does not redefine HTTP/2 stream state, frame ordering, or flow
control.

## Remaining Acceptance Model

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| TCP handler replacement | Handling `DuplexStream` replaces only that effect with `net` | `check/http2-service-transport-effect-replacement` |
| Two connections | Independent tasks invoke the same callback and preserve per-connection output order | `run/http2-service-two-connections` |
| Callback failure | The failed stream emits no later response bytes and every owned stream closes once | `run/http2-service-callback-failure` |
| Non-transport join failure | Later tasks are cancelled and joined; retained streams close once | `run/http2-service-join-failure-json` |
| Protocol failure | The service preserves protocol failure details and performs bounded cleanup | `run/http2-service-protocol-failure-json` |
| Transport failure | Runtime transport classification and prior committed writes remain visible; the case requires no later source cleanup observation | `run/http2-service-transport-failure-json` |
| No handler inheritance | The spawned connection job installs its own handler from explicit context | `check/http2-service-task-handler-boundary` |
| Client reuse boundary | Reuse occurs only while public core projections report an eligible open connection | `run/http2-client-service-reuse-boundary` |

The relative paths are planned directories below `examples/specification/`.
The remaining effect-row work is limited to the proposed HTTP/2 service APIs.

## Non-Goals

- Do not make arbitrary effect rows implicit at public boundaries.
- Do not add higher-ranked effects, effect subtyping, or effect access modes.
- Do not change task scheduling, cancellation, joining, or handler inheritance
  semantics beyond preserving job effects in task-creation types.
- Do not expose socket handles or handler contexts to application callbacks.
- Do not make HTTP/2 protocol state mutable.
- Do not add TLS, ALPN, proxy, origin-coalescing, or URI discovery behavior.
- Do not imply that source cleanup continues after an abrupt runtime transport
  failure unless a separate implemented boundary guarantees it.
- Do not promise unbounded production load or performance thresholds in
  functional executable cases.
- Do not require explicit resumptions unless their separate activation gate is
  met.

## Completion Boundary

This proposal is complete when one server and one client service boundary
preserve callback effects, the service state rows have executable evidence,
and the implemented behavior is promoted to the type, effect, execution,
HTTP/2, and example specification routes.
