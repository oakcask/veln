# Implemented Source Decisions

Status: implemented

These discussion results describe decisions that are represented by the current
implementation or by an intentional absence in the current reference. Read the
categorized files under `../language/` first when you need current behavior;
read these records only for rationale or compatibility context.

## Read First

- [../language/README.md](../language/README.md) for implemented behavior.
- [../language/source-decisions.md](../language/source-decisions.md) for the
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

## Read When

- Open exactly one category page for the task area, then one `result-*.md`
  record only when that category page names it.
- Use [result-index.md](result-index.md) only for audits that need the full
  record list or need to move records between categories.

## Boundary

If a decision record includes open details or future extensions, the
implemented reference still wins. Planned or incomplete decisions live under
`../../proposals/agent-language-spec-wall/`.

## Skip Unless Needed

- Do not read individual `result-*.md` records before choosing one of the topic
  indexes above.
- Do not use these records as implementation status when
  `../language/README.md` says otherwise.
