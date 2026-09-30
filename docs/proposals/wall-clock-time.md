---
role: proposal
update-when: Wall-clock time values, the standard time effect, timestamp precision, runtime clock substitution, or the temporary wall-clock test override is implemented, redesigned, or removed.
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
| T7 | Repository-owned runtime tests need exact wall-clock readings before a controlled host-clock seam exists. | Tests may use the temporary `VELN_TIME_WALL_CLOCK` process environment override. The override is not a public Veln or runtime interface. | JVM runtime tests and executable specification cases owned by this repository. |
| T8 | A Veln `time` effect handler supplies controlled wall-clock readings around code that calls `time::wall_time`. | The controlled cases cover the epoch boundary, a pre-epoch instant, equal readings, and backwards-moving readings without consulting the live machine clock. The handled body does not retain the handled `time` effect. | Executable specification cases that install a lexical fake-clock handler. |
| T9 | The replacement evidence passes without the temporary override. | Production runtime code does not read `VELN_TIME_WALL_CLOCK`, repository tests and examples do not set it, and the live-clock path still returns a normalized `WallTime`. | A repository search for the removed name, deterministic runtime cases, and one live-clock smoke test that does not assert an exact timestamp. |

## Temporary Verification Seam

The first implementation may use `VELN_TIME_WALL_CLOCK` to supply controlled
readings to repository-owned JVM tests and executable specification cases. This
is a temporary implementation aid. Its name, input grammar, sequence behavior,
and failure behavior are not compatibility commitments. Current specification
pages and standard-library package documentation must not teach users to set
it.

The production path continues to read the host wall clock when the override is
absent. Tests of that path must check only stable invariants, such as normalized
nanoseconds and representable signed seconds. They must not compare the result
with an exact live-machine timestamp or require successive wall-clock readings
to increase.

Replace the process-wide override with a Veln effect handler for the existing
`time` effect. The handler supplies the fake wall-clock readings to the handled
body. This is the durable verification path for exact values, including
pre-epoch and backwards-moving readings. It also verifies that wall-clock
access remains substitutable through the same effect boundary as the other
time operations.

Keep the system handler test separate. It verifies only that the live host path
returns a normalized `WallTime`; it does not assert an exact timestamp or
ordering between calls. After the effect-handler evidence passes, remove the
environment lookup and migrate every test and executable case that names the
override.

Do not close this proposal while the runtime or repository evidence still
depends on `VELN_TIME_WALL_CLOCK`. If the public wall-clock API lands first,
reduce this page and its catalog entry to the remaining test-seam replacement
and removal work instead of deleting it.

## Verification and Promotion

The exact-value tests need a lexical fake-clock handler; they must not assert a
live machine timestamp. Implementation must add the standard symbol, lowering,
effect metadata, runtime adapter, editor surface, package documentation, and
handler substitution needed by T8. After the public API cases pass, the current
execution and standard-library specification pages become the authority for
wall-clock behavior. This proposal remains open only for the handler-based
replacement and temporary test-seam removal until T8 and T9 also pass.

## Non-goals

- Calendar arithmetic, parsing, formatting, and time zones are separate work.
- The proposal does not change `time::monotonic_ms` or deadline behavior.
- The proposal does not specify synchronization with a network time service.
