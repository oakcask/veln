# Documentation Navigation Full

Use this page after [navigation.md](navigation.md) when the short route is not
enough. Choose one route and stop when the linked short page answers the
question.

## Current Behavior

- Changing syntax, types, effects, contracts, holes, commands, JSON output,
  runtime behavior, or examples:
  [specification/topic-map.md](specification/topic-map.md).
- Checking the stable boundary before using any proposal or review
  record: [specification/overview.md](specification/overview.md).
- Checking whether proposal text is current behavior: start with
  [specification/README.md](specification/README.md), then compare
  the chosen proposal through
  [proposals/implementation-route.md](proposals/implementation-route.md).

## Proposal Work

- Choosing an implementation target:
  [proposals/target-selection.md](proposals/target-selection.md).
- Promoting a completed proposal into current specification material:
  [proposals/implementation-route.md](proposals/implementation-route.md).
- Reading incomplete design-wall rationale:
  [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md).
- Stop proposal reading when `specification/` already covers the
  behavior; update the specification route instead of reading older design-wall
  notes.

## History And Evidence

- Explaining why implemented behavior exists:
  [specification/source-decisions.md](specification/source-decisions.md),
  then [reference/source-decisions/README.md](reference/source-decisions/README.md).
- Routing from a topic area directly to rationale records:
  [reference/source-decisions/topic-map.md](reference/source-decisions/topic-map.md).
- Checking gap evidence or completion claims:
  [reviews/README.md](reviews/README.md).
- Implemented language behavior:
  [specification/README.md](specification/README.md).
- Source support for claims:
  [reference/bibliography/README.md](reference/bibliography/README.md).
- Source-decision rationale or record audits:
  [reference/source-decisions/README.md](reference/source-decisions/README.md).

## Research Support

- Auditing sources behind a decision or claim:
  [reference/bibliography/README.md](reference/bibliography/README.md).
- Start with source families when the topic is known, claim map when the claim
  is known, and reference metadata only when exact source details are needed.

## Status Work

- Moving text between proposal, review, and reference areas:
  [document-status.md](document-status.md).

## Documentation Maintenance

- Entry-page routing, document movement, or status labels:
  [document-status.md](document-status.md).
- Source-decision category routing, record placement, or storage audits:
  [reference/source-decisions/README.md](reference/source-decisions/README.md).
- Link-health work: start from the page being edited, then verify links across
  `docs/` after the route is updated.

## Reading Order

- Current behavior work: `specification/README.md` first, then the
  topic-specific short page, then the matching `*-full.md` file only if named.
- Implementation target work: `proposals/target-selection.md` first, then
  `proposals/implementation-route.md` for the chosen target.
- Documentation routing work: `document-status.md` first, then the README for
  only the directory whose classification is changing.
- Rationale work: current specification page first, then
  `specification/source-decisions.md`, then one source-decision category.
  Open `reference/source-decisions/records/` only when that category names one
  record.
- Research-source work: `reference/bibliography/README.md` first, then one
  short bibliography route page, then one matching full source section.
- Status or movement work: `document-status.md` before editing labels or moving
  text between directories.

## Route Boundaries

- A behavior page under `specification/` wins over proposal, phase, and
  review wording.
- A proposal page can describe an implementation target, but it is not current
  behavior until the language specification also says so.
- Promotion work updates the smallest matching language specification page and
  leaves unfinished proposal text in `proposals/`.
- A review or phase page can explain why work happened, but it is not a route
  for changing the language specification.
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
- Do not open `*-full.md` files before a short route page names the relevant
  section.
- Do not read bibliography details before a rationale or claim route needs
  source support.
