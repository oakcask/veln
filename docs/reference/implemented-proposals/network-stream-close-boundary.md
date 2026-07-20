# Network Stream Close Boundary

Status: implemented

This record preserves the completed adapter-owned stream close lifecycle slice
from [external production socket runtime record](network-effect-integration-boundary.md).
Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-close-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-cancel-close-lifecycle/`,
and
`../../../examples/specification/check/socket-stream-close-effects/`.

## Outcome

The completed slice adds `net::close_stream(stream)` as a source-visible
standard-library operation over an adapter-owned `NetStream`. The call uses the
existing coarse `net` effect label, returns `()`, and records a fixture-backed
close event through the same runtime event log used by stream writes. It does
not add production socket behavior, half-close semantics, shutdown modes, or a
new effect label.

The clean-end executable case accepts a stream, repeatedly reads with
`net::read_chunk_or_end` until clean end, translates ordinary stream inputs
through a channel, applies ordered `SendBytes` response actions with
`net::write_chunk`, and then calls `net::close_stream`. The fixture log pins
the close event after the final write.

The cancellation cleanup executable case treats `WaitCancelled` as an ordinary
adapter routing outcome, produces a cleanup response action, applies ordered
`SendBytes` writes, and then records the close event. Cancellation does not
become a runtime failure on this value-returning wait path.

The effect check keeps ownership explicit: source that closes a `NetStream`
must declare `net`, and plain handler functions still receive ordinary event,
state, and action values without socket handles.

## Read When

- Auditing why explicit adapter-owned stream close is no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
