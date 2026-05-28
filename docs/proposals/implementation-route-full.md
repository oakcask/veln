# Proposal Implementation Route Full

Use this page after [implementation-route.md](implementation-route.md) when an
active proposal needs detailed comparison or an implemented proposal needs
promotion cleanup. Start from [target-selection.md](target-selection.md) when
the task has not already named one concrete target. This page does not define
current behavior.

## Choose A Target

- Start with [target-selection.md](target-selection.md) when the task has not
  already named one concrete short proposal page.
- Continue here only for an active short proposal page whose behavior is absent
  from `../specification/`.
- Keep the target to one short proposal page unless that page routes to a full
  detail record or companion proposal.
- Let [target-selection.md](target-selection.md) handle no-target states,
  implemented records, broad follow-up indexes, and exploratory inventories
  before implementation work starts.

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

## Implemented Proposal Records

- Implemented formatter stabilization:
  [formatter-stabilization.md](formatter-stabilization.md). Use
  [../specification/commands.md](../specification/commands.md) for current
  `veln fmt` behavior and
  [../reviews/formatter-stabilization-completion.md](../reviews/formatter-stabilization-completion.md)
  for completion evidence.
- Completed confirmation and explicit override behavior around `veln repair`:
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md).
  Keep current behavior anchored in
  [../specification/repair-candidates.md](../specification/repair-candidates.md).
- Implemented JVM bytecode backend:
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md). That short page routes
  current behavior, fixture organization, completion evidence, the Java source
  backend cleanup result, and original gates. Keep bytecode layout, generated
  artifacts, backend selectors, and structural test details out of
  `../specification/`.

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
- Do not infer an active target from an implemented record or no-target state.
- Do not keep implemented behavior in proposals merely to mark implementation
  status; keep current behavior in `../specification/` and leave only
  remaining proposal work in short route pages.
