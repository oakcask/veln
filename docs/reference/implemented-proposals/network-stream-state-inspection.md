# Network Stream State Inspection

Status: implemented

This record preserves the completed source-visible stream state inspection
slice from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under:

- `../../../examples/specification/run/transport-socket-stream-state-inspection/`
- `../../../examples/specification/run/transport-socket-stream-state-stale-write-json/`
- `../../../examples/specification/check/transport-socket-stream-state-effects/`

## Outcome

The completed slice adds `net::stream_can_read(stream)`,
`net::stream_can_write(stream)`, and `net::stream_is_closed(stream)` as
source-visible standard-library operations over an owned `NetStream`. Each
call uses the existing coarse `net` effect label, returns `Bool`, and preserves
stream ownership for later reads, writes, shutdown, or close.

The production-loopback executable case accepts a stream, observes that it can
read and write while open, observes that read-side shutdown clears read
availability while preserving write ownership, observes that write-side
shutdown clears write availability, and then observes full close on the same
handle. The case also writes one response chunk between the state checks to
pin that inspection does not consume ownership.

The effect check keeps the calls under the existing `net` effect and adds no
new effect label, transport permission split, or general production networking
runtime.

The stale-handle case observes `net::stream_is_closed(stream)` after full
close and then confirms that a later write through the same stale
`NetStream` fails with the existing runtime transport diagnostic style.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, richer stream routing, channel and task ownership beyond the
checked adapter slices, and HTTP/2 transport-adapter work.

## Read When

- Auditing why stream state inspection is no longer active proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
