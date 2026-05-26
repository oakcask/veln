# Documentation Navigation

Use this page after [README.md](README.md) when the first route is not obvious.
Choose one route, stop when the linked short page answers the question, and use
[navigation-full.md](navigation-full.md) only when these routes are not enough.

## Read First

- Current behavior: [reference/language/README.md](reference/language/README.md).
- Planned or accepted targets: [proposals/target-queue.md](proposals/target-queue.md).
- Rationale routes: [reference/language/source-decisions.md](reference/language/source-decisions.md).
- Document movement rules: [document-status.md](document-status.md).

## Choose One Route

### Current Behavior

- Syntax, types, effects, contracts, holes, commands, JSON, runtime behavior,
  or examples:
  [reference/language/topic-map.md](reference/language/topic-map.md).
- Human diagnostics, related notes, or stable diagnostic details:
  [reference/language/diagnostics-json.md](reference/language/diagnostics-json.md).
- Command JSON output after the diagnostic route is not enough:
  [reference/language/json-output.md](reference/language/json-output.md).

### Planned Work

- Proposal implementation or promotion:
  [proposals/implementation-route.md](proposals/implementation-route.md).

### History And Evidence

- Gap evidence or completion claims: [reviews/README.md](reviews/README.md).
- Historical implementation order: [phases/README.md](phases/README.md).
- Source support for claims:
  [reference/bibliography/README.md](reference/bibliography/README.md).
- Source-decision record storage or placement audits:
  [reference/source-decisions/result-index.md](reference/source-decisions/result-index.md).

### Documentation Maintenance

- Entry-page routing, document movement, or status labels:
  [document-status.md](document-status.md).
- Source-decision record placement:
  [reference/source-decisions/result-index.md](reference/source-decisions/result-index.md).
- Exhaustive source-decision storage:
  [reference/source-decisions/records/README.md](reference/source-decisions/records/README.md).
- Link-health work: start from the page being edited, then verify links across
  `docs/` after the route is updated.

## Boundary Rules

- Current behavior pages under `reference/language/` win over proposal, phase,
  and review wording.
- Proposal text is not current behavior until the language reference also says
  so.
- A `*-full.md` file is a detail record. Open it only through the short page
  that names the relevant section.
- A `result-*.md` source-decision file is a record. Open it through
  [reference/source-decisions/topic-map.md](reference/source-decisions/topic-map.md)
  for task work or
  [reference/source-decisions/result-index.md](reference/source-decisions/result-index.md)
  for record audits.

## Skip Unless Needed

- Do not use this page when one of the top-level README's read-first links
  already matches the task.
- Do not open [navigation-full.md](navigation-full.md) before choosing one short
  route above.
- Do not open `*-full.md` files before a short route page names the relevant
  section.
