# Document Status Rules

Use this page when adding, moving, or reclassifying durable documentation.
Start from [README.md](README.md) when you only need a reading route.

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

## Labels

Use these status labels at the top of durable specification documents:

- `implemented`: current code and tests support the described behavior.
- `accepted-proposal`: a decision record accepted the target, but implementation
  is absent or incomplete.
- `open-proposal`: the design is being explored and should not be treated as a
  commitment.
- `superseded`: another document replaces this one.
- `rejected`: the project decided not to pursue this design.
