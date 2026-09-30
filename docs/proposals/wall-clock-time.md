---
role: proposal
update-when: Wall-clock time values, the standard time effect, timestamp precision, or runtime clock substitution is implemented or redesigned.
---

# Wall-clock Time

Veln has a monotonic millisecond clock for deadlines and elapsed time. Logs,
events, and exported spans also need a timestamp that can be correlated with
other processes. A monotonic value cannot provide that timestamp.

## Outcome

Add a standard `WallTime` value and one operation to the existing `time`
effect:

```veln
pub type WallTime = {
  unix_seconds: Int,
  nanosecond: Int,
}

time::wall_time() -> WallTime effects [time]
```

`unix_seconds` counts seconds relative to the Unix epoch in UTC.
`nanosecond` is in the inclusive range `0..999999999`. The pair is normalized,
including for instants before the epoch.

## Clock Contract

The wall clock is suitable for timestamps and cross-process correlation. It is
not suitable for durations, deadlines, timeout ordering, or elapsed-time
measurement. Successive results may be equal or move backwards after host
clock correction. The existing monotonic clock remains the authority for
duration and deadline logic.

Clock resolution is host-defined. Nanosecond representation does not promise
nanosecond resolution.

The operation uses the existing `time` effect so a test handler or host adapter
can supply deterministic values. The public value does not expose a host time
class or a local time zone.

## Acceptance Model

| Case | Input or host state | Required observation | Planned evidence |
| --- | --- | --- | --- |
| T1 | The controlled wall clock is at an epoch boundary. | `wall_time` returns the corresponding normalized seconds and nanoseconds. | Table-driven runtime tests. |
| T2 | The controlled clock represents a pre-epoch instant. | The result remains normalized and reconstructs the same instant. | Table-driven runtime tests. |
| T3 | The host clock moves backwards. | A later call may return an earlier value without a runtime failure. | Deterministic runtime case. |
| T4 | Code uses `wall_time` without declaring or handling `time`. | Effect checking reports the missing `time` effect at the call. | Check and check-JSON cases. |
| T5 | The same source uses wall and monotonic clocks. | Wall time supplies timestamps; monotonic time continues to supply deadline ordering. | Run specification case. |
| T6 | JSON output represents a wall time. | Seconds and nanoseconds are emitted as integer fields without locale-dependent text. | Checked JSON fixture. |

## Verification and Promotion

The runtime tests need a controlled clock adapter; they must not assert a live
machine timestamp. Implementation must add the standard symbol, lowering,
effect metadata, runtime adapter, editor surface, and package documentation.
After the cases pass, the current execution and standard-library specification
pages become the authority.

## Non-goals

- Calendar arithmetic, parsing, formatting, and time zones are separate work.
- The proposal does not change `time::monotonic_ms` or deadline behavior.
- The proposal does not specify synchronization with a network time service.
