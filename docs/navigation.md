# Documentation Navigation

Use this page after [README.md](README.md) when the first route is not obvious.
Choose one route and stop when the linked short page answers the question. Use
[navigation-full.md](navigation-full.md) only for navigation history or when the
routes below are not enough.

## Read First

- Current behavior: [reference/language/README.md](reference/language/README.md).
- Planned or accepted targets: [proposals/target-queue.md](proposals/target-queue.md).
- Rationale routes: [reference/language/source-decisions.md](reference/language/source-decisions.md).
- Document movement rules: [document-status.md](document-status.md).
- Use [document-status.md](document-status.md) before moving text between
  reference, proposals, reviews, and phases.

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
- Accepted target selection:
  [proposals/target-queue.md](proposals/target-queue.md).
- Open design exploration after the accepted queue is empty:
  [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md).

### History And Evidence

- Gap evidence or completion claims: [reviews/README.md](reviews/README.md).
- Historical implementation order: [phases/README.md](phases/README.md).
- Source support for claims:
  [reference/bibliography/README.md](reference/bibliography/README.md).
- Source-decision rationale or record audits:
  [reference/source-decisions/README.md](reference/source-decisions/README.md).

### Documentation Maintenance

- Entry-page routing, document movement, or status labels:
  [document-status.md](document-status.md).
- Source-decision category routing, record placement, or storage audits:
  [reference/source-decisions/README.md](reference/source-decisions/README.md).
- Link-health work: start from the page being edited, then verify links across
  `docs/` after the route is updated.

## Boundary Rules

- Current behavior pages under `reference/language/` win over proposal, phase,
  and review wording.
- Proposal text is not current behavior until the language reference also says
  so.
- Promotion work updates the smallest matching language reference page and
  leaves unfinished proposal text in `proposals/`.
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
