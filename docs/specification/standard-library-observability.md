---
role: specification
authority: normative
update-when: The standard-library Observe effect, observation handlers, scoped span helpers, explicit task, channel-value, fan-in, or carrier context propagation, structured logging, named domain events, or observation data model changes.
specification-coverage: usage=#usage; behavior=#behavior; limits=#limits
---

# Standard-library Observability

The standard library provides a substitutable observation effect, scoped span
lifecycle, explicit child-task context propagation, an inspectable recording
handler, explicit channel-value context attachment, explicit fan-in parent and
link selection, explicit carrier context propagation, and call-site-aware
structured log helpers for four severities, named domain events, counter
increments, histogram samples, and a configured production metric-cardinality
policy with bounded health reporting. It also provides a synchronous JSON
Lines handler with configured resource identity, scope identity, attribute
limits, destinations, and exporter health.
Programs choose a lexical handler for the effect; the library does not select
an exporter implicitly.

## Usage

Import `observe`, `log`, `event`, `metric`, and `traced` from the standard package.
Supply optional trace context, a message, and an ordered list of typed
attributes to `log::debug`, `log::info`, `log::warning`, or `log::error`. Run
the call inside a lexical `Observe` handler:

```veln
use log from "std"
use observe from "std"

fn report() -> () effects [observe::Observe] callsite
	log::info(None, "ready", list_cons(observe::Attribute("attempt", observe::IntValue(1)), list_nil()))
end
```

Use `event::emit` for a named domain event. Supply optional trace context, a
stable event name, and an ordered list of typed attributes. The helper uses the
same `Observe` effect and lexical handlers as logging:

```veln
use event from "std"
use observe from "std"

fn completed() -> () effects [observe::Observe] callsite
	event::emit(None, "checkout.completed", list_cons(observe::Attribute("attempt", observe::IntValue(1)), list_nil()))
end
```

Use `metric::counter_increment` to record an integer increment and
`metric::histogram_sample` to record an integer sample. Supply optional trace
context, a stable metric name, the integer value, and an ordered list of typed
attributes:

```veln
use metric from "std"
use observe from "std"

fn measured() -> () effects [observe::Observe] callsite
	let attributes = list_cons(observe::Attribute("route", observe::StringValue("checkout")), list_nil())
	metric::counter_increment(None, "request.count", 1, attributes)
	metric::histogram_sample(None, "request.duration_ms", 17, attributes)
end
```

`Observe` has three operations: `start_span(SpanRequest) -> Span`,
`emit(Observation) -> ()`, and `finish_span(SpanFinish) -> ()`. `Span` is
opaque. Construct a request with a stable name, an optional parent
`TraceContext`, and an ordered list of linked `TraceContext` values. Pass an
empty list when the new span has no causal links. `observe::context(span)`
returns the context of a recording span and returns `None` for a no-op span.
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
	trace::in_span(observe::SpanRequest("work", None, list_nil()), work)
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

Create a production policy with `observe::new_production(limit)` and install
`observe::production(policy)` at the outer execution boundary. The limit is
the maximum number of distinct metric series retained by that policy. A
negative limit is normalized to zero. Read its non-consuming
`observe::production_health(policy)` snapshot after or during handling:

```veln
let policy = observe::new_production(100)
let result = handle measured() with observe::production(policy)
match observe::production_health(policy)
	observe::ProductionHealth(series, admitted, dropped) => ()
end
```

This handler remains useful when an application needs admission without
export. To export synchronously, construct a `Resource`, an
`InstrumentationScope`, finite `ExporterLimits`, and a JSON Lines exporter.
Install `observe::json_lines(exporter)` at the application's outer effect
boundary. `observe::new_json_lines_default` writes records to standard error,
which keeps application standard output separate. `observe::new_json_lines`
accepts an explicit record destination and a separate diagnostic destination:

```veln
let limits = observe::ExporterLimits(32, 1024, 8, list_cons("password", list_nil()))
let exporter = observe::new_json_lines_default(
	observe::Resource("checkout", list_nil()),
	observe::InstrumentationScope("checkout.web", "1"),
	100,
	limits,
	"checkout",
)
let result = handle measured() with observe::json_lines(exporter)
```

The identity prefix namespaces host-generated trace and span identities. Test
support can inject deterministic identities through the scoped exporter host
adapter. Use `observe::span_request` to attach the call site to an exported
span request. `observe::exporter_health` reads a
non-consuming health snapshot. `observe::report_exporter_health` writes one
summary only to the configured diagnostic destination; a disabled diagnostic
destination makes that call a successful no-op.

`observe::StandardError` writes and flushes the process standard error stream.
`observe::FileDestination(path)` creates the file when needed and appends
UTF-8 bytes without truncating prior records.
Diagnostic summaries use the separate `observe::DiagnosticStandardError`,
`observe::DiagnosticFileDestination(path)`, and
`observe::NoDiagnosticDestination` choices. The disabled choice accepts a
summary without producing output and cannot be selected for observation
records.
The default constructor uses standard error for records and the disabled
destination for health summaries. The explicit constructor accepts these
choices independently for records and summaries.

For direct composition, `observe::apply_metric_cardinality` applies one
record to a production policy and returns whether that record is admitted.
`observe::export_observation` writes one observation without applying that
policy. `observe::start_export_span` writes a span-start record and returns its
span handle. `observe::finish_export_span` writes a finish record for a span
returned by `observe::start_export_span`. The `observe::json_lines` handler
combines these lower-level operations and is the ordinary installation route.

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

Use the public generic `Traced<A>` value when one channel must carry a payload
with an explicit trace context. After `use traced from "std"`, construct
`Traced(payload, context)`, send it through an ordinary channel, and match the
same constructor after receipt:

```veln
let values = channel::bounded<Traced<String>>(1)
let _ = channel::send(values.tx, Traced("work", context))
match channel::recv(values.rx)
	Some(Traced(payload, attached)) => log::info(Some(attached), payload, list_nil())
	None => ()
end
```

The receiver chooses how to use the attached context. An observation helper
receives it only when the program passes it explicitly.

For fan-in, choose one received context as the parent and put every other
causal input context that must be retained in the link list. Parentage is not
inferred from receive order or task creation:

```veln
let request = observe::SpanRequest(
	"consume",
	Some(chosen_parent),
	list_cons(other_input, list_nil()),
)
trace::in_span(request, consume)
```

Use `observe::inject_context` before an adapter sends trace context across a
process boundary. It returns a protocol-neutral `TraceCarrier` with optional
trace identity, span identity, and sampling fields. The adapter maps those
fields to its external protocol. On receipt, map the external fields back to a
carrier and call `observe::extract_context`:

```veln
let context = observe::TraceContext("trace-1", "span-2", false)
let carrier = observe::inject_context(context)
match observe::extract_context(carrier)
	Some(observe::TraceContext(trace_id, span_id, sampled)) => ()
	None => ()
end
```

Extraction returns `None` when any carrier field is absent or either identity
is empty. Callers must handle that result as absent context.

## Behavior

### Metrics

Each metric helper performs exactly one `emit` operation. A counter increment
produces `Observation::CounterIncrement` with a `CounterIncrementRecord`; a
histogram sample produces `Observation::HistogramSample` with a
`HistogramSampleRecord`. Both records preserve the supplied name, integer
value, attributes in order, and optional `TraceContext`. The distinct
observation variants preserve metric kind even when two records have the same
name and attributes.

Both helpers are call-site-aware. An ordinary call records the complete span
of its call expression. A call-site-aware wrapper forwards the outer caller's
location. The recording handler stores metric records in the same operation
order as log, event, and span entries. Repeated snapshots retain earlier
metric entries without consuming them.

### Production metric cardinality

Each production policy has an independent finite metric-series limit and
state. A series identity consists of the metric kind, metric name, and the
complete typed attribute multiset. Attribute order does not affect identity,
but duplicate multiplicity and `BoolValue`, `IntValue`, and `StringValue`
remain significant. The counter increment or histogram sample value, trace
context, and source location do not participate in identity.

The handler admits the first distinct series until the configured limit is
full. It continues to admit observations for every known series. It drops an
observation only when that observation has a previously unseen metric series
and the limit is full. A zero limit therefore drops every metric observation;
a negative configured value is treated as zero. Counter and histogram records
with the same name and attributes remain distinct series.

`ProductionHealth` reports the retained series count, admitted metric
observation count, and dropped metric observation count. Both observation
counts saturate at the maximum `Int` instead of overflowing. Reading health
does not consume or reset it. Metric decisions and health snapshots are
serialized across tasks that share the policy, and separately constructed
policies do not share state. If a task observes cancellation before it enters
the metric decision's commit phase, that decision leaves the policy state
unchanged; later metric emissions and health reads remain available. The
retained identity index never exceeds the normalized series limit, and health
uses a fixed number of scalar fields. Identity lookup uses a bounded ordered
index rather than scanning all retained series. Each emission or health read
uses its calling task as the scoped transaction owner: a policy creates neither
a permanent owner task nor an auxiliary task per operation. At most one policy
state token is acquired at a time. Caller-local commit markers can remain live
until their deferred cleanup runs after the token is published; their number
is not bounded independently of active calls. Cancellation before the commit
phase restores the acquired token before the operation exits.

Dropping a metric returns normally and does not change the handled body's
result. Logs, events, span starts, and span finishes remain accepted terminal
operations and do not change metric cardinality or health. A production span
has no recording context, as with a no-op span.

### Named domain events

`event::emit` performs exactly one `emit` operation. The emitted
`Observation::Event` contains an `EventRecord` with the supplied stable name,
the supplied attributes in order, the supplied optional `TraceContext`, and
one `SourceLocation`. The attribute values have the same typed scalar forms as
log attributes. Empty attributes and absent context are preserved as `Nil` and
`None`.

The helper is call-site-aware. An ordinary caller records the complete span of
its call expression. A call-site-aware wrapper forwards its hidden source
location, so the event identifies the outer user call rather than the helper
implementation. The source path follows the canonical relative-source rules
of [Call-site Declarations](call-site-declarations.md).

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
strings and Boolean sampling state. The recording handler preserves these
values. The JSON Lines handler applies its configured attribute policy during
export.

Each log helper is call-site-aware. An ordinary caller records the complete
span of its call expression. A call-site-aware wrapper forwards its own hidden
source location, so the observation identifies the outer user call rather than
the helper implementation. The path follows the canonical relative-source
rules of [Call-site Declarations](call-site-declarations.md).

### No-op handler

`observe::noop()` handles every `Observe` operation and has no retained host
effect. It discards each emitted observation and each span finish. A span start
returns an opaque span value, so the operation is total. Programs cannot
construct or pattern-match the span representation. They can call
`observe::context` to inspect its optional recording context; the accessor
returns `None` for a no-op span. Installing the handler preserves the handled
program's result and opens no output destination. Counter increments and
histogram samples follow the same discard rule.

### Synchronous JSON Lines export

The JSON Lines handler applies metric-cardinality admission to the original
metric kind, name, and complete typed attribute multiset. It does this before
redaction, truncation, or attribute-count limits. A rejected unseen metric
series produces no line. Every accepted log, event, counter, histogram,
span-start, and span-finish operation attempts exactly one synchronous write
of one versioned JSON object followed by `\n`. Accepted operations consume
handler sequence values in operation order, including a write that the
destination rejects.

Each record follows the checked
[`json-lines-record.schema.json`](../../crates/veln-stdlib/schemas/observability/v1/json-lines-record.schema.json).
It contains the observation kind and name, wall-clock seconds and nanoseconds,
handler sequence, nullable severity and trace/span identity, ordered links,
source, nullable task identity, typed attributes, resource identity,
instrumentation-scope identity, cumulative cardinality and exporter failure
facts, and per-record attribute facts. Log records add `message`; metric
records add `value`. The current host does not expose a stable task identity,
so it encodes `task_id` as `null`.

Attribute count limits apply independently to observation and resource
attribute lists. Negative limits normalize to zero. String size is measured
in JVM UTF-16 code units. The size limit applies to every retained observation
and resource attribute name and to every retained, non-redacted `StringValue`
payload. Each name or value beyond the limit is shortened to the configured
size and increments `truncated`; one attribute can therefore add two to that
fact. Attributes after the count limit are omitted and increment `dropped`.
Redaction matching uses the original attribute name before name shortening. A
matching value is replaced by the fixed string `[REDACTED]` until the finite
redaction limit is reached and increments `redacted`. The encoded facts
aggregate changes to both observation and resource attributes.

A destination rejection or host write failure increments `failed_exports`
and leaves the handled application's ordinary result unchanged. The
cumulative failed, dropped, truncated, and redacted health totals saturate at
the maximum `Int`. Health reads do not consume those totals. Saturation does
not prevent a later export or health read. Export and health-summary writes do
not perform `Observe`, so a failure cannot recursively create telemetry.

Every direct or handler-driven export validates all public `SourceLocation`
components before encoding. A workspace source has an empty `package`; a
dependency source has its public package identity. `module` is a logical
module path. `file` is package-relative or a canonical virtual name. For each
component independently, the encoder replaces an absolute Unix, Windows, or
UNC spelling, a `.` or `..` traversal segment, an invalid path, or another
noncanonical component with `<invalid-source>`. It preserves compiler-produced
canonical values. Equivalent source trees at different absolute roots
therefore encode the same source identity.

### Direct exporter operations

`observe::apply_metric_cardinality(production, record)` returns `true` for a
non-metric observation, a known metric series, or a new series admitted below
the configured limit. It returns `false` for an unseen series after the limit
is full. Metric calls update the same admitted, dropped, and retained-series
state read by `observe::production_health`.

`observe::export_observation(exporter, record)` synchronously writes the
matching log, event, counter, or histogram record. It does not apply
metric-cardinality admission. A direct caller that needs the handler's metric
policy must call `observe::apply_metric_cardinality` first and export only an
admitted record.

`observe::start_export_span(exporter, request)` allocates the span identities,
writes the span-start record, and returns a span whose context is available
through `observe::context`. A located request created by
`observe::span_request` supplies the call site; an ordinary `SpanRequest`
produces the exporter fallback source. `observe::finish_export_span` writes a
span-finish record only when its `SpanFinish` contains an exporter span. Other
span representations are accepted as a no-op. Each direct start, finish, or
observation export uses the same sequence, destination, attribute-limit, and
failure-health rules as handler-driven export.

### Span lifecycle and recording

A root span receives a trace identity and a span identity from its recording.
A request with a parent context retains the parent's trace identity, records
the parent's span identity, and receives a distinct span identity. The parent
context and link list are independent request fields: zero links is valid, and
the recording preserves linked trace and span identities in caller-supplied
order. Each recording allocates identities in order from its caller-supplied
prefix and retains state in its own channel-backed scope. If a recording's
next span-identity candidate equals the supplied parent span identity, the
handler skips that candidate without changing the sampling state inherited
from the parent. Children accepted by one recording therefore remain distinct
from their parent and siblings. This local guarantee does not make identities
globally unique across unrelated recordings or processes.
A no-op span has no recording context, so `observe::context` returns `None`
for it.

`trace::in_span` registers the matching finish immediately after start. The
finish runs exactly once after a normal `Ok` return, after an `Err` result that
the caller propagates, and while a contract or runtime failure unwinds the
body. An existing abrupt failure remains primary. The finish entry uses the
same trace and span identities as its start entry.

`observe::recorded` returns a sequential snapshot in operation order without
consuming entries or resetting the next identity. A later snapshot includes
the preserved prefix followed by entries from subsequent recording operations.
`SpanStarted` exposes the name, trace identity, span identity, optional parent
span identity, ordered link contexts, and monotonic millisecond reading.
`SpanFinished` exposes the matching identities and finish reading. `Emitted` retains an emitted
observation alongside span entries, so one recording preserves the order
across all three `Observe` operations.

### Explicit fan-in context

A consumer selects parentage by placing one `TraceContext` in the optional
parent field of `SpanRequest`. It retains other causal inputs by placing their
contexts in the request's link list. The selected parent determines the new
span's trace identity. Linked contexts do not affect parentage or identity
allocation. The recording handler keeps parent and links in distinct fields,
including an empty link list.

The selection is explicit and independent of channel arrival order and worker
creation context. Starting and finishing the span preserve the body's ordinary
result. Repeated recording snapshots preserve the same parent and links without
consuming them. Raw task and channel operations continue to perform no implicit
observation-context propagation.

### Explicit child-task context

`traced::spawn` creates an ordinary child task through the task API and passes
the supplied `TraceContext` unchanged to its callback. The callback's effects
remain part of the helper's effect row. A scoped observation handler active at
task creation is inherited under the ordinary task-handler rule, so a child
record can retain the context when the callback explicitly supplies it to a
log or observation helper.

The helper does not create a span, allocate identifiers, install a handler, or
infer ambient context. Raw `task::spawn`, `task::spawn_with`, and channel
operations do not attach observation context implicitly. A program can pass a
`TraceContext` explicitly through the ordinary context argument of
`task::spawn_with`; `traced::spawn` provides the named observability helper for
that pattern.

### Explicit channel-value context

`Traced<A>` is an ordinary public generic value with one public constructor,
`Traced(value: A, context: TraceContext)`. Construction preserves the payload
and context unchanged. Matching the constructor returns those same two values.
The wrapper performs no effect and does not alter channel ordering, closure,
blocking, or failure behavior.

A long-lived worker can receive several `Traced<A>` values and select the
context attached to each value independently. Receiving or sending an
unwrapped `A` does not add a context, even when the task inherited an active
observation handler. Raw channel and task operations do not inspect payloads,
infer ambient context, or apply a context to later observations.

### Explicit carrier context

`TraceContext` contains an opaque trace identity string, an opaque span
identity string, and a Boolean sampling state. `inject_context` copies all
three values into the corresponding `TraceCarrier` fields. It performs no
effect and selects no external encoding.

`extract_context` returns a context only when the carrier has all three fields
and both identity strings are non-empty. A successful extraction preserves
the two identities and the sampling state unchanged. A missing field or empty
identity returns `None`; malformed carrier data therefore does not produce a
valid context. Injection does not validate a directly constructed context, so
injecting an empty identity and then extracting it also returns `None`.

A recording root span uses `true` as its sampling state. A recording child
span inherits its parent's sampling state. Carrier injection and extraction
do not create spans, allocate identities, install handlers, or make context
ambient. Raw task and channel operations remain unchanged.

## Limits

Metric query APIs, asynchronous metric callbacks, buffered export, automatic
shutdown flushing, and external telemetry-protocol adapters are not
available. The synchronous exporter has no retry or durable-audit guarantee.
Its identity bound counts series rather than the bytes in their names and
attributes. The standalone cardinality handler does not retain or export
accepted observations. The no-op handler does not record,
enrich, buffer, export, or report observations. `trace::in_span` currently
accepts only a body returning `Result<(), String>`, while forwarding that
body's effect row; it does not define rescue or exception recovery.
`traced::spawn` has the same callback-result restriction. `Traced<A>` does not
provide send, receive, fan-in, or context-selection helpers and does not
propagate context through raw task or channel operations.

`TraceCarrier` is a normalized library representation, not an external wire
format. External adapters remain responsible for protocol-specific field
names, serialization, identifier syntax and width, and malformed-input
decoding before constructing a carrier. Carrier extraction validates only
field presence and non-empty identities. Direct construction of
`TraceContext` does not perform validation.

An unhandled `Observe` effect follows the ordinary nominal-effect runnable
boundary in [Effects](effects.md): a runnable entry must handle it before
execution. Span handles and finish values remain opaque. Deferred finish has
the lexical cleanup limits in [Execution Boundary](execution.md#cleanup-limits):
process or machine termination can prevent it from running.

## References

- Standard-library API coverage:
  [`observe.test.veln`](../../crates/veln-stdlib/veln/observe.test.veln) and
  [`log.test.veln`](../../crates/veln-stdlib/veln/log.test.veln), named event
  coverage in
  [`event.test.veln`](../../crates/veln-stdlib/veln/event.test.veln), metric
  coverage in
  [`metric.test.veln`](../../crates/veln-stdlib/veln/metric.test.veln), plus scoped
  span lifecycle coverage in
  [`trace.test.veln`](../../crates/veln-stdlib/veln/trace.test.veln), carrier
  context coverage in `observe.test.veln`, and explicit task-, channel-value-,
  and fan-in-context coverage in
  [`traced.test.veln`](../../crates/veln-stdlib/veln/traced.test.veln).
- JSON Lines API, ordering, limit, cardinality, and failure-health coverage:
  [`observe.test.veln`](../../crates/veln-stdlib/veln/observe.test.veln), with
  scoped fakes in `observe_export_test_support_test.veln`.
  The checked schema and six captured encoder records are owned by
  [`observability-schema.test.mjs`](../../workflow-scripts/observability-schema.test.mjs)
  and the
  [`observability-json-lines-encoder`](../../examples/specification/run/observability-json-lines-encoder/)
  JVM integration case. They check every record kind, exact JSON Lines framing,
  limit facts, and hostile-source replacement.
- Production index boundary coverage:
  [`effect_boundary_policy.rs`](../../crates/veln-backend-jvm/src/tests/effect_boundary_policy.rs)
  checks bounded indexed state and the absence of owner tasks, while
  [`java_helpers_and_mapping.rs`](../../crates/veln-backend-jvm/src/tests/java_helpers_and_mapping.rs)
  checks the private adapter mappings.
- Public task-context execution evidence:
  [`observability-traced-task-context`](../../examples/specification/run/observability-traced-task-context/).
- Public channel-value context execution evidence:
  [`observability-traced-channel-context`](../../examples/specification/run/observability-traced-channel-context/).
- Call-site and effect-dispatch execution evidence:
  [`observability-log-severities`](../../examples/specification/run/observability-log-severities/)
  and
  [`observability-event-callsite`](../../examples/specification/run/observability-event-callsite/).
- Public metric-module and recording-handler execution evidence:
  [`observability-metric-recording`](../../examples/specification/run/observability-metric-recording/).
- Public no-op handler execution evidence:
  [`observability-noop-handler`](../../examples/specification/run/observability-noop-handler/).
- Runtime-failure cleanup and primary-failure evidence:
  [`observability-span-runtime-failure-human`](../../examples/specification/run/observability-span-runtime-failure-human/)
  and
  [`observability-span-runtime-failure-json`](../../examples/specification/run/observability-span-runtime-failure-json/).
