# Effect-Polymorphic HTTP/2 Services

Status: implemented

## Summary

Records the high-level HTTP/2 client service boundary after the
single-connection duplex-stream driver and server service boundary are
implemented. The client API owns connection tasks and reuses a connection only
while public core projections report that reuse is valid. Callback effects
must remain visible instead of being fixed to one concrete effect set.

## Activation Gate

This proposal depends on:

- [Lexical Operation Handlers](lexical-operation-handlers.md), and
- [HTTP/2 Duplex Stream Connection Driver](http2-duplex-stream-connection-driver.md), and
- [HTTP/2 Application Event And Action Boundary](http2-application-event-action-boundary.md).

The application-boundary activation gate is met by
`Http2ApplicationEvent`, `Http2ApplicationAction`, and
`drive_server_application`, with typed failures represented by
`Http2ApplicationBoundaryFailure`. The service uses those implemented values
and adapts the client response direction before adding effect rows. The
values support one request and one response without exposing `NetStream` or
mutable core state.

The server service boundary is current behavior in
`../../specification/http2.md`, `../../specification/execution.md`, and
`../../specification/names-effects.md`. The checked evidence is under
`../../../examples/specification/check/http2-service-transport-effect-replacement/`,
`../../../examples/specification/check/http2-service-task-effect-row/`,
`../../../examples/specification/check/http2-service-task-handler-boundary/`,
`../../../examples/specification/run/http2-service-two-connections/`,
`../../../examples/specification/run/http2-service-callback-failure/`,
`../../../examples/specification/run/http2-service-join-failure-json/`,
`../../../examples/specification/run/http2-service-protocol-failure-json/`, and
`../../../examples/specification/run/http2-service-transport-failure-json/`.

This proposal selects the current abrupt runtime boundary for transport
failure. Source cleanup after that failure is not required. A separate
value-returning transport or scoped-cleanup proposal is required before a
service can guarantee that service-owned cleanup continues in source.

[One-Shot Resumable Effect Handlers](../source-decisions/records/result-one-shot-resumable-handler-boundary.md)
was rejected because no checked service case requires handler-controlled
suspension or continuation disposition.

## Effect-Preserving Boundaries

The effect-row language foundation is implemented and current. The
source grammar, effect diagnostics, assignment compatibility, checked-core and
typed-IR preservation, and executable evidence are specified by
`../../specification/source-surface.md`, `../../specification/names-effects.md`,
`../../specification/types.md`,
`../../../examples/specification/check/effect-row-syntax-diagnostics/`, and
`../../../examples/specification/check/http2-service-effect-row/`.

The current specification also includes effect-preserving task creation:
`task::spawn<T, effect E>(job: fn() -> T effects [...E])` and
`task::spawn_with<T, C, effect E>(job: fn(C) -> T effects [...E], context: C)`
infer `concurrency` plus the concrete job effects. The checked
`../../../examples/specification/check/http2-service-task-effect-row/` case fixes
the current task boundary for pure jobs, effectful jobs, and a context job
whose lexical transport handler replaces `transport::DuplexStream` with `net`
while preserving an application `db` effect.

The current HTTP/2 application driver also preserves callback effects through
the real single-connection boundary. The checked
`../../../examples/specification/check/http2-service-transport-effect-replacement/`
case fixes `drive_server_application<effect E>` with pure callbacks,
effectful callbacks, TCP duplex-stream replacement, and missing public callback
effects.

The implemented service uses the effect-row and task boundaries to preserve
callback effects through the HTTP/2 client reuse surface:

```veln
fn(Request) -> Result<Response, String> effects [...E]
```

At the client service boundary, a caller must not erase an application effect
by passing the callback through the service API. Handling
`transport::DuplexStream` removes only that nominal effect and must not remove
effects supplied by the callback row.

The following table is authoritative for the service effect boundaries:

| Boundary | Callback or job effects | Required expression effects |
| --- | --- | --- |
| Client reuse service with database callback | `[db]` | `[net, concurrency, db]` |

## Service Ownership

The implemented server ownership boundary is specified in
`../../specification/http2.md` and checked under
`../../../examples/specification/run/http2-service-two-connections/` and the
neighboring HTTP/2 server service cases.

The client surface owns connected-stream tasks and reuses a
connection only while the existing HTTP/2 core state permits it. It must not
propagate an installed handler implicitly through `task::spawn_with`.

## Service State Transitions

| State | Event | Next state | Required action |
| --- | --- | --- | --- |
| Client idle | Request with no reusable connection | Pending task | Connect and spawn one connection task |
| Client idle | Request with reusable connection | Pending task | Reuse the retained connection task |
| Pending task | Response success | Client idle or complete | Preserve ordered response bytes and updated public core projections |
| Pending task | Ordinary callback, join, or protocol failure | Failing | Cancel and join later tasks; close retained streams once |
| Failing | Cleanup complete | Failed | Return the first service failure with cleanup context |
| Any active state | Abrupt runtime transport failure | Runtime termination | Preserve the transport failure and prior committed writes; require no later source write, cancel, join, stream close, or listener close |

This table extends the current adapter-owned fail-fast task and stream cleanup
shape. It does not redefine HTTP/2 stream state, frame ordering, or flow
control.

## Acceptance Evidence

| Case | Required observation | Evidence |
| --- | --- | --- |
| Client reuse boundary | Reuse occurs only while public core projections report an eligible open connection | `run/http2-client-service-reuse-boundary` |

The relative path is below `examples/specification/`. The effect-row evidence
is `../../../examples/specification/check/http2-client-service-effect-row/`.

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
- Do not require explicit resumptions for this service boundary.

## Completion Boundary

This proposal is complete. The client service boundary preserves callback
effects, the client reuse rows have executable evidence, and the implemented
behavior is promoted to the effect, execution, HTTP/2, and example
specification routes.
