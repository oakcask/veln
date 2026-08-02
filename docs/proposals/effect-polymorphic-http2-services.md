# Effect-Polymorphic HTTP/2 Services

Status: proposed

## Summary

Add high-level HTTP/2 server and client service boundaries after the
single-connection duplex-stream driver is implemented. The service APIs own
listener or connection tasks while preserving the effects of application
callbacks instead of fixing those callbacks to one concrete effect set.

## Activation Gate

This proposal depends on:

- [Lexical Operation Handlers](../reference/implemented-proposals/lexical-operation-handlers.md), and
- [HTTP/2 Duplex Stream Connection Driver](../reference/implemented-proposals/http2-duplex-stream-connection-driver.md), and
- [HTTP/2 Application Event And Action Boundary](../reference/implemented-proposals/http2-application-event-action-boundary.md).

The application-boundary activation gate is met by
`Http2ApplicationEvent`, `Http2ApplicationAction`, and
`drive_server_application`. This service proposal must use those implemented
values or explicitly adapt them before adding effect rows. The values support
one request and one response without exposing `NetStream` or mutable core
state.

This proposal selects the current abrupt runtime boundary for transport
failure. Source cleanup after that failure is not required. A separate
value-returning transport or scoped-cleanup proposal is required before a
service can guarantee that service-owned cleanup continues in source.

[One-Shot Resumable Effect Handlers](one-shot-resumable-effect-handlers.md) is
not a dependency unless a checked service case meets its separate activation
gate.

## Effect Rows

Add one effect-row variable to function declarations and function types. The
proposed notation uses `...E` as the final entry of an effect set:

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

The `effect` binder distinguishes an effect-row variable from an ordinary type
parameter. The executable type grammar is authoritative once implemented. It
must reject an unbound row variable, more than one row tail, or a row tail
outside the final effect-set position.

Effect-row behavior is set based:

- Instantiation substitutes one duplicate-free effect set for `E`.
- Concrete effects written beside `...E` are unioned with the substitution.
- A caller cannot erase an application effect by passing the callback through
  the service API.
- Handling `transport::DuplexStream` removes only that nominal effect. It does
  not remove effects substituted for `E`.
- Public diagnostics render the concrete instantiated effects when they are
  known at the call boundary.

The standard task creation functions must preserve the effects of their job
functions. Their effect-row shapes are:

```veln
task::spawn<T, effect E>(job: fn() -> T effects [...E]) -> Task<T> effects [concurrency, ...E]
task::spawn_with<T, C, effect E>(job: fn(C) -> T effects [...E], context: C) -> Task<T> effects [concurrency, ...E]
```

These signatures replace the fixed job-effect boundary only after effect rows
are implemented. A pure job substitutes the empty set for `E`. A connection
job that installs the TCP handler substitutes `net` together with the
application callback effects. The spawn expression therefore exposes
`concurrency`, `net`, and those callback effects. Installed lexical handlers
still do not cross the task boundary.

The following table is authoritative for the service effect boundaries:

| Boundary | Callback or job effects | Required expression effects |
| --- | --- | --- |
| Abstract connection with pure callback | `[]` | `[transport::DuplexStream]` |
| Abstract connection with database callback | `[db]` | `[transport::DuplexStream, db]` |
| TCP handler around database connection job | `[transport::DuplexStream, db]` | `[net, db]` |
| Spawn of handled database connection job | `[net, db]` | `[net, concurrency, db]` |
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

## Acceptance Model

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| Effect-row identity | A pure callback adds no effect beyond the service effects | `check/http2-service-effect-row` |
| Effect-row preservation | A callback with `db` or `stdio` makes the instantiated service boundary expose that effect | `check/http2-service-effect-row` |
| Task effect preservation | Spawning a connection job exposes `concurrency`, `net`, and its callback effects without retaining `DuplexStream` | `check/http2-service-task-effect-row` |
| TCP handler replacement | Handling `DuplexStream` replaces only that effect with `net` | `check/http2-service-transport-effect-replacement` |
| Two connections | Independent tasks invoke the same callback and preserve per-connection output order | `run/http2-service-two-connections` |
| Callback failure | The failed stream emits no later response bytes and every owned stream closes once | `run/http2-service-callback-failure` |
| Non-transport join failure | Later tasks are cancelled and joined; retained streams close once | `run/http2-service-join-failure-json` |
| Protocol failure | The service preserves protocol failure details and performs bounded cleanup | `run/http2-service-protocol-failure-json` |
| Transport failure | Runtime transport classification and prior committed writes remain visible; the case requires no later source cleanup observation | `run/http2-service-transport-failure-json` |
| No handler inheritance | The spawned connection job installs its own handler from explicit context | `check/http2-service-task-handler-boundary` |
| Client reuse boundary | Reuse occurs only while public core projections report an eligible open connection | `run/http2-client-service-reuse-boundary` |

The relative paths are planned directories below `examples/specification/`.
The effect-row cases must also have focused semantic tests for parsing,
inference, substitution, assignment compatibility, diagnostics, checked-core,
typed-IR preservation, and the effect-preserving standard task signatures.

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

This proposal is complete when the effect-row grammar and substitution rules
are checked, task creation preserves job effects, one server and one client
service boundary preserve callback effects, the service state rows have
executable evidence, and the implemented behavior is promoted to the type,
effect, execution, HTTP/2, and example specification routes.
