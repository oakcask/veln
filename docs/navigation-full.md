# Documentation Navigation Full

Use this page after [navigation.md](navigation.md) when the short route is not
enough. Choose one route and stop when the linked short page answers the
question.

## Current Behavior

- Changing syntax, types, effects, contracts, holes, commands, JSON output,
  runtime behavior, or examples:
  [reference/language/topic-map.md](reference/language/topic-map.md).
- Checking the stable boundary before using any proposal, review, or phase
  record: [reference/language/overview.md](reference/language/overview.md).
- Checking whether proposal text is current behavior: start with
  [reference/language/README.md](reference/language/README.md), then compare
  the selected proposal through
  [proposals/implementation-route.md](proposals/implementation-route.md).

## Proposal Work

- Choosing an implementation target:
  [proposals/target-queue.md](proposals/target-queue.md).
- Promoting a completed proposal into reference material:
  [proposals/implementation-route.md](proposals/implementation-route.md).
- Reading incomplete design-wall rationale:
  [proposals/agent-language-spec-wall/README.md](proposals/agent-language-spec-wall/README.md).
- Stop proposal reading when the target queue says the behavior is already
  covered by `reference/language/`; update the reference route instead of
  reading older design-wall notes.

## Rationale And History

- Explaining why implemented behavior exists:
  [reference/language/source-decisions.md](reference/language/source-decisions.md),
  then [reference/source-decisions/README.md](reference/source-decisions/README.md).
- Routing from a topic area directly to rationale records:
  [reference/source-decisions/topic-map.md](reference/source-decisions/topic-map.md).
- Checking gap evidence or completion claims:
  [reviews/README.md](reviews/README.md).
- Reconstructing implementation order: [phases/README.md](phases/README.md).

## Research Support

- Auditing sources behind a decision or claim:
  [reference/bibliography/README.md](reference/bibliography/README.md).
- Start with source families when the topic is known, claim map when the claim
  is known, and reference metadata only when exact source details are needed.

## Status Work

- Moving text between proposal, review, phase, and reference areas:
  [document-status.md](document-status.md).

## Reading Order

- Current behavior work: `reference/language/README.md` first, then the
  topic-specific short page, then the matching `*-full.md` file only if named.
- Implementation target work: `proposals/target-queue.md` first, then
  `proposals/implementation-route.md` for the selected target.
- Documentation routing work: `document-status.md` first, then the README for
  only the directory whose classification is changing.
- Rationale work: current reference page first, then
  `reference/language/source-decisions.md`, then one source-decision category.
- Research-source work: `reference/bibliography/README.md` first, then one
  short bibliography route page, then one matching full source section.
- Status or movement work: `document-status.md` before editing labels or moving
  text between directories.

## Route Boundaries

- A behavior page under `reference/language/` wins over proposal, phase, and
  review wording.
- A proposal page can describe an implementation target, but it is not current
  behavior until the language reference also says so.
- A review or phase page can explain why work happened, but it is not a route
  for changing the language specification.
- A `*-full.md` file is a detail record. Open it only through the short page
  that names the relevant section.

## Skip Unless Needed

- Do not use this page when one of the top-level README's read-first links
  already matches the task.
- Do not open `*-full.md` files before a short route page names the relevant
  section.
- Do not read bibliography details before a rationale or claim route needs
  source support.
