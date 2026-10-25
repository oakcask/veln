---
role: proposal
update-when: The buffered observation-export capacity, admission outcomes, failure accounting, shutdown state machine, summary ordering, or interruption contract changes.
---

# Buffered Observation Export Lifecycle

The synchronous JSON Lines exporter and its data contract are implemented in
[standard-library observability](../specification/standard-library-observability.md).
This proposal defines the remaining buffered exporter. Its decisions and
acceptance conditions are complete enough for implementation; the evidence
below is planned, not passing evidence for current behavior.

## Purpose and API Boundary

Buffering lets application observation calls finish without waiting for the
record destination. An application explicitly constructs the buffered
exporter, installs its handler, and shuts it down after its producers finish.
Leaving a handler scope does not implicitly shut down a shared exporter.
An application can register explicit shutdown with `defer`.

The planned API has these operations:

| Operation | Contract |
| --- | --- |
| `new_buffered_json_lines` | Accept the existing explicit JSON Lines constructor inputs plus `capacity: Int`. Return an exporter or `InvalidCapacity(capacity)`. |
| `buffered_json_lines` | Install the buffered `Observe` handler. Observation operations retain their existing unit or span-handle results. |
| `buffered_export_observation` | Submit a direct observation without metric-cardinality policy and return an admission outcome. |
| `buffered_start_export_span` | Return the allocated span handle together with its start-record admission outcome. |
| `buffered_finish_export_span` | Submit the finish record and return its admission outcome. |
| `buffered_exporter_health` | Read the non-consuming health snapshot using the existing exporter-health fields. |
| `shutdown_buffered` | Close admission and return a shutdown report with completion state, frozen health, and final-summary outcome. |

Admission outcomes are `Accepted`, `Full`, `Closed`, and `InvalidRecord`.
`Accepted` means the exporter owns the pending record, not that bytes reached
the destination. The handler consumes these outcomes without changing the
application result. A rejected span start still returns its allocated handle;
its later finish is a separate submission.

Capacity must be positive. Zero and negative values fail construction before
starting work or opening a destination. There is no buffered default capacity
and no unbounded setting. Every positive representable `Int` is an admissible
logical bound; allocation failure remains a runtime resource failure rather
than a promise that the requested capacity can be preallocated. Capacity bounds
record count, not total bytes: observation messages and other existing record
fields do not all have byte limits.

## Admission and Record Ordering

One exporter has a total admission order consistent with completed calls.
Concurrent calls may take either order. The same order controls sequence
allocation and destination-call order. The capacity counts all accepted records
whose destination attempt has not completed, including the in-flight record.
Removing a record from a waiting queue does not release its capacity slot.

For an open exporter, the handler applies existing metric-cardinality admission
before export validation and capacity admission. Cardinality rejection emits no
record, consumes no exporter sequence, and updates only its existing policy
facts. A cardinality-admitted metric can subsequently be rejected as full;
its retained metric-series identity and admitted count are not rolled back.

Every remaining open-export submission allocates a sequence. Invalid record
validation produces `InvalidRecord` and increments `failed_exports` once.
Validation precedes the full-capacity check, so an invalid record at capacity
reports `InvalidRecord`. A valid submission at capacity returns `Full`
immediately without waiting for destination progress, evicting an older record,
or changing the queue. It increments `failed_exports` once. Full and invalid
submissions do not add attribute-transformation counters.

An accepted record captures its immutable observation, context, timestamp,
sequence, resource, scope, attribute transformations, and cumulative health
facts at admission. Later task-context changes and destination failures do not
rewrite that record. The existing schema, cardinality ordering, redaction, and
attribute-limit rules apply. Delivered records may contain sequence gaps and
older failure totals; the final health snapshot includes later failures.

The exporter makes at most one destination attempt per accepted record, in
sequence order, with at most one record call in flight. It begins draining when
work is available; no batch threshold or periodic flush interval delays the
first attempt. A successful attempt includes the existing destination's write
and flush completion. A rejection, exception, or unconfirmed interrupted
attempt increments `failed_exports` once. A failed attempt does not retry or
prevent the next record from being attempted. Partial host output can remain
visible, as with the synchronous exporter; this is not a durable delivery API.

`failed_exports` already represents rejected or failed observation-export
attempts. It also accounts for full admission and accepted records abandoned
at shutdown. It does not mean that no bytes were delivered: a failed host
write or an interrupted in-flight call can have committed bytes. No new v1
JSON fields are added. All existing health counters retain saturating `Int`
arithmetic. Counting one record cannot overflow or recursively perform
`Observe`.

## Shutdown State Machine

The states are `Open`, `Closing`, and `Closed`. The first shutdown call owns one
transition to `Closing`. A concurrent shutdown joins the same transition; a
later call returns the same frozen report. Cancellation of any waiting shutdown
caller requests interruption of that shared transition. Ordinary caller task
cancellation still propagates through the language's cleanup boundary; it does
not become a successful application result.

| State | Event and guard | Next state | Visible result and preserved state |
| --- | --- | --- | --- |
| Open | Valid submission and outstanding count below capacity | Open | `Accepted`; append one immutable record and retain one slot. |
| Open | Valid submission at capacity | Open | `Full`; increment failed count once; preserve pending records. |
| Open | Invalid record | Open | `InvalidRecord`; increment failed count once; no destination call. |
| Open | First shutdown | Closing | Close admission atomically; retain accepted records for draining. |
| Closing or Closed | Submission | Same | `Closed`; no sequence, policy, health, or queue change and no destination call. |
| Open or Closing | Record destination completes | Same | Release its slot; on rejection or failure account for it once. |
| Closing | All accepted records settled and no interruption | Closed | Attempt the final diagnostic summary once, then freeze the report. |
| Closing | Shutdown interruption before completion | Closed | Revoke future destination calls; account for every unsettled record once; freeze an interrupted report. |
| Closing | Another shutdown caller | Closing | Observe the shared transition; do not start another drain or summary. |
| Closed | Any shutdown caller or a late destination completion | Closed | Return or preserve the frozen report; no retry, new call, or counter change. |

Normal shutdown settles every accepted record before attempting its final
health summary. Settling means a confirmed successful write/flush or one
failed-export accounting increment. The summary goes only to the configured
diagnostic destination. If both destinations identify the same output, all
record calls finish before the summary call starts. An empty exporter still
attempts one summary when diagnostics are enabled. A disabled diagnostic
destination produces no output.

The shutdown report contains `Completed` or `Interrupted`, the final existing
health record, and summary outcome `Accepted`, `Rejected`, `Disabled`, or
`NotAttempted`. `Completed` means lifecycle work finished; failed exports may
still be nonzero. Summary rejection or exception yields `Rejected`, without
incrementing failed exports, retrying, or changing the ordinary application
result. No public manual-summary operation is added for buffered exporters;
this keeps the final report and its one summary unambiguous.

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

The exporter retains the destination provider selected when it is constructed.
Its worker does not switch providers after the constructing handler scope ends.
Tasks using one exporter share its admission order and health; independently
constructed exporters share neither. Scoped Veln test providers supply
controlled destination completion and interruption. Production providers use
real host effects and do not select fakes through ambient variables.

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

All cases below are planned. The state machine and existing v1 schemas supply
the expected outcomes independently of the eventual queue implementation.

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| O14 | Shutdown after accepted records | Every accepted record is settled before normal return; final summary follows all record attempts. | Stdlib companion lifecycle test with scoped Veln destination support. |
| O15 | Capacity one; first destination call held; submit a second valid record | Second result is `Full`; first remains accepted and occupies capacity; exactly one failed export. | Companion admission barrier test. |
| O16 | Capacity zero or negative; capacity one becomes available after completion | Invalid capacity has no worker or I/O effects; successful completion permits the next admission. | Companion constructor and capacity boundary cases. |
| O17 | Concurrent submissions, invalid input, full queue, and cardinality rejection | FIFO accepted sequence with specified gaps; invalid/full do not transform attributes; cardinality precedes queue admission without rollback. | Companion controlled schedules and schema-checked captures. |
| O18 | Record write or flush rejection, exception, and counter saturation | One attempt and one failed increment per record; later records proceed; maximum counter does not overflow; application result unchanged. | Companion failure schedules and saturation cases. |
| O19 | Repeated and concurrent shutdown; emission races closure | One drain and at most one summary; shared report; losing submission is `Closed` without state changes. | Companion barrier-controlled transitions. |
| O20 | Enabled, disabled, and rejected diagnostic destinations | One ordered summary or no output as configured; summary failure never changes export health. | Companion destination-order and schema checks. |
| O21 | Interruption with queued work and an unresolved host call | Prompt interrupted report after call-start revocation; unsettled records counted once; no new record/summary calls. | Companion cancellation schedule. |
| O22 | In-flight record or summary completes after interrupted return | Late bytes are permitted; frozen report, counters, and call count remain unchanged. | Companion late-completion schedule. |
| O23 | Interruption before summary, during summary, and after closed | `NotAttempted`, unconfirmed `Rejected`, and unchanged completed report respectively. | Companion state-boundary schedules. |
| O24 | Exporter crosses handler scope and is shared with child task | Creating provider retained; outer handler restored; independent exporter state remains separate. | Scoped Veln companion tests; backend case only for independently identified provider/cleanup integration. |

Use the [test placement policy](../reference/toolchain-test-harness.md) for API
coverage under `crates/veln-stdlib/veln/`. Run companion tests through the
repository's guarded test runner when implementation lands. A separate real
host case must check actual destination writes and cleanup; fake-provider
success cannot establish that production effects remain real.

Once these cases pass, update the current observability specification and
executable evidence, then remove this proposal and its catalog entry. External
telemetry protocols, automatic exporter selection, persistent spooling, and
durable audit guarantees are outside this proposal.
