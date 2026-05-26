# Document Status Rules Full

Use [document-status.md](document-status.md) first unless you are changing
documentation placement, route policy, or status labels.

## Placement

- Put behavior that works in the current code and tests in `reference/`.
- Put accepted or open design targets that are not fully implemented in
  `proposals/`.
- Keep promoted proposal routing and history in `proposals/`, but do not label
  those pages `implemented`.
- Put implemented rationale and decision history in `reference/source-decisions/`.
- Put planned or incomplete rationale and decision history in `proposals/`.
- Put implementation order, completion notes, and working plans in `phases/`.
- Put implementation gaps, verification evidence, and correction lists in
  `reviews/`.
- Treat `reference/language/` as the current behavior source before changing
  code, tests, diagnostics, or samples.

## Entry Pages

- Keep top-level and directory README files as routing pages.
- Keep expected topic paths short when a file grows around historical detail;
  move the long body behind a sibling `*-full.md` file.
- Link from short pages to the specific full section needed for a task instead
  of asking readers to scan a full record.
- Keep status and promotion rules in [document-status.md](document-status.md)
  and this full page; keep current behavior in `reference/language/`.

## Directories

- `reference/language/` is the first stop for implemented language behavior.
- `reference/source-decisions/` is historical rationale grouped by topic.
- `phases/` and `reviews/` contain longer working records behind short indexes.
- `proposals/` contains accepted or open targets that still need promotion into
  the reference after implementation.

## Labels

Use these status labels at the top of durable specification documents:

- `implemented`: use only for `reference/` pages whose described behavior is
  supported by current code and tests.
- `promoted`: a proposal page is retained only as routing or history because
  the implemented behavior has moved to `reference/`.
- `accepted-proposal`: a decision record accepted the target, but implementation
  is absent, incomplete, or only partially promoted.
- `open-proposal`: the design is being explored and should not be treated as a
  commitment.
- `superseded`: another document replaces this one.
- `rejected`: the project decided not to pursue this design.

## Status And Implementation Fields

- `Status:` describes the document's authority and placement, not whether every
  idea in the file exists in the product.
- `Implementation:` may appear on proposal pages, but it must name the covered
  scope instead of using a bare `implemented` when the proposal includes
  historical, partial, or future-facing material.
- Use `Implementation: not implemented` for open proposal work with no current
  behavior.
- Use `Implementation: implemented subset: ...` for promoted slices where the
  reference documents only part of the broader proposal.
- Use `Implementation: promoted to reference: ...` when all behavior still
  described by the short proposal page is current behavior and the matching
  reference page is the source of truth.
- A proposal page with `Status: promoted` must name the matching
  `reference/language/` page before it may describe any implementation as
  complete.
- Do not cite `Implementation:` on a proposal page as proof of current behavior;
  cite the matching `reference/` page.

## Skip Unless Needed

- Do not move text from a proposal into `reference/` until current code and
  tests support it.
- Do not use phase or review files as the source for current behavior when
  `reference/language/` has a matching page.
- Do not add new long background sections to a README when a short route plus a
  full detail page would preserve the same content with less first-pass
  reading.
