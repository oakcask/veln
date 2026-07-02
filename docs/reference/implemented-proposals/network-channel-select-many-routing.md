# Network Channel Select-Many Routing

Status: implemented

This record preserves the completed receiver-list channel-first stream routing
slice from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, and the checked examples under
`../../../examples/specification/run/channel-first-stream-routing-general-list/case.toml`
and `../../../examples/specification/run/channel-select-many-timeout/`
and
`../../../examples/specification/run/channel-select-many-timeout-cancellable/`
and
`../../../examples/specification/run/channel-select-many-timeout-cancellable-forced-cancel/`
and
`../../../examples/specification/run/stream-adapter-cancellable-channel-first-routing/`
and
`../../../examples/specification/check/channel-first-stream-routing-general-list-effects/case.toml`
and
`../../../examples/specification/check/channel-select-many-timeout-effects/`
and
`../../../examples/specification/check/channel-select-many-timeout-cancellable-effects/`
and
`../../../examples/specification/check/stream-adapter-cancellable-channel-first-routing-effects/`.
The general-list examples are the primary scalable evidence for receiver-list
routing beyond the canonical two-, three-, and four-route fixtures.

## Outcome

The completed general receiver-list routing slice adds a source-visible helper
shape that accepts a non-empty `List<Receiver<StreamInput>>`, calls
`channel::select_many_priority`, and returns the selected route index plus
ordinary `StreamInput` value. The executable example uses more than four
routes and checks that repeated selection preserves lower-index priority and
the selected value.

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
smaller routing examples. The cancellable channel-first adapter example uses
the receiver-list helper directly rather than adding another fixed route-count
fixture, and maps routed, timed-out, and cancelled helper results into
ordinary adapter action values.

The checked effect examples keep the adapter boundary explicit: source that
owns channel routing declares `concurrency`, and source that owns cancellable
channel routing declares both `time` and `concurrency`, while the handler
receives only ordinary stream input and state values and remains effect-free.
Missing effects on the adapter path are rejected by static checking.

## Remaining Work

The broader network integration proposal remains open for production socket
ownership, richer stream routing beyond the checked general receiver-list
helper, richer deadline and cancellation APIs, channel and task ownership
beyond the checked adapter slices, and HTTP/2 transport-adapter work.

## Read When

- Auditing why receiver-list stream routing is no longer active proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current channel, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
