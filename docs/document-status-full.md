# Document Status Rules Full

Use [document-status.md](document-status.md) first unless you are changing
documentation placement, route policy, or status labels.

## Placement

- Put behavior that works in the current code and tests in `reference/`.
- Put accepted or open design targets that are not fully implemented in
  `proposals/`.
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

- `implemented`: current code and tests support the described behavior.
- `accepted-proposal`: a decision record accepted the target, but implementation
  is absent or incomplete.
- `open-proposal`: the design is being explored and should not be treated as a
  commitment.
- `superseded`: another document replaces this one.
- `rejected`: the project decided not to pursue this design.

## Skip Unless Needed

- Do not move text from a proposal into `reference/` until current code and
  tests support it.
- Do not use phase or review files as the source for current behavior when
  `reference/language/` has a matching page.
- Do not add new long background sections to a README when a short route plus a
  full detail page would preserve the same content with less first-pass
  reading.
