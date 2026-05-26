# Source Decision Record Index

Use this page only when auditing source-decision record placement or storage.
For normal implementation work, start with [README.md](README.md) and choose
the smallest category page.

## Read First

- Current implemented behavior: [../language/README.md](../language/README.md).
- Category route: [README.md](README.md).
- Record storage route: [records/README.md](records/README.md).

## Use For

- Checking whether a record is routed through the right category page.
- Moving a source-decision record between category routes.
- Finding the exhaustive storage list without scanning `records/` directly.

## Topic Routes

- Language syntax, names, types, contracts, holes, and effects:
  [language-surface.md](language-surface.md).
- CLI behavior, JSON schemas, doctests, tests, and observable I/O:
  [commands-output.md](commands-output.md).
- Runtime, AST, architecture, mutability, concurrency, and compatibility
  boundaries: [implementation-boundaries.md](implementation-boundaries.md).
- Decision placement, comparison tasks, repair policy, and diagnostic severity:
  [process-rationale.md](process-rationale.md).
- Storage-only exhaustive list:
  [records/result-index-full.md](records/result-index-full.md).

## Skip Unless Needed

Do not open the full list when a language topic page, command page, or category
route already names the record needed for the task.
