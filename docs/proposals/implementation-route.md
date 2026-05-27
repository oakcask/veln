# Proposal Implementation Route

Use this page after choosing to turn proposal text into implemented behavior.
Start from [README.md](README.md) when you only need to find the relevant
proposal. Use [implementation-route-full.md](implementation-route-full.md) only
for detailed comparison, promotion, or queue cleanup.

## Choose A Target

- Choose from [target-queue.md](target-queue.md) before opening broader
  design-wall material.
- If [target-queue.md](target-queue.md) lists no accepted targets, there is no
  selected proposal completion condition. Do not implement or promote
  design-wall material until the queue selects a target.
- Remaining first-slice implementation targets are tracked in
  [first-slice-follow-ups.md](first-slice-follow-ups.md), with full detail in
  [first-slice-follow-ups-full.md](first-slice-follow-ups-full.md).

## Compare And Promote

- Compare the target with current behavior in
  [../reference/language/README.md](../reference/language/README.md) so the
  implementation changes only the missing behavior.
- Keep the comparison scoped to the chosen target. Do not use nearby design-wall
  text as requirements unless the queue or short proposal page points to it.
- Use [../reference/language/topic-map.md](../reference/language/topic-map.md)
  to choose the smallest reference page to update after the code changes.
- Use [implementation-route-full.md](implementation-route-full.md) when the
  target requires gap evidence, queue updates, or promotion cleanup.

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

## Skip Unless Needed

- Do not read design-wall material before the accepted target queue fails to
  route the task.
- Do not treat proposal text as implemented behavior unless
  `../reference/language/` also states it.
