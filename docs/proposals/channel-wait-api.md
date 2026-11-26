---
role: proposal
update-when: The proposed single-receiver and receiver-list channel wait API, selection policy, result contract, cancellation semantics, or acceptance gates change.
---

# Single-receiver and Receiver-list Channel Waits

Organize channel waiting around one receiver and a runtime-sized list of
receivers. Give both forms the same timeout and cancellation capabilities.
Remove the special two-receiver interface. This is a Design In Progress
proposal; none of the proposed signatures or acceptance cases is implemented
by this document.

## Problem and current boundary

The current [concurrency calls](../specification/effects.md#concurrency-calls)
provide a blocking `recv(rx)`, two-receiver `select(left, right)` variants,
and receiver-list `select_many_*` variants. A caller waiting on one receiver
with a timeout or cancellation token must build a singleton list. Changing
from two receivers to a runtime-sized list also changes available selection
policies: ordinary two-receiver selection rotates ties, while list selection
uses supplied order. Timeout and interrupt reporting have separate variants.

The capability gap is direct single-receiver timeout and cancellable waiting,
with consistent behavior when an application changes its receiver count.
Adding another numbered arity or another route-count example does not solve
that problem.

The checker constructs either two `Receiver<T>` parameters or one
`List<Receiver<T>>` parameter in
`crates/veln-sema/src/effects/concurrency.rs`. The JVM helper in
`crates/veln-backend-jvm/src/runtime/concurrency.java.inc` converts the pair
into an array before entering the same list-capable wait implementation.
Two receivers are an API boundary, not a runtime limit. The single-receiver
`channelRecv` currently uses a separate monitor-based path. Reusing the
list implementation must not silently replace that path with polling.

## Recommended API

Use `recv` for one receiver and `select` for a non-empty list. A singleton
list remains valid. These are two families, not source-level variadic
arguments or overloads by argument count.

The following are proposed interface sketches. `Selected<T>` below abbreviates
`{ index: Int, value: T }` in this document; it does not propose another public
type. `ReceiveError` and `SelectionOrder` are proposed exported channel types.
Their publication through the compiler-known channel namespace must be
resolved before Ready promotion.

| Operation | One receiver | Receiver list |
| --- | --- | --- |
| Wait indefinitely | `recv(rx)` | `select(receivers, order)` |
| Wait with a timeout | `recv_timeout(rx, timeout_ms)` | `select_timeout(receivers, timeout_ms, order)` |
| Wait with timeout and cancellation | `recv_timeout_cancellable(rx, timeout_ms, token)` | `select_timeout_cancellable(receivers, timeout_ms, token, order)` |

All one-receiver operations take `Receiver<T>` and return
`Result<Option<T>, ReceiveError>`. All list operations take
`List<Receiver<T>>` and return
`Result<Option<Selected<T>>, ReceiveError>`. All receivers in a list have
the same element type. Selection receives one value as part of the operation;
it does not return a readiness notification that requires a later receive.

Non-cancellable operations carry `effects [concurrency]`. Cancellable
operations carry `effects [time, concurrency]`, following the existing token
observation boundary. Negative timeouts mean indefinite waiting, zero means
an immediate attempt, and positive timeouts are milliseconds. An indefinite
cancellable wait uses the cancellable operation with a negative timeout;
it does not need another family of names.

### Outcomes

| Observation | One receiver | Receiver list |
| --- | --- | --- |
| A value is received | `Ok(Some(value))` | `Ok(Some({ index, value }))` |
| Every participating channel is closed and drained | `Ok(None)` | `Ok(None)` |
| Timeout expires without a received value | `Ok(None)` | `Ok(None)` |
| Explicit token cancellation is observed before receive commitment | `Err(Cancelled)` | `Err(Cancelled)` |
| A host wait interruption is reported, without task cancellation | `Err(Interrupted)` | `Err(Interrupted)` |
| Receiver list is empty | Inapplicable | `Err(EmptyReceivers)` |

`Cancelled`, `Interrupted`, and `EmptyReceivers` denote inspectable variants
of `ReceiveError`, not strings that callers must parse. Every entry point
returns `Result`; there is no separate `_result` family that callers must
choose to avoid losing interruption information. This deliberately changes
the return type of existing `recv` and non-result selection operations.

This proposal keeps the existing `Option` convention for closure and timeout.
It does not claim that those states are distinguishable. A future outcome
ADT could distinguish them, but changing that convention is not necessary
to make receiver cardinality and waiting capabilities consistent.

Task cancellation continues to unwind through the existing task and cleanup
boundary. It must not become a catchable ordinary `ReceiveError` that lets a
cancelled task resume. Explicit `CancelToken` observation and host interruption
are distinct from that task-lifetime rule.

### Selection policy

Every list operation requires an explicit `SelectionOrder` argument. No name
suffix or receiver count chooses a policy implicitly.

| Policy | Required observation |
| --- | --- |
| `Priority` | If several receivers remain ready throughout an attempt, choose the lowest original list index. A continuously ready early receiver can starve later ones. |
| `Unspecified` | Choose any ready receiver. Do not promise alternation, randomness, or starvation freedom. |

A selection attempt need not observe an atomic snapshot of all channels.
Arrivals and closures during an attempt may change the available choices.
`Unspecified` allows the existing rotating implementation, but does not
promise its process-wide cursor behavior. Applications that require stable
priority choose `Priority`. Exact per-caller round-robin state would need an
explicit stateful selector and is outside this proposal. The choice to relax
the old two-receiver alternation contract is a review gate, not an assertion
that it has no consumers.

### Cardinality, races, and preserved state

- A list may contain one, two, or any runtime-sized number of receivers that
  fits available resources. No receiver count has a separate signature.
- A returned index is zero-based in the original list. Closed receivers are
  skipped without renumbering other entries. One closed receiver does not
  terminate selection while another remains open.
- Validate an empty list before waiting or observing cancellation. It returns
  `EmptyReceivers`, including with timeout zero or a cancelled token.
- Repeated references to the same receiver are allowed. At most one value is
  consumed. Under `Priority`, the earliest occurrence wins if that receiver
  supplies the selected value. Under `Unspecified`, any matching occurrence
  may be reported. Duplicates must not create extra transfers.
- After input validation, an already-cancelled token wins over ready values,
  closure, and timeout. If cancellation is not observed, an immediately ready
  value wins over timeout zero. Closed-and-drained inputs return `Ok(None)`.
- Concurrent arrival, cancellation, timeout, and closure have a single receive
  commitment boundary. Exactly one winning receive consumes a value. A
  cancellation observed before that commitment prevents consumption. A
  cancellation racing after commitment does not retract the returned value.
  No total ordering of physically simultaneous events is promised.
- Timeout, cancellation, interruption, and invalid-input outcomes consume no
  values. Losing channels retain their values and FIFO order. Selected
  channels retain FIFO order after the one consumed value.
- Capacity-zero rendezvous obeys the same one-value commitment rule. A losing
  pending sender must not report a successful transfer caused solely by
  temporary registration of the losing wait.

For fresh equivalent channel states and identical wait conditions, a
singleton `select` must have the same value, closure, timeout, error,
cancellation, and consumption outcome as the corresponding `recv`, after
removing the index field. Singleton selection always reports index zero.
The rule does not compare two consecutive consuming calls on one channel.

## Usage and source form

Illustrative future source, not a currently executable example:

```veln
fn receive_one(rx: Receiver<String>) -> Result<Option<String>, ReceiveError> effects [concurrency]
	channel::recv_timeout(rx, 1000)
end

fn receive_any(receivers: List<Receiver<String>>) -> Result<Option<{ index: Int, value: String }>, ReceiveError> effects [concurrency]
	channel::select_timeout(receivers, 1000, SelectionOrder::Priority)
end
```

These calls use existing function-call, list, record, and ADT syntax. There
is no new grammar, variadic syntax, default argument, or overload resolution.
Formatting follows the current
[source surface](../specification/source-surface.md) and
[formatter contract](../specification/command-fmt.md).
`veln fmt` does not migrate call names, arity, policies, or return types.
Migration needs explicit source edits because old calls can have different
selection and error semantics. Existing single-line and multiline call
formatting and comment attachment rules continue to apply; formatting a
migrated call twice must produce identical source.

## Alternatives and related work

### Keep two-receiver convenience calls

Retaining `select(a, b)` would preserve a short spelling but create a third
shape alongside single-receiver and list waiting. The current common JVM
implementation shows that this distinction buys no additional capability.
The recommendation removes it rather than adding a third-receiver overload.

### Use only a list API

A singleton list can express every wait, but requires constructing a list and
unwrapping an index for the common one-receiver case. It also hides the direct
single-channel contract behind selection policy. Keep a direct `recv` family
and require observable parity rather than forcing one public shape.

### Use one options record for every wait

A shared timeout-and-token record could reduce the API to two functions.
With the current coarse effects, a token-capable entry point would require
`time` even for calls that supply no token, unless the language gained
value-dependent effect inference. The recommendation uses paired wait modes
to preserve the existing effect boundary without a new effect mechanism.

### Preserve lossier wrappers and implicit policies

[Veln PR #382](https://github.com/oakcask/veln/pull/382) added receiver-list
timeout selection with explicit interrupt reporting. Its inspected PR body
supports a historical need for a result boundary, not a reason to retain two
parallel public families indefinitely. The recommendation keeps reporting in
every operation. Current behavior was checked against the effects specification
and runtime, rather than inferred from that PR.

[Veln PR #393](https://github.com/oakcask/veln/pull/393) added cancellable list
selection with `time` and `concurrency` effects. Its PR body identifies
cancellation versus ready-value ordering as a risk. The recommendation retains
those effects and makes ordering and no-consumption outcomes explicit for both
cardinalities. Neither PR establishes a reason for two-receiver-only calls.

### Crossbeam-channel comparison

The inspected official API documentation for
[Crossbeam-channel 0.5.17 Receiver](https://docs.rs/crossbeam-channel/0.5.17/crossbeam_channel/struct.Receiver.html)
provides direct receive, immediate receive, and timed receive operations.
This supports keeping single-receiver waiting as an independent usable API.
Its timeout errors distinguish disconnection; Veln retains its existing
combined `None` outcome in this proposal, so its result convention is not
copied wholesale.

[Crossbeam-channel 0.5.17 Select](https://docs.rs/crossbeam-channel/0.5.17/crossbeam_channel/struct.Select.html)
accepts a dynamically built operation set. Its ordinary selection chooses
randomly among ready operations, while `new_biased` prefers the lowest index.
This supports separating selection policy from cardinality. Veln does not
adopt a mutable operation builder, heterogeneous send/receive selection, or
a randomness guarantee. Crossbeam also considers disconnected operations
ready; Veln instead skips drained closed channels until all inputs are drained.
These differences limit direct transfer of its tests and semantics.

## Planned acceptance evidence

The table is an acceptance model for future implementation. No row is reported
as passing by this documentation change. Standard-library behavior belongs in
companion tests under `crates/veln-stdlib/veln/`, according to the
[test placement policy](../reference/toolchain-test-harness.md#test-placement-policy).
Use independently prepared input and expected outcomes, not one new helper as
the sole oracle for another.

| Case | Input or controlled event | Required observation | Intended evidence |
| --- | --- | --- | --- |
| Single and singleton parity | Equivalent ready, drained, timeout, cancelled, and interrupted states | Same result after projecting index zero; same remaining FIFO values | Stdlib companions; JVM interruption contract where Veln cannot inject it |
| Cardinality | One, two, and a dynamically built list | Same generic signatures and index contract; no arity-specific branch in the public API | Stdlib companions; checker type tests |
| Empty input | Empty list with indefinite wait, timeout zero, and cancelled token | `EmptyReceivers`, no wait or channel-state change | Stdlib companions |
| Closed input | Closed buffered receiver before an open ready receiver | Drain buffered values; skip drained entries; preserve original indexes | Stdlib companions |
| All closed | Every input closed and drained | Immediate `Ok(None)` | Stdlib companions |
| Immediate attempt | Timeout zero with ready and empty-open channels | Receive ready value; otherwise `Ok(None)` without suspension | Stdlib companions |
| Timeout boundary | Positive timeout expires; negative timeout later receives | No consumption on expiry; negative timeout stays pending until value or closure | Scoped Veln fake wait support; bounded real-host integration |
| Already cancelled | Cancelled token plus queued value or closed input | `Cancelled`; queued value remains readable | Stdlib companions with separate Veln support |
| Cancellation during wait | Fake observes cancellation before receive commitment | `Cancelled`; values and losing sends are preserved | Scoped Veln fake wait support |
| Receive wins a race | Receive commits before cancellation observation | Exactly one returned value; no rollback or second consumption | JVM synchronization harness for internal interleavings |
| Policy | All inputs prefilled, with duplicates and drained entries | `Priority` returns lowest ready original index; `Unspecified` returns a valid ready index | Stdlib companions |
| Rendezvous | Several pending senders; one receive, timeout, or cancellation | At most one transfer; losing senders remain pending until independently received or closed | Stdlib task/channel coordination; JVM commitment harness |
| Task cancellation | Cancel a blocked task with cleanup | Task cancellation unwinds; cleanup runs; ordinary error matching cannot resume it | Existing task-lifetime and cleanup integration harness |
| Types and effects | Wrong list element, policy, timeout, token, or missing effect | Rejection at the relevant argument or missing effect; generic element type preserved | Checker tests and narrowly scoped human/JSON diagnostics cases |
| API removal | References to old two-argument and `select_many_*` forms | No stale descriptor, runtime mapping, tooling completion, or documentation surface | Symbol audit and checker/tooling tests |

Deterministic tests must use separate scoped Veln fake handlers. Production
waits continue to use real host effects. The current injected clock can control
token observations in `waitCancelled`, but the selection timeout path itself
uses `System.nanoTime` and sleep. A fakeable wait boundary therefore needs
feasibility evidence before Ready promotion; a clock fake alone does not prove
deterministic timeout behavior. Apply the effect-handler maintenance workflow
to that implementation. Do not introduce environment selectors or use fragile
elapsed-time assertions as a substitute for controlled schedules.

## Migration and implementation boundaries

- Replace the old `recv` signature and two-receiver `select*` forms. Convert
  pairs to ordinary receiver lists and choose a policy explicitly.
- Replace `select_many_*` with the corresponding list-taking `select*` forms.
  Preserve existing list priority by selecting `Priority`.
- Consolidate `_result` and non-result variants. Update callers to handle
  `ReceiveError`; do not silently discard errors during migration.
- Map singleton list callers to direct `recv*` where their index is unused.
  Callers that require the list/index shape may keep a singleton selection.
- Review former rotating two-receiver callers individually. `Unspecified`
  permits rotating behavior but weakens their exact tie-order contract.
- Update compiler-known symbols, generic signatures, effect metadata, JVM
  mappings, source-less navigation, API documentation, and affected callers
  together. Do not keep compatibility aliases; the project is experimental.
- Inspect timeout probes in observation shutdown and provider-state restoration
  code. Their immediate-attempt and no-consumption behavior must survive
  migration. Check pure stream-routing handlers remain separate from waits.

This proposal does not change channel construction, MPSC ownership, sender
cloning, channel closure, FIFO order, task workers, or sending APIs. It adds
no heterogeneous selection, combined send/receive cases, general IO readiness,
absolute deadlines, fairness guarantee, or latency target. The current polling
algorithm is implementation evidence, not a required future algorithm.

Before implementation, settle these Ready gates:

1. Approve the result-only family and the deliberate closure/timeout ambiguity.
   If those choices change, revise the signatures and acceptance table together.
2. Approve explicit `Priority` and `Unspecified` policies, including removal of
   the exact global alternation promise, or specify a caller-owned alternative.
3. Establish how channel-owned public error and policy types are declared,
   imported, documented, and resolved alongside compiler-known channel calls.
4. Demonstrate deterministic timeout/cancellation support and rendezvous losing
   transfer preservation without changing production behavior through test flags.
5. Fix the migration scope through an audit of symbol descriptors, callers,
   tooling, specifications, and tests. A new API must not leave an old callable
   path with different effects or results.

During implementation, run focused checker and JVM tests through
`bash scripts/agent-test`, and stdlib companions through
`bash scripts/agent-stdlib-test`. Run host integration only for independently
identified backend, cancellation, or command contracts. Update the concurrency
specification and executable evidence when behavior lands; keep this proposal
out of current-behavior documentation until then. Remove the completed proposal
and its catalog entry once all acceptance cases are covered and the current
specification owns the implemented contract.
