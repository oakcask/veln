# Network Channel Select-Many Routing

Status: implemented

This record preserves the completed receiver-list channel-first stream routing
slice from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, and the checked examples under
`../../../examples/specification/run/channel-first-stream-routing-five-route/`
and `../../../examples/specification/run/channel-first-stream-routing-six-route/`
and
`../../../examples/specification/check/channel-first-stream-routing-five-route-effects/`.

## Outcome

The completed slice adds the narrow `channel::select_many_priority(receivers)`
standard-library boundary for a non-empty `List<Receiver<T>>`. It returns
`Option<{index: Int, value: T}>`, where `index` is the zero-based receiver
position in the supplied list. When multiple receivers are ready, selection
uses the existing priority rule: the earliest ready receiver in the supplied
list wins.

The helper uses only the existing `concurrency` effect. It does not add channel,
socket, task, timer, or network-specific effect labels. The checked run
examples route five and six ordinary `StreamInput` values through typed
channels, select them by receiver-list priority, and only then invoke the same
pure handler shape used by the smaller routing examples.

The checked effect example keeps the adapter boundary explicit: source that
owns channel routing declares `concurrency`, while the handler receives only
ordinary stream input and state values and remains effect-free. Missing
`concurrency` on the adapter path is rejected by static checking.

## Remaining Work

The broader network integration proposal remains open for production socket
ownership, richer stream routing beyond the checked narrow route counts, richer
deadline and cancellation APIs, channel and task ownership beyond the checked
adapter slices, and HTTP/2 transport-adapter work.

## Read When

- Auditing why receiver-list stream routing is no longer active proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current channel, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
