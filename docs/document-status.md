# Document Status Rules

Use this page when adding, moving, or reclassifying durable documentation.
Start from [README.md](README.md) when you only need a reading route, and open
[document-status-full.md](document-status-full.md) only when changing labels,
promotion rules, or placement policy.

## Read First

- Current behavior belongs in [specification/README.md](specification/README.md).
- Active proposal targets and implemented proposal records belong in
  [proposals/README.md](proposals/README.md).
- Gap evidence belongs in [reviews/README.md](reviews/README.md).
- Implemented language behavior belongs in
  [specification/README.md](specification/README.md).

## Choose One Route

- Reading task only: return to [README.md](README.md) or
  [navigation.md](navigation.md).
- Moving behavior, proposal, or review text:
  [document-status-full.md#placement](document-status-full.md#placement).
- Updating README or topic-page routing:
  [document-status-full.md#entry-pages](document-status-full.md#entry-pages).
- Applying or changing status labels:
  [document-status-full.md#labels](document-status-full.md#labels).
- Distinguishing document status from implementation coverage:
  [document-status-full.md#status-and-implementation-fields](document-status-full.md#status-and-implementation-fields).

## Placement Summary

- Use `specification/` for current implemented language behavior.
- Use `reference/` for durable rationale and source support.
- Use `proposals/` for proposed behavior that is not fully implemented and for
  implemented proposal records that still carry history or cleanup routes.
- Use `reviews/` for evidence, gaps, and verification notes.

## Stop Rule

- Keep top-level and directory README files as short routing pages.
- Keep expected topic paths short when a file grows around historical detail;
  move the long body behind a sibling `*-full.md` file.
- Use `specification/` as the current behavior source before changing
  code, tests, diagnostics, or samples.

## Skip Unless Needed

- Do not move proposal text into `specification/` until current code and tests
  support it.
- Do not use proposal or review files as the source for current behavior when
  `specification/` has a matching page.
