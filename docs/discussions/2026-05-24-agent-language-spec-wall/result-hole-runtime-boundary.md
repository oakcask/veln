# Discussion Result: Hole Runtime Boundary

Date: 2026-05-24

## Picked Question

- Should a file containing holes be runnable, checkable only, or runnable only
  when holes are outside executed code?

## Decision

A file containing holes should always be checkable, but `veln run` should only
execute when holes are outside the selected entry point's conservative reachable
code.

Holes are valid partial-program expressions for analysis, not runtime values.
The first slice should reject execution before user code starts if the chosen
entry point may evaluate a hole. A file may still contain holes in unused
functions, unselected modules, or examples that are not part of the run target.

## Rationale

Making every file with a hole checkable preserves the core repair loop: parse
partial code, infer expected types, report contracts and effects where possible,
and guide the next edit. Making every file with a hole non-runnable would make
small experiments awkward because unfinished neighboring code would block an
otherwise complete entry point.

Treating a hole as a runtime placeholder would create a worse failure mode.
The interpreter would need placeholder semantics, callers could accidentally
depend on incomplete behavior, and tests might pass around a value that should
never exist. For an agent-facing language, incompleteness should be explicit in
diagnostics before execution begins.

## First-Slice Rule

- `veln check` accepts holes and reports them through hole diagnostics.
- `veln run <entry>` first performs the same parse, binding, and type work
  needed to identify holes reachable from the selected entry point.
- If a hole is definitely or possibly reachable from the selected entry point,
  `run` exits before executing user code with a `hole.runtime_blocked`
  diagnostic.
- Holes outside the selected entry point's conservative reachable graph do not
  block `run`.
- If reachability is incomplete, ambiguous, or unavailable, classify the hole
  as possibly reachable and block execution.

## Open Detail

The exact reachability model can remain conservative in the first slice. Direct
function calls, selected entry points, and obvious module initializers are
enough to establish the policy. Later effect analysis and dependency graphs can
make the same rule more precise without changing the runtime boundary.

## Consequence

Typed holes stay useful for partial programs while `veln run` keeps a simple
trust contract: executed code has no known incomplete expressions.
