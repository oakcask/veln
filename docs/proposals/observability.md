---
role: proposal
update-when: Structured logging, events, metrics, tracing, observation context propagation, exporter behavior, or the observability proposal dependencies change.
---

# Standard-library Observability

Veln programs need structured runtime evidence that tools and AI agents can
correlate with source, tasks, requests, and external services. The language
core must not prescribe an exporter or an external telemetry protocol.

The public observation effect and no-op handler, typed scalar attributes,
optional trace context, opaque span handle, and call-site-aware structured log
records at debug, info, warning, and error severities are implemented in the
[current observability specification](../specification/standard-library-observability.md).
This proposal covers only the remaining facilities described below.

## Dependencies

This proposal depends on:

- implemented [lexical deferred cleanup](../specification/execution.md#runtime-readiness-and-host-boundaries), so every started
  span can finish across normal and abrupt exits;
- the implemented [wall-clock boundary](../specification/effects.md#network-and-time-boundary-calls),
  so exported records can be correlated across processes; and
- implemented [direct, indirect, and runtime-callback call-site
  propagation](../specification/call-site-declarations.md), so library wrappers
  preserve the user's call expression and dependency locations remain stable
  across package relocation. The same implemented contract gives generated
  sources canonical virtual names and keeps retained locations meaningful
  during later observation.

The existing monotonic clock remains the source for elapsed duration.

## Boundary

Observability is a standard-library facility built from ordinary Veln types,
an ordinary public effect, and substitutable handlers. The proposal does not
add log, event, metric, span, or trace syntax to the language core.

The runtime may provide narrow host operations for identifier generation,
buffering, export, and shutdown flushing. Those operations are implementation
support for the library and do not define a global ambient observability API.

## Observation Effect Remainder

Extend the current `Observation` model with domain events, counter increments,
and histogram samples. Add span lifecycle handlers that create valid opaque
handles, and add trace-context operations that inject and extract context at
process boundaries. Export handlers add wall time, resource identity, sequence
information, and exporter-specific encoding.

## Library Surface

The remaining standard-library work supplies these groups:

- `event`: named domain events;
- `metric`: counter increments and histogram samples;
- `trace`: span start, finish, and scoped `in_span` helpers;
- `observe`: recording, JSON Lines, and configured export handlers; and
- `traced`: explicit context attachment for task and channel values.

Future event helpers follow the current call-site-aware logging contract.

`trace::in_span` starts a span, registers its finish operation with `defer`,
and invokes its body with the child context. The helper does not require a
special span construct in the language.

## Context Propagation

Trace context is explicit application data. Raw `task` and `channel`
operations do not acquire observability semantics.

- A direct child task can receive context through a `traced::spawn` library
  helper.
- A long-lived worker receives the producer's context in a `Traced<A>` value.
- Fan-in code chooses which context is the parent and can represent other
  causal inputs as links.
- Network adapters inject and extract a trace context through an explicit
  carrier.

This rule prevents a worker's creation context from being mistaken for the
context of every message it later receives. It also keeps custom observability
libraries possible.

## Handler and Failure Contract

Applications install an observation handler at their outer execution boundary.
Tests can use a recording handler that replaces clocks, identifiers, and
resource attributes with deterministic values.

Normal telemetry is best effort. `emit` and `finish_span` return `()`. An
export failure does not change the application result and does not recursively
perform `Observe`. The handler records bounded internal drop and failure
counters and can report a final exporter health summary on a separately
configured diagnostic channel.

Audit records whose persistence determines application success are outside
this effect. A future audit facility must return an ordinary typed result.

## Structured Data Contract

Production handlers apply configured size, count, redaction, and
metric-cardinality limits before export. A record then reports whether
attributes were dropped or truncated.

The JSON Lines handler emits one versioned object per line to a configured
destination that is separate from application stdout by default. Its checked
schema must include:

- observation kind and stable name;
- wall-clock seconds and nanoseconds;
- per-handler sequence;
- severity for log records;
- trace, span, parent-span, and link identities when present;
- canonical source location;
- task identity when available;
- typed attributes;
- resource and instrumentation-scope identity; and
- drop, truncation, redaction, and exporter-health facts when applicable.

The first implementation establishes the schema version. The proposal does not
invent a version before a checked schema exists.

## Acceptance Model

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| O2 | `in_span` completes normally or propagates `Err`. | Exactly one finish operation follows its matching start operation. | Ordered recording-handler cases. |
| O3 | A span body raises a contract or runtime failure. | Deferred cleanup finishes the span and the original failure remains primary. | Human and JSON runtime-failure cases. |
| O4 | A parent starts two child spans. | Both children identify the parent and have distinct span identities. | Deterministic recording-handler case. |
| O5 | `traced::spawn` starts a child task. | Records from that task use the explicitly supplied parent context. | Task runtime case. |
| O6 | A value crosses a raw channel without `Traced<A>`. | No trace context is implicitly added. | Type-check and recording-handler case. |
| O7 | A long-lived worker receives two traced values. | Each operation uses the context attached to its own value. | Channel worker case. |
| O8 | Several producers feed one consumer. | The selected parent and additional links match the consumer's explicit choice. | Fan-in case. |
| O9 | An exporter rejects or drops a record. | The application result is unchanged and bounded health facts reflect the failure. | Forced-export-failure case. |
| O10 | Counter and histogram records use the same name and attributes. | The handler preserves metric kind and values and applies configured cardinality limits. | Table-driven recording-handler cases. |
| O11 | A trace crosses a carrier boundary. | Injection followed by extraction preserves valid trace identity and sampling state; malformed input produces no forged valid context. | Carrier conformance cases. |
| O12 | JSON Lines export is enabled. | Each line validates against the checked schema and contains no machine-specific absolute source path. | Schema and relocation cases. |
| O14 | Handler shutdown follows buffered emission. | Accepted records are flushed or counted as dropped before shutdown returns. | Deterministic exporter lifecycle case. |

## Verification and Promotion

The next implementation work starts with a public recording handler and span
lifecycle. Their deterministic cases extend the current emission evidence
before a production exporter is added. The JSON Lines schema and fixtures then
establish the machine-readable contract. An external protocol adapter is
verified against that protocol's conformance fixtures and remains replaceable.

Executable cases belong under `examples/specification/`. When implementation
is complete, focused current specification pages must explain usage, handler
installation, propagation, failure behavior, limits, and the checked JSON
schema. This proposal and its catalog entry must then be removed.

## Non-goals

- Raw task spawn and channel send do not implicitly propagate context.
- The compiler does not automatically trace every call or effect operation.
- The core language does not depend on OpenTelemetry or another telemetry
  protocol.
- Observability does not provide durable audit guarantees.
- The initial metrics surface does not include asynchronous callbacks or a
  query language.
