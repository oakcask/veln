# Source Decisions

These discussion results support the implemented language specification. Read
them for rationale or future design work, not as a substitute for the
categorized specification files. The full implemented decision index is
[../source-decisions/README.md](../source-decisions/README.md).

## Read When

- Choose one categorized route instead of scanning `result-*.md` records
  directly. Return to the language reference when you need implemented
  behavior rather than rationale.
- [Language surface decisions](../source-decisions/language-surface.md):
  syntax, names, types, contracts, holes, effects, tests, and source grammar.
- [Command and output decisions](../source-decisions/commands-output.md):
  command behavior, JSON schemas, doctests, test selection, and observable I/O.
- [Implementation boundary decisions](../source-decisions/implementation-boundaries.md):
  runtime, AST, architecture, mutability, concurrency, and compatibility
  boundaries.
- [Process and rationale decisions](../source-decisions/process-rationale.md):
  decision placement, comparison tasks, repair policy, and diagnostic severity.
