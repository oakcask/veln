# Network Write Until Boundary

Status: implemented

This record preserves the completed deadline-aware stream-write slice from
`../../proposals/network-effect-integration-boundary.md`. Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-socket-write-until-boundary/`,
`../../../examples/specification/run/transport-socket-write-until-deadline/`,
`../../../examples/specification/run/transport-socket-write-until-production-outcomes/`,
`../../../examples/specification/run/transport-socket-write-until-failure-json/`,
and `../../../examples/specification/check/transport-socket-write-until-effects/`.

## Outcome

The completed slice adds
`net::write_chunk_until(stream, chunk, deadline)` as a source-visible
standard-library operation over an adapter-owned `NetStream`, immutable
`ByteChunk`, and source-visible `Deadline`. The call uses the existing coarse
`net` and `time` effect labels and returns `StreamWriteOutcome`.

Fixture-backed examples check `WriteCompleted` for a write before the
deadline and `WriteDeadlineExpired` for fixture-reported write deadline
expiry. The production-loopback example checks the same two outcomes through
owned host streams. The focused failure case keeps forced host write failure
as a runtime transport failure instead of returning an outcome value. The
effect check keeps ownership explicit: source that performs the
deadline-aware write must declare both `net` and `time`.

The boundary does not add an effect label, change `Deadline`, add
cancellation behavior, expose socket handles to pure protocol handlers, or
turn host write failures into ordinary source values.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, cancellation ownership, scheduler
integration, and HTTP/2 transport-adapter behavior beyond the checked
deadline-aware and cancellable boundary slices.

## Read When

- Auditing why deadline-aware stream writes are no longer active proposal
  work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
