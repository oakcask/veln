# Network Stream Shutdown Read Boundary

Status: implemented

This record preserves the completed adapter-owned read-side half-close slice
from [external production socket runtime record](network-effect-integration-boundary.md).
Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-production-shutdown-read-lifecycle/`
and
`../../../examples/specification/check/socket-stream-shutdown-read-effects/`.

## Outcome

The completed slice adds `net::shutdown_read(stream)` as a source-visible
standard-library operation over an adapter-owned `NetStream`. The call uses
the existing coarse `net` effect label and returns `()`.

Read-side shutdown is narrower than full cleanup. After the adapter calls
`net::shutdown_read`, optional stream reads observe clean end through the
existing `net::read_chunk_or_end` path. The same stream still owns its write
side, so adapter code can write response bytes, shut down the write side, and
then close the stream explicitly. Transport shutdown failures remain runtime
failures, not pure protocol diagnostics.

The production-loopback lifecycle case accepts a stream, shuts down its read
side, observes clean read end, writes one response chunk, shuts down the write
side, and closes the stream. The effect-checking case keeps the operation
under the existing `net` effect.

## Read When

- Auditing why read-side half-close is no longer active proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
