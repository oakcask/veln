# Network Write Chunks Until Boundary

Status: implemented

This record preserves the completed deadline-aware chunk-list stream-write
slice from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-socket-write-chunks-until-boundary/`,
`../../../examples/specification/run/transport-socket-write-chunks-until-deadline/`,
`../../../examples/specification/run/transport-socket-write-chunks-until-failure-json/`,
and
`../../../examples/specification/check/transport-socket-write-chunks-until-effects/`.

## Outcome

The completed slice adds
`net::write_chunks_until(stream, chunks, deadline)` as a source-visible
standard-library operation over an adapter-owned `NetStream`, a source-owned
`List<ByteChunk>`, and source-visible `Deadline`. The call uses the existing
coarse `net` and `time` effect labels and returns `StreamWriteOutcome`.

The successful executable case writes every chunk in source list order and
returns `WriteCompleted`. The deadline case returns `WriteDeadlineExpired`
before the list is fully written. The focused failure case keeps forced host
write failure as a runtime transport failure instead of returning an outcome
value. The effect check keeps ownership explicit: source that performs the
deadline-aware chunk-list write must declare both `net` and `time`.

The boundary does not add cancellation behavior, add an effect label, change
`Deadline`, add buffering or flow-control ownership, or turn host write
failures into ordinary source values. Cancellation remains owned by
`net::write_chunks_until_cancellable`.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, stream routing ownership, cancellation ownership, scheduler
integration, and HTTP/2 transport-adapter behavior beyond the checked
deadline-aware and cancellable boundary slices.

## Read When

- Auditing why deadline-aware chunk-list stream writes are no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
