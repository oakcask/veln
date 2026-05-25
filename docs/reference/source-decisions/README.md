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

## Read When

- Choose the first category that matches the task. Do not scan individual
  `result-*.md` files before a category page points to one.
- [language-surface.md](language-surface.md): syntax, names, typing,
  contracts, holes, and effects.
- [commands-output.md](commands-output.md): CLI behavior, JSON schemas, test
  selection, and observable I/O.
- [implementation-boundaries.md](implementation-boundaries.md): runtime, AST,
  architecture, mutability, and compatibility boundaries.
- [process-rationale.md](process-rationale.md): decision placement, comparison
  tasks, and repair policy.

## History

- [result-index.md](result-index.md): exhaustive record index for audits,
  deduplication, or moving a record between categories. Do not start there for
  ordinary implementation work.

## Boundary

If a decision record includes open details or future extensions, the
implemented reference still wins. Planned or incomplete decisions live under
`../../proposals/agent-language-spec-wall/`.

## Skip Unless Needed

- Do not read individual `result-*.md` records before choosing one of the topic
  indexes above.
- Do not use these records as implementation status when
  `../language/README.md` says otherwise.
