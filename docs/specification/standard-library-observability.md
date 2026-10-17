---
role: specification
authority: normative
update-when: The standard-library Observe effect, observation handlers, scoped span helpers, explicit task-context propagation, structured logging, or observation data model changes.
specification-coverage: usage=#usage; behavior=#behavior; limits=#limits
---

# Standard-library Observability

The standard library provides a substitutable observation effect, scoped span
lifecycle, explicit child-task context propagation, an inspectable recording
handler, and call-site-aware structured log helpers for four severities.
Programs choose a lexical handler for the effect; the library does not select
an exporter implicitly.

## Usage

Import `observe`, `log`, and `traced` from the standard package. Supply optional trace
context, a message, and an ordered list of typed attributes to `log::debug`,
`log::info`, `log::warning`, or `log::error`. Run the call inside a lexical
`Observe` handler:

```veln
use log from "std"
use observe from "std"

fn report() -> () effects [observe::Observe] callsite
	log::info(None, "ready", list_cons(observe::Attribute("attempt", observe::IntValue(1)), list_nil()))
end
```

`Observe` has three operations: `start_span(SpanRequest) -> Span`,
`emit(Observation) -> ()`, and `finish_span(SpanFinish) -> ()`. `Span` is
opaque. Construct a request with a stable name and an optional opaque parent
span from a recording handler when recorded parentage is required.
`trace::in_span<effect E>` accepts a body of type
`fn(Span) -> Result<(), String> effects [...E]`. It starts the span, passes its
opaque handle to the body, forwards the callback effect row beside `Observe`,
and finishes the span with lexical deferred cleanup:

```veln
use observe from "std"
use trace from "std"

fn work(span: Span) -> Result<(), String> effects [stdio]
	stdio::println("working")
	Ok(())
end

fn traced_work() -> Result<(), String> effects [observe::Observe, stdio]
	trace::in_span(observe::SpanRequest("work", None), work)
end
```

Create a recording with `observe::new_recording(prefix)`, install
`observe::recording(recording)`, and call `observe::recorded(recording)` to
inspect its ordered entries. A read is a non-consuming sequential snapshot.
Repeated reads preserve the same prior entries. Later recording operations
append after that preserved prefix and continue identity allocation from the
same recording state. The prefix scopes deterministic trace and span identities
to that recording. The handler uses the monotonic clock, so a scoped
`host_effects::Clock` handler can supply deterministic readings in tests while
the unhandled production boundary reads the real host clock.

Install the public no-op handler when the program must discharge the effect
without retaining or exporting observations:

```veln
pub fn main() -> ()
	let _ = handle report() with observe::noop()
end
```

The recording and no-op handlers preserve the handled program's ordinary
result. They do not select behavior through process environment or system
properties.

Use `traced::spawn(context, body)` to start a child task whose callback needs
an explicit `observe::TraceContext`. The callback accepts that context as its
only argument and returns `Result<(), String>`. Its effect row is forwarded:

```veln
use observe from "std"
use traced from "std"

fn child(context: TraceContext) -> Result<(), String> effects [observe::Observe]
	# Supply context explicitly to observation helpers used here.
	Ok(())
end

fn start(context: TraceContext) -> Task<Result<(), String>> effects [observe::Observe, concurrency]
	traced::spawn(context, child)
end
```

The helper returns the ordinary task handle. `task::join` therefore returns
`Result<Result<(), String>, JoinError>`: the inner result is the callback
result, while the outer result retains ordinary task join and cancellation
behavior.

## Behavior

### Structured log records

Each log helper performs exactly one `emit` operation. The emitted
`Observation::Log` contains the stable name `log`, the helper's severity, the
supplied message, the supplied attributes in order, the supplied optional
`TraceContext`, and one `SourceLocation`.

| Helper | Emitted severity |
| --- | --- |
| `log::debug` | `Severity::Debug` |
| `log::info` | `Severity::Info` |
| `log::warning` | `Severity::Warning` |
| `log::error` | `Severity::Error` |

An attribute value is exactly one of `BoolValue(Bool)`, `IntValue(Int)`, or
`StringValue(String)`. A trace context contains its trace and span identity
strings. The standard library preserves these values; policy such as
redaction, truncation, or export encoding belongs to a future handler.

Each log helper is call-site-aware. An ordinary caller records the complete
span of its call expression. A call-site-aware wrapper forwards its own hidden
source location, so the observation identifies the outer user call rather than
the helper implementation. The path follows the canonical relative-source
rules of [Call-site Declarations](call-site-declarations.md).

### No-op handler

`observe::noop()` handles every `Observe` operation and has no retained host
effect. It discards each emitted observation and each span finish. A span start
returns an opaque span value, so the operation is total even though programs
cannot construct or inspect that value through the public API. Installing the
handler preserves the handled program's result and opens no output destination.

### Span lifecycle and recording

A root span receives a trace identity and a span identity from its recording.
A child request retains the parent's trace identity, records the parent's span
identity, and receives a distinct span identity. Each recording allocates
identities in order from its caller-supplied prefix and retains state in its own
channel-backed scope. A recording handler accepts an opaque parent returned by
another recording. If both recordings use the same prefix, child allocation
skips a candidate that equals the supplied parent identity. Children accepted
by one recording therefore remain distinct from their parent and siblings.
This local guarantee does not make identities globally unique across unrelated
recordings or processes. A no-op span has no recording identity; if one is
supplied as a parent to a recording handler, that request starts a root span
and records no parent identity.

`trace::in_span` registers the matching finish immediately after start. The
finish runs exactly once after a normal `Ok` return, after an `Err` result that
the caller propagates, and while a contract or runtime failure unwinds the
body. An existing abrupt failure remains primary. The finish entry uses the
same trace and span identities as its start entry.

`observe::recorded` returns a sequential snapshot in operation order without
consuming entries or resetting the next identity. A later snapshot includes
the preserved prefix followed by entries from subsequent recording operations.
`SpanStarted` exposes the name, trace identity, span identity, optional parent
span identity, and monotonic millisecond reading. `SpanFinished` exposes the
matching identities and finish reading. `Emitted` retains an emitted
observation alongside span entries, so one recording preserves the order
across all three `Observe` operations.

### Explicit child-task context

`traced::spawn` creates an ordinary child task through the task API and passes
the supplied `TraceContext` unchanged to its callback. The callback's effects
remain part of the helper's effect row. A scoped observation handler active at
task creation is inherited under the ordinary task-handler rule, so a child
record can retain the context when the callback explicitly supplies it to a
log or observation helper.

The helper does not create a span, allocate identifiers, install a handler, or
infer ambient context. Raw `task::spawn`, `task::spawn_with`, and channel
operations have no observation-context behavior. Context propagation across a
task boundary occurs only through the `TraceContext` argument of
`traced::spawn`.

## Limits

Events, metrics, channel propagation helpers, carrier propagation,
production exporters, wall-time or resource enrichment, exporter health, and
shutdown flushing are not available. The no-op handler does not record,
enrich, buffer, export, or report observations. `trace::in_span` currently
accepts only a body returning `Result<(), String>`, while forwarding that
body's effect row; it does not define rescue or exception recovery.
`traced::spawn` has the same callback-result restriction. It does not wrap
channel values or propagate context through raw task or channel operations.

An unhandled `Observe` effect follows the ordinary nominal-effect runnable
boundary in [Effects](effects.md): a runnable entry must handle it before
execution. Span handles and finish values remain opaque. Deferred finish has
the lexical cleanup limits in [Execution Boundary](execution.md#cleanup-limits):
process or machine termination can prevent it from running.

## References

- Standard-library API coverage:
  [`observe.test.veln`](../../crates/veln-stdlib/veln/observe.test.veln) and
  [`log.test.veln`](../../crates/veln-stdlib/veln/log.test.veln), plus scoped
  span lifecycle coverage in
  [`trace.test.veln`](../../crates/veln-stdlib/veln/trace.test.veln) and explicit
  task-context coverage in
  [`traced.test.veln`](../../crates/veln-stdlib/veln/traced.test.veln).
- Public task-context execution evidence:
  [`observability-traced-task-context`](../../examples/specification/run/observability-traced-task-context/).
- Call-site and effect-dispatch execution evidence:
  [`observability-log-severities`](../../examples/specification/run/observability-log-severities/).
- Public no-op handler execution evidence:
  [`observability-noop-handler`](../../examples/specification/run/observability-noop-handler/).
- Runtime-failure cleanup and primary-failure evidence:
  [`observability-span-runtime-failure-human`](../../examples/specification/run/observability-span-runtime-failure-human/)
  and
  [`observability-span-runtime-failure-json`](../../examples/specification/run/observability-span-runtime-failure-json/).
