# Document Status Rules

Use this page when adding, moving, or reclassifying durable documentation.
Start from [README.md](README.md) when you only need a reading route, and open
[document-status-full.md](document-status-full.md) only when changing labels,
promotion rules, or placement policy.

## Read First

- Current behavior belongs in [specification/README.md](specification/README.md).
- Keep prose specification thin over time by moving behavior detail into
  executable or mechanically checked evidence when practical.
- Prefer executable or checked specification evidence for language behavior:
  `examples/specification/`, generated grammar from
  `specification/source-surface-executable.pl`, compiler tests, or CLI
  harness cases.
- Proposal targets belong in [proposals/README.md](proposals/README.md).
- Implemented proposal records belong in
  [reference/implemented-proposals/README.md](reference/implemented-proposals/README.md).
- Gap evidence belongs in the matching proposal or reference page.

## Choose One Route

- Reading task only: return to [README.md](README.md) or
  [navigation.md](navigation.md).
- Moving behavior or proposal text:
  [document-status-full.md#placement](document-status-full.md#placement).
- Updating README or topic-page routing:
  [document-status-full.md#entry-pages](document-status-full.md#entry-pages).
- Applying or changing status labels:
  [document-status-full.md#labels](document-status-full.md#labels).
- Distinguishing document status from implementation coverage:
  [document-status-full.md#status-and-implementation-fields](document-status-full.md#status-and-implementation-fields).

## Placement Summary

- Use `specification/` for current implemented language behavior.
- Use prose specification pages to route, summarize, and explain checked
  behavior. Do not grow prose as a substitute for executable or mechanically
  verified examples when behavior can be expressed that way.
- Use `reference/` for durable rationale, source support, and completed
  proposal records.
- Use `proposals/` for proposed behavior that is not fully implemented.
- Keep evidence, gaps, and verification notes in the matching proposal or
  reference page.

## Stop Rule

- Keep top-level and directory README files as short routing pages.
- Keep expected topic paths short when a file grows around historical detail;
  move the long body behind a sibling `*-full.md` file.
- Prefer shrinking detailed prose when executable coverage can carry the same
  behavior more directly.
- Use `specification/` as the current behavior source before changing
  code, tests, diagnostics, or samples.
- When prose and executable evidence disagree, resolve the mismatch by updating
  the implementation, executable evidence, or prose together.

## Skip Unless Needed

- Do not move proposal text into `specification/` until current code and tests
  support it.
- Do not use proposal files as the source for current behavior when
  `specification/` has a matching page.
