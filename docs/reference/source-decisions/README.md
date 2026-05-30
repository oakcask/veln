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
  severity: [process-rationale.md](process-rationale.md).

## Audit Routes

- Record placement: [result-index.md](result-index.md).
- Storage layer: [records/README.md](records/README.md).
- Exhaustive grouped list: [records/result-index-full.md](records/result-index-full.md).

## Read When

- Open exactly one category page for the task area, then one `result-*.md`
  record only when that category page names it.
- Use [result-index.md](result-index.md) only for audits that need category
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
