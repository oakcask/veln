# Document Status Rules

Use this page when adding, moving, or reclassifying durable documentation.
Start from [README.md](README.md) when you only need a reading route, and open
[document-status-full.md](document-status-full.md) only when changing labels,
promotion rules, or placement policy.

## Read First

- Current behavior belongs in [reference/language/README.md](reference/language/README.md).
- Planned or accepted targets belong in [proposals/README.md](proposals/README.md).
- Gap evidence belongs in [reviews/README.md](reviews/README.md).
- Historical implementation order belongs in [phases/README.md](phases/README.md).

## Choose One Route

- Reading task only: return to [README.md](README.md) or
  [navigation.md](navigation.md).
- Moving behavior, proposal, review, or phase text:
  [document-status-full.md#placement](document-status-full.md#placement).
- Updating README or topic-page routing:
  [document-status-full.md#entry-pages](document-status-full.md#entry-pages).
- Applying or changing status labels:
  [document-status-full.md#labels](document-status-full.md#labels).

## Placement Summary

- Use `reference/` for implemented behavior and durable rationale.
- Use `proposals/` for planned or accepted behavior that is not fully
  implemented.
- Use `reviews/` for evidence, gaps, and verification notes.
- Use `phases/` for ordering, plans, and historical implementation notes.

## Stop Rule

- Keep top-level and directory README files as short routing pages.
- Keep expected topic paths short when a file grows around historical detail;
  move the long body behind a sibling `*-full.md` file.
- Use `reference/language/` as the current behavior source before changing
  code, tests, diagnostics, or samples.

## Skip Unless Needed

- Do not move proposal text into `reference/` until current code and tests
  support it.
- Do not use phase or review files as the source for current behavior when
  `reference/language/` has a matching page.
