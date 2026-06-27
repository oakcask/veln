# Network Monotonic Clock Boundary

Status: implemented

This record preserves the completed source-visible monotonic clock slice from
`../../proposals/network-effect-integration-boundary.md`. Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-monotonic-clock/` and
`../../../examples/specification/check/transport-monotonic-clock-effects/`.

## Outcome

The completed slice adds `time::monotonic_ms() -> Int effects [time]` as a
source-visible standard-library call. It returns a host-owned monotonic
millisecond counter for elapsed-time measurement and uses the existing coarse
`time` effect label.

The executable run case calls `time::monotonic_ms` twice and checks only that
the second observed value does not move backward. The focused effect case
keeps ownership explicit: a public function that reads the monotonic counter
must declare `time`.

The boundary does not add a timer handle, wall-clock timestamp, date, time
zone, sleep API, schema behavior, protocol-core dependency, or separate clock
or timer effect label.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, scheduler integration, and deadline or
cancellation APIs beyond the current monotonic clock and wait boundaries.

## Read When

- Auditing why source-visible monotonic clock reads are no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
