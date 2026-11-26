---
role: proposal
update-when: The proposed time::wait_until_cancellable result cases, deadline completion semantics, interruption behavior, or outcome-wrapper retirement changes.
---

# Return Cancellable Wait Outcomes Directly

Make `time::wait_until_cancellable` return its ordinary outcome and remove the
parallel `_outcome` entry point. This is a Ready design independent of ordinary
generic functions. Its fixed outcome ADT already fits the language.

## Current boundary and inconsistency

The [time boundary](../specification/effects.md#network-and-time-boundary-calls)
exposes a unit-returning wait plus an outcome-returning wait. The
[JVM wrapper](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-backend-jvm/src/runtime/effects.java.inc)
turns `Cancelled` and injected `DeadlineExpired` outcomes into runtime failures.
The host loop reports normal deadline arrival as `Completed`; an injected
clock can instead report `DeadlineExpired`. Host interruption also currently
sets the token's cancellation state. A caller must choose an entry point to
recover ordinary control flow, and cancellation can conceal interruption.

A timer wait reaching its target is successful timer completion. It is not
an operation that failed to finish before that target. This proposal makes
that distinction explicit rather than copying IO timeout names into a timer.

## Proposed contract

```veln
time::wait_until_cancellable(deadline: Deadline, token: CancelToken) -> CancellableWaitOutcome effects [time]
```

The outcome has three public cases: `Completed`, `Cancelled`, and
`Interrupted`. Remove `DeadlineExpired` from this timer outcome. Both ordinary
host target arrival and injected deadline expiry map to `Completed`.
Do not change network or stream timeout outcomes in which deadline expiry is
an operation failure. Keep the existing non-cancellable `wait_until` outside
this proposal.

| State or event | Required outcome and state |
| --- | --- |
| Token already cancelled, including with a past deadline | `Cancelled`; no suspension. |
| Active token and deadline already reached | `Completed`; no suspension. |
| Target reached during the wait | `Completed`. |
| Cancellation observed before completion | `Cancelled`. |
| Host interruption without task cancellation | `Interrupted`; retain interrupt state and do not cancel the token. |
| Task cancellation | Unwind through task cleanup; do not return an ordinary wait outcome. |

Racing completion and cancellation may choose either event before the wait
commits its outcome. One wait returns at most one outcome. Clock-provider failure
retains its existing abrupt boundary; it is not relabeled as cancellation.

## Related work and alternatives

The inspected [Veln time runtime](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-backend-jvm/src/runtime/effects.java.inc)
shows the unit wrapper delegates to the outcome implementation, so the split
is not required by the host algorithm. The public ADT is already in the
[prelude](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/prelude.veln). The design consequence is
to retain one reporting path rather than two error-handling conventions.

The inspected [Rust Result API](https://doc.rust-lang.org/std/result/enum.Result.html)
illustrates typed success/failure cases instead of string discrimination.
Veln needs a dedicated three-case outcome rather than copying its two-case
shape. Renaming the wrapper to advertise throwing behavior would preserve
avoidable public duplication. Returning unit for cancellation would erase
information callers already need. Neither alternative is selected.

## Planned acceptance

| Case | Required observation | Intended verification |
| --- | --- | --- |
| Past deadline, ready cancellation, and ordinary target arrival | Match the table, including cancellation precedence for already-cancelled inputs. | Stdlib time companions with scoped Veln clock support |
| Injected complete and expiry replies | Both yield timer `Completed`, with no runtime-failure wrapper. | Clock-boundary companions |
| Host interruption | Distinct `Interrupted`; independent observers still see an active token. | Bounded JVM host-integration fixture |
| Task cancellation with defer | Task remains cancelled and cleanup runs once. | Task/cleanup integration harness |
| Removed wrapper and case | No `_outcome` callable path or timer `DeadlineExpired` constructor remains. | Checker, docs, navigation, and symbol audit |

No planned acceptance has run for this proposal. Migrate all timer consumers
and injected clock fixtures explicitly; a textual `DeadlineExpired` rename
would incorrectly change IO outcomes. Apply the effect-handler maintenance
workflow and preserve real host defaults. API behavior tests belong in stdlib
companions under the test placement policy.

This changes signatures and cases, not source grammar. Existing call and match
formatting applies; `veln fmt` must not rewrite potentially ignored results or
case meanings. Update the current time/prelude specifications and executable
evidence when implemented, then remove this proposal and its catalog entry.
