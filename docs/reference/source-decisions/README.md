# Implemented Source Decisions

Status: implemented

These discussion results describe decisions represented by the current
implementation or by an intentional absence in the current specification. Read
`../../specification/` first for behavior; use this area only for rationale,
compatibility context, or record-placement audits.

## Read First

- [../../specification/README.md](../../specification/README.md) for implemented behavior.
- [../../specification/source-decisions.md](../../specification/source-decisions.md) for the
  short language-facing rationale route.
- [topic-map.md](topic-map.md) when the task area is known but the decision
  category is not.

## Choose One Route

- Source syntax, names, types, contracts, holes, or effects:
  [language-surface.md](language-surface.md).
- Commands, diagnostics, JSON output, tests, doctests, or observable output:
  [commands-output.md](commands-output.md).
- AST shape, runtime targets, mutability, concurrency, or compatibility
  boundaries: [implementation-boundaries.md](implementation-boundaries.md).
- Documentation placement, comparison tasks, repair policy, or diagnostic
  severity: [Process Rationale](#process-rationale).

## Process Rationale

Read these records only when documentation placement, comparison tasks, repair
policy, or diagnostic severity need rationale.

- Documentation and evaluation:
  [ADR-Lite Decision Location](records/result-adr-lite-decision-location.md),
  [Comparison Example Task](records/result-comparison-example-task.md).
- Repair and diagnostics policy:
  [Safe Repair Candidate Boundary](records/result-safe-repair-candidate-boundary.md),
  [Satisfy Unknown Severity](records/result-satisfy-unknown-severity.md).

## Audit Routes

- Use this page for record placement audits before opening storage details.
- Storage layer: [records/README.md](records/README.md).
- Exhaustive grouped list: [records/result-index-full.md](records/result-index-full.md).

## Record Placement

Use this route only when auditing source-decision record placement or storage.
For normal implementation work, choose the smallest category page above.

- Check whether a record is routed through the right category page.
- Move a source-decision record between category routes.
- Find the exhaustive, category-grouped storage list without scanning
  `records/` directly.

Topic routes:

- Language syntax, names, types, contracts, holes, and effects:
  [language-surface.md](language-surface.md).
- CLI behavior, JSON schemas, doctests, tests, and observable I/O:
  [commands-output.md](commands-output.md).
- Runtime, AST, architecture, mutability, concurrency, and compatibility
  boundaries: [implementation-boundaries.md](implementation-boundaries.md).
- Decision placement, comparison tasks, repair policy, and diagnostic severity:
  [Process Rationale](#process-rationale).
- Storage-only exhaustive list grouped by the routes above:
  [records/result-index-full.md](records/result-index-full.md).

## Read When

- Open exactly one category route for the task area, then one `result-*.md`
  record only when that route names it.
- Use [Record Placement](#record-placement) only for audits that need category
  placement or a route to the full record set.
- Use [records/README.md](records/README.md) only when checking the storage
  layer itself.

## Boundary

If a decision record includes open details or future extensions, the
implemented reference still wins. Planned or incomplete decisions live under
`../../proposals/`.

## Skip Unless Needed

- Do not read individual `result-*.md` records before choosing one of the topic
  indexes above.
- Do not use these records as implementation status when
  `../../specification/README.md` says otherwise.
- Do not open the full list when a language topic page, command page, or
  category route already names the record needed for the task.
