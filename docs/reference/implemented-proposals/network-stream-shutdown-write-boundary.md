# Network Stream Shutdown Write Boundary

Status: implemented

This record preserves the completed adapter-owned write-side half-close slice
from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-shutdown-write-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-production-shutdown-write-lifecycle/`,
`../../../examples/specification/run/socket-stream-adapter-shutdown-write-failure-json/`,
and
`../../../examples/specification/check/socket-stream-shutdown-write-effects/`.

## Outcome

The completed slice adds `net::shutdown_write(stream)` as a source-visible
standard-library operation over an adapter-owned `NetStream`. The call uses the
existing coarse `net` effect label, returns `()`, and records a write-side
shutdown event through the same runtime event log used by stream writes and
stream close.

Write-side shutdown is narrower than full cleanup. After the adapter calls
`net::shutdown_write`, later writes on the same stream fail as runtime
transport failures. The read side still reports clean end through the existing
`net::read_chunk_or_end` path. `net::close_stream(stream)` remains the full
resource cleanup operation.

The fixture lifecycle case accepts a stream, reads one chunk, writes one
response chunk, shuts down the write side, observes clean read end through
`net::read_chunk_or_end`, and then closes the stream. The production-loopback
case records the same boundary through production event-log entries and
captures the client-observed bytes after write-side shutdown. The failure case
checks that a write attempted after write-side shutdown remains a runtime
transport failure, not an HTTP/2 protocol diagnostic.

## Remaining Work

The broader network integration proposal remains open for richer production
socket ownership, richer stream routing, richer deadline and cancellation APIs,
channel and task ownership beyond the checked adapter slices, and HTTP/2
transport-adapter work.

## Read When

- Auditing why write-side half-close is no longer active proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
