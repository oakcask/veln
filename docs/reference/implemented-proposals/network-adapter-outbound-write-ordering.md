# Network Adapter Outbound Write Ordering

Status: implemented

This record preserves the completed adapter-owned outbound write-ordering
slice from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/socket-stream-adapter-write-chunks-ordering/`
and
`../../../examples/specification/run/socket-stream-adapter-write-chunks-failure-json/`,
with human transport-failure output pinned by
`../../../examples/specification/run/socket-stream-adapter-write-chunks-failure-human/`,
and
`../../../examples/specification/check/socket-stream-adapter-write-chunks-ordering-effects/`.

## Outcome

The completed slice keeps protocol handlers ordinary and transport-free while
making outbound transport ordering an adapter-owned responsibility. The
executable run case accepts deterministic production-loopback streams, reads
ordinary `StreamInput` values through adapter-owned socket calls, routes those
values through the existing channel boundary, calls multiple pure handler
functions, and combines their ordinary `ResponseAction` values into one
explicit outbound order.

Only `SendBytes` actions are projected into a `List<ByteChunk>`, and the
adapter writes that list with `net::write_chunks`. Non-write actions remain
ordinary response intent values for adapter code to interpret. The checked
case verifies the ordered production write log and captured client bytes for
two accepted streams. The failure cases force the same adapter-owned
`net::write_chunks` path after production accept, read, channel routing, and
response projection, keeping the write failure as a runtime transport failure
without recording response writes or stream close.

The effect check keeps ownership explicit: handler functions remain free of
transport, time, and concurrency effects, while the adapter boundary that
reads socket input, routes through a channel, and writes ordered chunks must
declare the existing coarse `net` and `concurrency` effects. The slice adds no
effect labels, no socket primitive, and no service framework.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, richer stream routing, richer deadline and cancellation APIs,
channel and task ownership beyond the checked adapter slices, and HTTP/2
transport-adapter work.

## Read When

- Auditing why adapter-owned multi-handler outbound write ordering is no
  longer active proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
