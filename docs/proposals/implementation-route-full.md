# Proposal Implementation Route Full

Use this page after [implementation-route.md](implementation-route.md) when a
proposal target needs detailed comparison, reference promotion, or queue
cleanup. This page routes planned work; it does not define current behavior.

## Choose A Target

- Choose from [target-queue.md](target-queue.md) before opening broader
  design-wall material.
- Remaining first-slice implementation targets are tracked in
  [first-slice-follow-ups.md](first-slice-follow-ups.md), with full detail in
  [first-slice-follow-ups-full.md](first-slice-follow-ups-full.md).
- Use design-wall material only when [target-queue.md](target-queue.md) has no
  accepted target for the task.

## Compare And Promote

- Compare the target with current behavior in
  [../reference/language/README.md](../reference/language/README.md) so the
  implementation changes only the missing behavior.
- Keep the comparison scoped to the chosen target. Do not use nearby design-wall
  text as requirements unless the queue or short proposal page points to it.
- Use [../reference/language/topic-map.md](../reference/language/topic-map.md)
  to choose the smallest reference page to update after the code changes.
- Use [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md)
  for evidence about known gaps before treating a proposal as complete.
- After implementation, promote the resulting behavior into
  `../reference/language/` and keep proposal text only for remaining incomplete
  or historical context.
- Use [../document-status.md](../document-status.md) before promoting,
  superseding, or rejecting proposal text.

## Reference Update Routes

- Source syntax, tests, doctests, names, types, and effects:
  [../reference/language/topic-map.md#source-surface](../reference/language/topic-map.md#source-surface).
- Repair candidates, satisfy constraints, and hole diagnostics:
  [../reference/language/holes.md](../reference/language/holes.md) and
  [../reference/language/diagnostics-json.md](../reference/language/diagnostics-json.md).
- Contract predicate validation, static obligation classification, and result
  bindings: [../reference/language/contracts.md](../reference/language/contracts.md).
- Effect propagation or compiler-known calls:
  [../reference/language/names-effects.md](../reference/language/names-effects.md).
- Command-specific machine-readable output:
  [../reference/language/json-output.md](../reference/language/json-output.md)
  before the command-specific JSON page.

## Exit Checklist

- The changed behavior is documented under `../reference/language/`.
- Remaining proposal text still describes only absent or incomplete behavior.
- The target queue still names only accepted work that is not fully implemented.
- Links from [target-queue.md](target-queue.md) still route to a live accepted
  target, or the target has been removed from the accepted queue.

## Skip Unless Needed

- Do not read design-wall material before the accepted target queue fails to
  route the task.
- Do not edit full proposal history merely to mark implementation status; keep
  current behavior in `../reference/language/` and leave remaining proposal work
  in short route pages.
