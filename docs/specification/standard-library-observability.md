---
role: specification
authority: normative
update-when: The standard-library Observe effect, structured logging, observation data model, or recording-handler evidence changes.
specification-coverage: usage=#usage; behavior=#structured-log-records; limits=#limits
---

# Standard-library Observability

The standard library provides a substitutable observation effect and
call-site-aware structured log helpers for four severities. Programs choose a
lexical handler for the effect; the library does not select an exporter
implicitly.

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
opaque. This slice exposes the complete effect shape so handlers can remain
compatible as later observation kinds are added, but only log emission has a
public construction path.

## Structured log records

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

## Limits

Events, metrics, span lifecycle helpers, propagation helpers, no-op and
production exporters, wall-time or resource enrichment, exporter health, and
shutdown flushing are not available. The standard package does not export a
recording handler; repository tests install separate Veln handlers with
isolated channel state.

An unhandled `Observe` effect follows the ordinary nominal-effect runnable
boundary in [Effects](effects.md): a runnable entry must handle it before
execution. The constructorless span request, handle, and finish types do not
provide a public span lifecycle in this slice.

## References

- Standard-library API coverage:
  [`log.test.veln`](../../crates/veln-stdlib/veln/log.test.veln).
- Call-site and effect-dispatch execution evidence:
  [`observability-log-severities`](../../examples/specification/run/observability-log-severities/).
