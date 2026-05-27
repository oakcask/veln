# Proposal Implementation Route Full

Use this page after [implementation-route.md](implementation-route.md) when a
proposal target needs detailed comparison, reference promotion, or cleanup.
This page routes planned work; it does not define current behavior.

## Choose A Target

- Choose any short proposal page in this directory when the task explicitly
  selects proposal work and the behavior is absent from
  `../specification/`.
- Use design-wall material only when a short proposal page routes the task
  there.

## Compare And Promote

- Compare the target with current behavior in
  [../specification/README.md](../specification/README.md) so the
  implementation changes only the missing behavior.
- Keep the comparison scoped to the chosen target. Do not use nearby design-wall
  text as requirements unless the short proposal page points to it.
- Use [../specification/topic-map.md](../specification/topic-map.md)
  to choose the smallest specification page to update after the code changes.
- Use [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md)
  for evidence about known gaps before treating a proposal as complete.
- After implementation, promote the resulting behavior into
  `../specification/` and keep proposal text only for remaining incomplete
  or historical context.
- Use [../document-status.md](../document-status.md) before promoting,
  superseding, or rejecting proposal text.

## Target-Specific Routes

- JVM bytecode backend:
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md) first, then
  [../specification/execution.md](../specification/execution.md),
  [../specification/commands.md](../specification/commands.md), and
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
  After implementation, promote command-visible setup behavior, runtime
  behavior, and command JSON behavior to the smallest matching specification
  pages. Keep generated artifacts, bytecode layout, helper layout, backend
  selectors, and structural test details in proposal, reference, test, or
  implementation documentation unless a later accepted proposal makes them
  user-facing behavior.

## Specification Update Routes

- Source syntax, tests, doctests, names, types, and effects:
  [../specification/topic-map.md#source-surface](../specification/topic-map.md#source-surface).
- Repair candidates, satisfy constraints, and hole diagnostics:
  [../specification/holes.md](../specification/holes.md) and
  [../specification/diagnostics-json.md](../specification/diagnostics-json.md).
- Contract predicate validation, static obligation classification, and result
  bindings: [../specification/contracts.md](../specification/contracts.md).
- Effect propagation or compiler-known calls:
  [../specification/names-effects.md](../specification/names-effects.md).
- Command-specific machine-readable output:
  [../specification/json-output.md](../specification/json-output.md)
  before the command-specific JSON page.

## Exit Checklist

- The changed behavior is documented under `../specification/`.
- Remaining proposal text still describes only absent or incomplete behavior.
- The proposal index still routes remaining proposal work that is not fully
  implemented.
- Links from [README.md](README.md) still route to live proposal pages.

## Skip Unless Needed

- Do not read broad design-wall material before the chosen short proposal page
  routes the task there.
- Do not keep implemented behavior in proposals merely to mark implementation
  status; keep current behavior in `../specification/` and leave only
  remaining proposal work in short route pages.
