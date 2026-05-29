# Document Status Rules Full

Use [document-status.md](document-status.md) first unless you are changing
documentation placement, route policy, or status labels.

## Placement

- Put language behavior that works in the current code and tests in
  `specification/`.
- The long-term documentation direction is thin prose over executable or
  mechanically checked behavior. When practical, move detailed behavior
  statements into fixtures, generated grammar, compiler tests, or CLI harness
  cases, and leave prose as routing, summary, and explanation.
- Prefer executable or mechanically checked specification evidence for language
  behavior when the behavior can be expressed that way. Use
  `examples/specification/`, generated grammar from
  `specification/source-surface-executable.pl`, compiler tests, or CLI harness
  cases as the concrete evidence behind prose behavior pages.
- Treat prose specification pages as routing and explanation over checked
  behavior. A prose-only change should describe current behavior only when the
  behavior is already covered by code and tests or cannot reasonably be made
  executable.
- Put proposed design targets that are not fully implemented in `proposals/`.
- Put implemented rationale and decision history in `reference/source-decisions/`.
- Put implemented proposal records in `reference/implemented-proposals/` after
  the observable behavior has moved into `specification/` or checked examples.
- Put planned or incomplete rationale and decision history in `proposals/`.
- Put implementation gaps, verification evidence, and correction lists in the
  matching proposal or reference page.
- Treat `specification/` as the current behavior source before changing
  code, tests, diagnostics, or samples.
- When prose and executable evidence disagree, resolve the mismatch by changing
  the implementation, executable evidence, or prose together instead of citing
  one stale surface as authoritative.

## Entry Pages

- Keep top-level and directory README files as routing pages.
- Keep expected topic paths short when a file grows around historical detail;
  move the long body behind a sibling `*-full.md` file.
- When executable evidence can replace detailed prose without losing reader
  guidance, shrink the prose and link to the evidence instead.
- Link from short pages to the specific full section needed for a task instead
  of asking readers to scan a full record.
- Keep status and promotion rules in [document-status.md](document-status.md)
  and this full page; keep current behavior in `specification/`.

## Directories

- `specification/` is the first stop for implemented language behavior.
- `reference/source-decisions/` is historical rationale grouped by topic.
- `reference/implemented-proposals/` is completed proposal history and
  completion evidence, not current behavior.
- `proposals/` contains proposed targets that still need promotion into the
  specification after implementation.

## Labels

Use these status labels at the top of durable documents:

- `implemented`: use only for `specification/` behavior pages or implemented
  reference rationale pages whose described behavior is supported by current
  code and tests.
- `proposed`: the target is committed as proposal text, but implementation is
  absent or incomplete.
- `routing`: the page is an index or selection route and does not define
  behavior by itself.
- `closed`: a former proposal route remains only to preserve old links and no
  longer carries implementation requirements.
- `superseded`: another document replaces this one.
- `rejected`: the project decided not to pursue this design.

## Status And Implementation Fields

- `Status:` describes the document's authority and placement, not whether every
  idea in the file exists in the product.
- Proposal pages should not use `Implementation:` to describe current behavior.
- When proposal behavior becomes implemented, move the behavior into
  `specification/`, move completed records to `reference/implemented-proposals/`,
  and leave only absent proposal work or a short closed route.
- Do not cite a proposal page as proof of current behavior; cite the matching
  `specification/` page.

## Skip Unless Needed

- Do not move text from a proposal into `specification/` until current code and
  tests support it.
- Do not use proposal files as the source for current behavior when
  `specification/` has a matching page.
- Do not add new long background sections to a README when a short route plus a
  full detail page would preserve the same content with less first-pass
  reading.
