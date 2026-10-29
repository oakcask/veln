---
role: proposal
update-when: The buffered observation-export capacity, admission outcomes, failure accounting, shutdown state machine, summary ordering, or interruption contract changes.
---

# Buffered Observation Export Lifecycle

The synchronous exporter and the normal direct buffered lifecycle are
implemented in
[standard-library observability](../specification/standard-library-observability.md).
This proposal defines only the remaining buffered handler, span, failure,
concurrency, and interruption work. The evidence below is planned, not passing
evidence for current behavior.

## Purpose and API Boundary

The implemented direct API admits log, event, counter, and histogram records,
reports health, drains on normal shutdown, writes the final diagnostic summary,
and freezes its completed report. The remaining API surface is:

| Operation | Contract |
| --- | --- |
| `buffered_json_lines` | Install the buffered `Observe` handler. Observation operations retain their existing unit or span-handle results. |
| `buffered_start_export_span` | Return the allocated span handle together with its start-record admission outcome. |
| `buffered_finish_export_span` | Submit the finish record and return its admission outcome. |

The handler consumes admission outcomes without changing the application's
ordinary result. A rejected span start still returns its allocated handle; its
later finish is a separate submission. No buffered default capacity, unbounded
mode, or public manual-summary operation is planned.

## Admission and Record Ordering

Concurrent submissions must extend the implemented admission order without
changing its sequence and destination-order guarantees. The handler applies
metric-cardinality admission before validation and capacity admission.
Cardinality rejection emits no record and consumes no sequence. A metric that
passes cardinality admission and is then rejected as full retains its admitted
series facts.

The remaining failure work must contain destination exceptions, continue with
later records, saturate health counters, and leave the handled application
result unchanged. An accepted record keeps its admission-time observation,
context, timestamp, sequence, resource, scope, transformations, and health
facts. A failed or interrupted host attempt can have committed a byte prefix,
so `failed_exports` is accounting rather than proof of byte loss.

## Shutdown State Machine

Normal single-caller shutdown and completed-report reuse are implemented. The
remaining state-machine work makes concurrent shutdown callers join one shared
`Closing` transition and makes cancellation of any waiting caller request
interruption of that transition. Ordinary task cancellation still propagates
through the language cleanup boundary; it does not become an application
success value.

| State | Event and guard | Next state | Visible result and preserved state |
| --- | --- | --- | --- |
| Closing | Shutdown interruption before completion | Closed | Revoke future destination calls; account for every unsettled record once; freeze an interrupted report. |
| Closing | Another shutdown caller | Closing | Observe the shared transition; do not start another drain or summary. |
| Closed | Any shutdown caller or a late destination completion | Closed | Return or preserve the frozen report; no retry, new call, or counter change. |

Interruption atomically prevents further record or summary calls from starting.
It discards queued records and classifies an unresolved in-flight record as
failed or unconfirmed. The report accounts for each such record exactly once.
If interruption precedes the summary call, the summary outcome is
`NotAttempted`. If interruption occurs during the summary call, its outcome is
`Rejected` unless completion was already confirmed. Already confirmed outcomes
remain confirmed. An interruption that loses the race to `Closed` has no
exporter effect.

Interrupted shutdown does not wait for an uninterruptible host call to finish.
That already-started call can commit bytes after shutdown returns. Its eventual
completion cannot change the frozen report, release another record for export,
retry, or start a final summary. This is an explicit limit on interrupted
shutdown, not a claim of definite byte loss. Normal completed shutdown has no
outstanding destination call and produces no later output. Without an
interruption, shutdown can wait indefinitely for a blocking host destination;
this proposal adds no hard deadline or guarantee of host-call cancellation.
Process or machine termination can prevent both cleanup and accounting.

## Effect and Ownership Boundaries

Provider retention for the direct worker is implemented. Remaining scoped Veln
tests must cover handler use, span operations, child-task sharing,
independently constructed state, controlled failure, and interruption without
adding ambient fake selectors to production providers.

Telemetry delivery failures leave the handled application's ordinary value,
error, or cancellation unchanged. Exporter-internal failures must be contained
at the exporter boundary so a worker failure cannot escape through a shutdown
join as a new application failure. The existing
[effect](../specification/effects.md) and
[execution cleanup](../specification/execution.md#cleanup-limits) contracts
remain the authority for task cancellation and process termination.

## Related Work and Decisions

The inspected OpenTelemetry
[Tracing SDK batching processor](https://opentelemetry.io/docs/specs/otel/trace/sdk/#batching-processor)
uses finite capacity, drops at capacity, and serializes exporter calls. Veln
adopts those boundaries but requires an explicit capacity and counts the
in-flight record within it. Rejecting new work preserves the already-accepted
prefix and prevents a slow destination from blocking observation calls.
OpenTelemetry's span defaults do not establish an appropriate Veln capacity:
Veln handles logs, events, metrics, and span records with differing sizes.

OpenTelemetry's
[processor shutdown and force flush](https://opentelemetry.io/docs/specs/otel/trace/sdk/#shutdown-1)
include flushing and allow timeout to take priority over finishing export.
Veln adapts this to interruption with explicit uncertain-delivery accounting.
A strict deadline cannot establish that a blocking host write has stopped.
The selected contract therefore freezes health and prohibits new calls while
allowing a previously started host call to finish late. Waiting indefinitely
after interruption and pretending that late success proves earlier delivery
were rejected. These are Veln design choices, not OpenTelemetry requirements.

## Acceptance Model

All remaining cases below are planned. The state machine and existing v1
schemas supply expected outcomes independently of their implementation.

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| O17 | Concurrent handler submissions and cardinality rejection | FIFO accepted sequence with specified gaps; cardinality precedes queue admission without rollback; invalid/full outcomes retain the direct API's accounting rules. | Companion controlled schedules and schema-checked captures. |
| O18 | Record write or flush exception and counter saturation | One failed increment per exceptional record; later records proceed; maximum counter does not overflow; application result unchanged. | Companion failure schedules and saturation cases. |
| O19 | Concurrent shutdown and emission races closure | One drain and at most one summary; concurrent callers share the report; losing submission is `Closed` without state changes. | Companion barrier-controlled transitions. |
| O20 | Diagnostic destination exception | Contain the exception as one rejected summary outcome; do not change export health or the application's ordinary result. | Companion failure schedule and schema checks. |
| O21 | Interruption with queued work and an unresolved host call | Prompt interrupted report after call-start revocation; unsettled records counted once; no new record/summary calls. | Companion cancellation schedule. |
| O22 | In-flight record or summary completes after interrupted return | Late bytes are permitted; frozen report, counters, and call count remain unchanged. | Companion late-completion schedule. |
| O23 | Interruption before summary, during summary, and after closed | `NotAttempted`, unconfirmed `Rejected`, and unchanged completed report respectively. | Companion state-boundary schedules. |
| O24 | Exporter is shared with a child task; another exporter is constructed independently | The shared exporter retains one provider and state; independently constructed exporter state remains separate. | Scoped Veln companion tests; backend case only for independently identified provider/cleanup integration. |

Use the [test placement policy](../reference/toolchain-test-harness.md) for API
coverage under `crates/veln-stdlib/veln/`. Run companion tests through the
repository's guarded test runner when implementation lands. A separate real
host case must check actual destination writes and cleanup; fake-provider
success cannot establish that production effects remain real.

Once these cases pass, update the current observability specification and
executable evidence, then remove this proposal and its catalog entry. External
telemetry protocols, automatic exporter selection, persistent spooling, and
durable audit guarantees are outside this proposal.
