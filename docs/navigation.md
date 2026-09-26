---
role: routing
update-when: A documentation route is added, moved, reclassified, or no longer points to the smallest current authority.
---

# Documentation Navigation

Use this page after [README.md](README.md) when the first route is not obvious.
Choose one route and stop when the linked short page answers the question.

## Current Behavior

- Changing syntax, types, effects, contracts, holes, commands, JSON output,
  runtime behavior, or examples:
  [specification/topic-map.md](specification/topic-map.md).
- Checking the stable boundary before using any proposal record:
  [specification/overview.md](specification/overview.md).
- Checking whether proposal text is current behavior: start with
  [specification/README.md](specification/README.md), then compare the chosen
  proposal with the matching specification page.
- Checking agent routing between published language topics and repository work:
  [specification/agent-language-routing.md](specification/agent-language-routing.md).

## Proposal Work

- Choosing, implementing, or promoting proposal work:
  [proposals/README.md](proposals/README.md), then the matching specification
  page.
- Stop proposal reading when `specification/` already covers the behavior;
  update the specification route instead of reading older design-wall notes.

## History, Evidence, and Research

- Explaining why implemented behavior exists:
  [specification/source-decisions.md](specification/source-decisions.md), then
  [reference/source-decisions/README.md](reference/source-decisions/README.md).
- Routing from a topic area directly to rationale records:
  [reference/source-decisions/topic-map.md](reference/source-decisions/topic-map.md).
- Auditing sources behind a decision or claim:
  [reference/bibliography/README.md](reference/bibliography/README.md).

## Documentation Maintenance

- Documentation structure, metadata, presentation, and specification writing:
  [reference/documentation-authoring.md](reference/documentation-authoring.md).
- Entry-page routing, document movement, or status labels: use the README for
  the directory whose classification is changing.
- Link-health work: start from the page being edited, then verify links across
  `docs/` after the route is updated.

## Route Boundaries

- A behavior page under `specification/` wins over proposal wording.
- A proposal page is not current behavior until the matching specification
  page says so.
- Promotion work updates the smallest matching specification page, removes
  completed proposal text, and leaves only unfinished proposal work in
  `proposals/`.
- Open a detail record only through the route that names its relevant subject.
- Open `result-*.md` source-decision records through
  [reference/source-decisions/topic-map.md](reference/source-decisions/topic-map.md)
  for task work or
  [reference/source-decisions/README.md#record-placement](reference/source-decisions/README.md#record-placement)
  for record audits.

## Skip Unless Needed

- Do not use this page when one of the top-level README's read-first links
  already matches the task.
- Do not open broad detail records before a route page names the relevant
  subject.
- Do not read bibliography details before a rationale or claim route needs
  source support.
