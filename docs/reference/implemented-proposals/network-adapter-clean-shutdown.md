# Network Adapter Clean Shutdown

Status: implemented

This record preserves the completed adapter-owned clean shutdown slice from
[external production socket runtime record](network-effect-integration-boundary.md).
Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-clean-shutdown/`
and
`../../../examples/specification/check/socket-stream-adapter-clean-shutdown-effects/`.

## Outcome

The completed slice keeps transport handles owned by adapter code during
shutdown. The executable run case accepts with
`net::accept_until_cancellable`, owns the `NetListener` and accepted
`NetStream`, routes an ordinary `StreamInput` value through the existing
channel boundary, and calls a plain handler that receives only source-visible
event and state values.

The adapter observes cancellation and deadline expiry through
`time::wait_until_cancellable_outcome` and translates those outcomes into
ordinary `ResponseAction` values. Only `SendBytes` actions are projected to
ordered `net::write_chunk` calls. Cleanup then records `net::close_stream`
followed by `net::close_listener`, so cancellation and deadline-expiry
decisions do not emit extra bytes.

The effect check keeps ownership explicit: the clean shutdown adapter boundary
must declare the existing coarse `net`, `time`, and `concurrency` effects,
while the pure handler remains callable without transport, time, or
concurrency effects. The slice adds no effect label, socket primitive, channel
route-count fixture, TLS, ALPN, production polling, or HTTP application
framework.

## Read When

- Auditing why adapter-owned clean shutdown after cancellation and deadline
  expiry is no longer active proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
