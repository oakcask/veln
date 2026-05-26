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
- [topic-map.md](topic-map.md) when you know the task area but not the right
  decision category.

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

- Choose the first category that matches the task. Do not scan individual
  `result-*.md` files before a category page points to one.
- [topic-map.md](topic-map.md): fastest route from a task area to the relevant
  category section.
- [language-surface.md](language-surface.md): syntax, names, typing,
  contracts, holes, and effects.
- [commands-output.md](commands-output.md): CLI behavior, JSON schemas, test
  selection, and observable I/O.
- [implementation-boundaries.md](implementation-boundaries.md): runtime, AST,
  architecture, mutability, and compatibility boundaries.
- [process-rationale.md](process-rationale.md): decision placement, comparison
  tasks, and repair policy.

## Category Route Order

- Start with [topic-map.md](topic-map.md) when the category is unclear.
- Open exactly one category page for the task area.
- Open an individual `result-*.md` record only when the category page names it.
- Return to `../language/` before treating any rationale text as current
  behavior.
- Use [result-index.md](result-index.md) only for audits that need the
  exhaustive record list.

## History

- [result-index.md](result-index.md): audit route for deduplication or moving a
  record between categories. It links to the exhaustive record list only after
  the category routes are not enough.

## Boundary

If a decision record includes open details or future extensions, the
implemented reference still wins. Planned or incomplete decisions live under
`../../proposals/agent-language-spec-wall/`.

## Skip Unless Needed

- Do not read individual `result-*.md` records before choosing one of the topic
  indexes above.
- Do not use these records as implementation status when
  `../language/README.md` says otherwise.
