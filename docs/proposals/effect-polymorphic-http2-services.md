# Effect-Polymorphic HTTP/2 Services

Status: proposed

## Summary

Add high-level HTTP/2 server and client service boundaries after the
single-connection duplex-stream driver is implemented. The service APIs own
listener or connection tasks while preserving the effects of application
callbacks instead of fixing those callbacks to one concrete effect set.

## Activation Gate

This proposal depends on:

- [Lexical Operation Handlers](lexical-operation-handlers.md), and
- [HTTP/2 Duplex Stream Connection Driver](http2-duplex-stream-connection-driver.md).

Implementation may start only after the connection-driver evidence identifies
the application event and action values that cross the pure protocol boundary.
The proposal must be revised if those values do not support one request and
one response without exposing `NetStream` or mutable core state.

Implementation must also identify the resource outcome for abrupt runtime
transport failure. A separate value-returning transport or scoped-cleanup
proposal is required if service-owned cleanup must continue in source after
such a failure.

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
- explicit join and close of every owned task and stream;
- fail-fast cleanup after the first callback, join, protocol, or transport
  failure.

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
| Draining | Task or ordinary protocol failure | Failing | Cancel and join later tasks; suppress their writes; close retained streams once |
| Failing | Cleanup complete | Failed | Return the first service failure with cleanup context |
| Complete | No pending work | Complete | Close the listener once and return success |

This table extends the current adapter-owned fail-fast task and stream cleanup
shape. It does not redefine HTTP/2 stream state, frame ordering, or flow
control.

## Acceptance Model

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| Effect-row identity | A pure callback adds no effect beyond the service effects | `check/http2-service-effect-row` |
| Effect-row preservation | A callback with `db` or `stdio` makes the instantiated service boundary expose that effect | `check/http2-service-effect-row` |
| TCP handler replacement | Handling `DuplexStream` replaces only that effect with `net` | `check/http2-service-transport-effect-replacement` |
| Two connections | Independent tasks invoke the same callback and preserve per-connection output order | `run/http2-service-two-connections` |
| Callback failure | The failed stream emits no later response bytes and every owned stream closes once | `run/http2-service-callback-failure` |
| Join failure | Later tasks are cancelled and joined; retained streams close once | `run/http2-service-join-failure-json` |
| Protocol failure | The service preserves protocol failure details and performs bounded cleanup | `run/http2-service-protocol-failure-json` |
| Transport failure | Runtime transport classification remains distinct from application and protocol failures; cleanup claims do not exceed the selected runtime boundary | `run/http2-service-transport-failure-json` |
| No handler inheritance | The spawned connection job installs its own handler from explicit context | `check/http2-service-task-handler-boundary` |
| Client reuse boundary | Reuse occurs only while public core projections report an eligible open connection | `run/http2-client-service-reuse-boundary` |

The relative paths are planned directories below `examples/specification/`.
The effect-row cases must also have focused semantic tests for parsing,
inference, substitution, assignment compatibility, diagnostics, checked-core,
and typed-IR preservation.

## Non-Goals

- Do not make arbitrary effect rows implicit at public boundaries.
- Do not add higher-ranked effects, effect subtyping, or effect access modes.
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
are checked, one server and one client service boundary preserve callback
effects, the service state rows have executable evidence, and the implemented
behavior is promoted to the type, effect, execution, HTTP/2, and example
specification routes.
