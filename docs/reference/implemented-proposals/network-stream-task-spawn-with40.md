# Network Stream Task Spawn With40

Status: implemented

This record preserves the completed forty-argument stream-handler task
slice from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/source-surface.md`,
and the checked examples under
`../../../examples/specification/run/socket-stream-adapter-routing-spawn40/` and
`../../../examples/specification/check/socket-stream-adapter-routing-spawn40-effects/`.

## Outcome

The completed slice adds `task::spawn_with40` as the forty-argument
sibling of the earlier `task::spawn_withN` helpers. It starts a
forty-argument callable under the existing `concurrency` effect, accepts
the same optional return-type argument shape, and returns `Task<T>`.

Executable stream adapter coverage passes ordinary event, state, adapter
context, routing metadata, and thirty-six additional ordinary metadata values
as separate task arguments. The spawned handler receives no `NetStream` or
other transport handle, and the slice adds no `net`, `time`, routing, or
task-specific effect label beyond `concurrency`.

The checked effect case keeps ownership explicit: adapter code that spawns and
joins the task declares `concurrency`, socket-routing adapter code declares
`net`, and the handler boundary remains free of socket ownership.

## Remaining Work

The broader network integration proposal remains open for production socket
ownership, richer stream routing, richer deadline and cancellation APIs, and
HTTP/2 transport-adapter work beyond the checked narrow task boundary.

## Read When

- Auditing why forty-argument stream-handler task spawning is no longer
  active proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current task, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
