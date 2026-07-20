# Network Adapter Accept-Loop Helper

Status: implemented

This record preserves the completed source-visible adapter accept-loop helper
slice from [external production socket runtime record](network-effect-integration-boundary.md).
Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-accept-loop/case.toml`,
`../../../examples/specification/check/socket-stream-adapter-accept-loop-effects/case.toml`,
`../../../examples/specification/run/socket-stream-adapter-accept-loop-accept-failure-json/case.toml`,
`../../../examples/specification/run/socket-stream-adapter-accept-loop-read-failure-json/case.toml`,
`../../../examples/specification/run/socket-stream-adapter-accept-loop-write-failure-json/case.toml`,
`../../../examples/specification/run/socket-stream-adapter-accept-loop-stream-close-failure-json/case.toml`,
and
`../../../examples/specification/run/socket-stream-adapter-accept-loop-listener-close-failure-json/case.toml`.

## Outcome

The completed slice adds `stream_adapter_accept_loop(listener, handler)`. The
helper accepts one adapter-owned `NetListener` and the same pure
`fn(StreamInput) -> List<StreamAdapterAction>` handler shape used by
`stream_adapter_drain_actions`. It repeatedly accepts streams with
`net::accept_or_end`, stops on clean listener end, drains each accepted stream
through `stream_adapter_drain_actions`, writes only ordered `SendBytes` chunks
through `net::write_chunks`, closes each accepted stream, and closes the
listener after clean end.

The helper declares the existing coarse `net` and `concurrency` effects and
does not add new network labels, transport-handle exposure to handlers, a
service interface, middleware, or HTTP/2 protocol-core behavior. The checked
production-loopback case accepts two streams and records accept, read,
write-chunks, stream close, clean listener end, and listener close ordering.
The matching effect case rejects adapter entry points that omit either `net`
or `concurrency` while leaving the handler boundary effect-free.

Helper-specific runtime cases preserve forced accept, read, write, stream
close, and listener close failures as runtime transport failures at the same
host boundaries. They do not reinterpret those failures as protocol,
diagnostic, or handler outcomes.

## Read When

- Auditing why the source-visible adapter accept-loop helper is no longer
  active proposal work.
- Checking completion evidence before changing `stream_adapter_accept_loop`,
  listener-owned adapter lifecycle, or helper-owned transport-failure
  ordering.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
