# Network Deadline At Boundary

Status: implemented

This record preserves the completed absolute monotonic deadline slice from
[external production socket runtime record](network-effect-integration-boundary.md).
Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-deadline-at-boundary/`,
`../../../examples/specification/run/transport-socket-read-until-deadline-at-expired/`,
and `../../../examples/specification/check/transport-deadline-at-effects/`.

## Outcome

The completed slice adds
`time::deadline_at_ms(target_ms) -> Deadline effects [time]` as a
source-visible standard-library call. It constructs the same `Deadline` value
shape as `time::deadline_after_ms`, but its argument is an absolute monotonic
millisecond target in the same clock domain observed through
`time::monotonic_ms`.

The executable run cases check both source-visible outcomes required for the
boundary. A future absolute target flows through the normal deadline-aware
socket read path and returns the same successful read shape as a relative
deadline. A target at the current monotonic tick is already expired for the
same existing `net::read_chunk_until` consumer and returns `None`. The focused
effect case keeps ownership explicit: source that constructs and waits on an
absolute deadline must declare `time`.

The boundary does not add wall-clock time, calendar conversion, time zones,
sleep handles, a new effect label, a new `Deadline` source-visible value
shape, or separate consumer paths for relative and absolute deadline
construction.

## Read When

- Auditing why absolute monotonic deadline construction is no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
