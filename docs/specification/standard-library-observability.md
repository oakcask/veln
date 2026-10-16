---
role: specification
authority: normative
update-when: The standard-library Observe effect, observation handlers, scoped span helpers, structured logging, or observation data model changes.
specification-coverage: usage=#usage; behavior=#behavior; limits=#limits
---

# Standard-library Observability

The standard library provides a substitutable observation effect, scoped span
lifecycle, an inspectable recording handler, and call-site-aware structured
log helpers for four severities. Programs choose a lexical handler for the
effect; the library does not select an exporter implicitly.

## Usage

Import `observe` and `log` from the standard package. Supply optional trace
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
span. `trace::in_span` accepts a body that returns `Result<(), String>`, starts
the span, passes its opaque handle to the body, and finishes it with lexical
deferred cleanup:

```veln
use observe from "std"
use trace from "std"

fn work(span: Span) -> Result<(), String>
	Ok(())
end

fn traced_work() -> Result<(), String> effects [observe::Observe]
	trace::in_span(observe::SpanRequest("work", None), work)
end
```

Create a recording with `observe::new_recording(prefix)`, install
`observe::recording(recording)`, and call `observe::recorded(recording)` to
inspect its ordered entries. The prefix scopes deterministic trace and span
identities to that recording. The handler uses the monotonic clock, so a scoped
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
channel-backed scope.

`trace::in_span` registers the matching finish immediately after start. The
finish runs exactly once after a normal `Ok` return, after an `Err` result that
the caller propagates, and while a contract or runtime failure unwinds the
body. An existing abrupt failure remains primary. The finish entry uses the
same trace and span identities as its start entry.

`observe::recorded` returns entries in operation order. `SpanStarted` exposes
the name, trace identity, span identity, optional parent span identity, and
monotonic millisecond reading. `SpanFinished` exposes the matching identities
and finish reading. `Emitted` retains an emitted observation alongside span
entries, so one recording preserves the order across all three `Observe`
operations.

## Limits

Events, metrics, task or channel propagation helpers, carrier propagation,
production exporters, wall-time or resource enrichment, exporter health, and
shutdown flushing are not available. The no-op handler does not record,
enrich, buffer, export, or report observations. `trace::in_span` currently
accepts only a body returning `Result<(), String>`; it does not define rescue
or exception recovery.

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
  [`trace.test.veln`](../../crates/veln-stdlib/veln/trace.test.veln).
- Call-site and effect-dispatch execution evidence:
  [`observability-log-severities`](../../examples/specification/run/observability-log-severities/).
- Public no-op handler execution evidence:
  [`observability-noop-handler`](../../examples/specification/run/observability-noop-handler/).
- Runtime-failure cleanup and primary-failure evidence:
  [`observability-span-runtime-failure-human`](../../examples/specification/run/observability-span-runtime-failure-human/)
  and
  [`observability-span-runtime-failure-json`](../../examples/specification/run/observability-span-runtime-failure-json/).
