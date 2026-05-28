# Proposal Implementation Route

Use this page after choosing to turn active proposal text into implemented
behavior, or when an implemented proposal needs promotion cleanup. Start from
[README.md](README.md) when you only need to find the relevant proposal. Use
[implementation-route-full.md](implementation-route-full.md) only for detailed
comparison, promotion, or cleanup.

## Choose A Target

- Choose any active short proposal page in this directory when the task
  explicitly selects proposal work and the behavior is absent from
  `../specification/`.
- For implemented proposal records, start with the matching specification page
  and open the proposal only for history, evidence, or cleanup.
- Keep the target to one short proposal page unless that page routes to a full
  detail record or companion proposal.

## Compare And Promote

- Compare the target with current behavior in
  [../specification/README.md](../specification/README.md) so the
  implementation changes only the missing behavior.
- Keep the comparison scoped to the chosen target. Do not use nearby design-wall
  text as requirements unless the short proposal page points to it.
- Use [../specification/topic-map.md](../specification/topic-map.md)
  to choose the smallest specification page to update after the code changes.
- Use [implementation-route-full.md](implementation-route-full.md) when the
  target requires gap evidence or promotion cleanup.

## Target-Specific Routes

- Completed confirmation and explicit override behavior around `veln repair`:
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md).
  Keep current behavior anchored in
  [../specification/repair-candidates.md](../specification/repair-candidates.md).
  Do not include broader ranking models, external verification commands,
  partial application, or general automatic repair unless a short proposal page
  selects that work.
- Implemented JVM bytecode backend:
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md). That short page routes
  current behavior, fixture organization, completion evidence, the Java source
  backend cleanup result, and original gates. Keep bytecode layout,
  generated artifacts, backend selectors, and structural test details out of
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

## Skip Unless Needed

- Do not read broad design-wall material before the chosen short proposal page
  routes the task there.
- Do not treat proposal text as implemented behavior unless
  `../specification/` also states it.
