# Network Write Until Cancellable Boundary

Status: implemented

This record preserves the completed cancellable deadline-aware stream-write
slice from [external production socket runtime record](network-effect-integration-boundary.md).
Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-socket-write-until-cancellable-boundary/`,
`../../../examples/specification/run/transport-socket-write-until-cancellable-deadline/`,
`../../../examples/specification/run/transport-socket-write-until-cancellable-cancelled/`,
`../../../examples/specification/run/transport-socket-write-until-cancellable-production-outcomes/`,
`../../../examples/specification/run/transport-socket-write-until-cancellable-failure-json/`,
and
`../../../examples/specification/check/transport-socket-write-until-cancellable-effects/`.

## Outcome

The completed slice adds
`net::write_chunk_until_cancellable(stream, chunk, deadline, token)` as a
source-visible standard-library operation over an adapter-owned `NetStream`,
immutable `ByteChunk`, source-visible `Deadline`, and source-visible
`CancelToken`. The call uses the existing coarse `net` and `time` effect
labels and returns `StreamWriteOutcome`.

Fixture-backed examples check `WriteCompleted` for a write before the
deadline and before cancellation, `WriteDeadlineExpired` for
fixture-reported write deadline expiry, and `WriteCancelled` for token
cancellation. The production-loopback example checks the same three outcomes
through owned host streams. The focused failure case keeps forced host write
failure as a runtime transport failure instead of returning an outcome value.
The effect check keeps ownership explicit: source that performs the
cancellable deadline-aware write must declare both `net` and `time`.

The boundary does not add an effect label, change `Deadline`, `CancelToken`,
or existing accept/read outcomes, add buffering or flow-control ownership, or
turn host write failures into ordinary source values.

## Read When

- Auditing why cancellable deadline-aware stream writes are no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
