# Source Decisions

These discussion results support the implemented language specification. Read
them for rationale or future design work, not as a substitute for the
categorized specification files. The full implemented decision index is
[../source-decisions/README.md](../source-decisions/README.md).

## Read First

- Current implemented behavior: [README.md](README.md).
- Task-specific behavior route: [topic-map.md](topic-map.md).
- Source decision category route:
  [../source-decisions/README.md](../source-decisions/README.md).

## Choose One Route

- Use [../source-decisions/topic-map.md](../source-decisions/topic-map.md)
  when you know the task area but not the decision category.
- Syntax, names, types, contracts, holes, effects, tests, or source grammar:
  [../source-decisions/language-surface.md](../source-decisions/language-surface.md).
- Command behavior, JSON schemas, doctests, test selection, or observable I/O:
  [../source-decisions/commands-output.md](../source-decisions/commands-output.md).
- Runtime, AST, architecture, mutability, concurrency, or compatibility:
  [../source-decisions/implementation-boundaries.md](../source-decisions/implementation-boundaries.md).
- Decision placement, comparison tasks, repair policy, or diagnostic severity:
  [../source-decisions/process-rationale.md](../source-decisions/process-rationale.md).

## Read When

- Choose one categorized route instead of scanning `result-*.md` records
  directly.
- Return to the language reference before treating rationale text as current
  behavior.
- Open an individual source-decision record only when a category route names
  it.

## Skip Unless Needed

- Do not use source decisions as implementation status when a language
  reference page says otherwise.
- Do not open [../source-decisions/result-index.md](../source-decisions/result-index.md)
  unless auditing the full decision-record set.
