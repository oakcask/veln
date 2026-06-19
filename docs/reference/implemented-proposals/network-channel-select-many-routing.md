# Network Channel Select-Many Routing

Status: implemented

This record preserves the completed receiver-list channel-first stream routing
slice from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, and the checked examples under
`../../../examples/specification/run/channel-first-stream-routing-five-route/`
and `../../../examples/specification/run/channel-first-stream-routing-six-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-seven-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-eight-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-nine-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-ten-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-eleven-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-twelve-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-thirteen-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-fourteen-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-fifteen-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-sixteen-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-seventeen-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-eighteen-route/`
and
`../../../examples/specification/run/channel-first-stream-routing-nineteen-route/`
and `../../../examples/specification/run/channel-select-many-timeout/`
and
`../../../examples/specification/run/channel-select-many-timeout-cancellable/`
and
`../../../examples/specification/run/channel-select-many-timeout-cancellable-forced-cancel/`
and
`../../../examples/specification/check/channel-first-stream-routing-five-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-seven-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-eight-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-nine-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-ten-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-eleven-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-twelve-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-thirteen-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-fourteen-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-fifteen-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-sixteen-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-seventeen-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-eighteen-route-effects/`
and
`../../../examples/specification/check/channel-first-stream-routing-nineteen-route-effects/`
and
`../../../examples/specification/check/channel-select-many-timeout-effects/`
and
`../../../examples/specification/check/channel-select-many-timeout-cancellable-effects/`.

## Outcome

The completed route-count slices add checked five-, six-, seven-, eight-,
nine-, ten-, eleven-, twelve-, thirteen-, fourteen-, fifteen-, sixteen-,
seventeen-, eighteen-, and nineteen-route evidence for the narrow
`channel::select_many_priority(receivers)` standard-library boundary over a
non-empty `List<Receiver<T>>`. It returns
`Option<{index: Int, value: T}>`, where `index` is the zero-based receiver
position in the supplied list. When multiple receivers are ready, selection
uses the existing priority rule: the earliest ready receiver in the supplied
list wins.

The completed timeout slice adds
`channel::select_many_timeout(receivers, timeout_ms)` with the same receiver
list, return shape, and priority rule, plus an `Int` millisecond timeout. It
returns `None` when no receiver has a ready value before the timeout. Negative
timeouts wait without a timeout, matching the priority helper.

The completed timeout-result slice adds
`channel::select_many_timeout_result(receivers, timeout_ms)` with the same
receiver list, priority rule, and timeout behavior, plus the existing
fallible selection result boundary. It returns `Ok(Some(selected))` when a
receiver produces a value, `Ok(None)` when selection closes or times out
without a value, and `Err(SelectError)` when cooperative cancellation
interrupts the waiting selection.

The completed cancellable timeout-result slice adds
`channel::select_many_timeout_cancellable(receivers, timeout_ms, token)` with
the same receiver list, priority rule, timeout behavior, and selected result
shape, plus source-visible `CancelToken` observation. It returns
`Err(SelectError)` when the token is already cancelled or becomes cancelled
before a receiver wins.

These helpers use the existing `concurrency` effect, and the cancellable
helper also uses the existing `time` effect. They do not add channel, socket,
task, timer, cancellation, or network-specific effect labels. The checked run
examples route ordinary `StreamInput` values through typed channels, select
them by receiver-list priority, timeout, timeout-result, or cancellable
timeout-result, and only then invoke the same pure handler shape used by the
smaller routing examples.

The checked effect examples keep the adapter boundary explicit: source that
owns channel routing declares `concurrency`, and source that owns cancellable
channel routing declares both `time` and `concurrency`, while the handler
receives only ordinary stream input and state values and remains effect-free.
Missing effects on the adapter path are rejected by static checking.

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
