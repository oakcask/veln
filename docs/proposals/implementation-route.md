# Proposal Implementation Route

Use this page after choosing a proposal page whose behavior is absent from
`../specification/`. This page routes implementation and promotion mechanics;
it does not override the current language specification. Use
[implementation-route-full.md](implementation-route-full.md) only for detailed
comparison, promotion, or cleanup.

## Entry Check

- Start from the proposal page named by the task or from
  [README.md](README.md).
- Stop when the proposal page is implemented, closed, superseded, rejected, or
  already covered by `../specification/`.
- Keep the implementation scope to the chosen proposal page unless that page
  routes to a full detail record or companion proposal.

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

## Exit Check

- The changed behavior is documented under `../specification/` only after code
  and tests support it.
- Remaining proposal text still describes only absent or incomplete behavior.
- Proposal indexes route remaining work without restating current behavior.

## Skip Unless Needed

- Do not read broad design-wall material before the chosen short proposal page
  routes the task there.
- Do not infer current behavior from proposal text; return to
  `../specification/` for implemented behavior.
- Do not treat proposal text as implemented behavior unless
  `../specification/` also states it.
