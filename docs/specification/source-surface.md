# Source Surface

This is the routing page for implemented source syntax. Use it to choose the
smallest section to read before opening the full grammar notes.

## Read First

- Module headers, imports, functions, tests, source ADT type declarations,
  source path derived local module identity, public member aliases, canonical
  `#` comments, `##` documentation comments, doctests, and ADR-lite metadata:
  [source-surface-full.md](source-surface-full.md).
  Use [commands.md](commands.md) for formatter layout and canonical comment
  spelling behavior.
- Doctest fence metadata, `runtime=contract`, `runtime=ensure`, and
  `runtime=result` expectations, expected-output fences, `> ` hidden setup,
  visible hash comments inside doctests, and negative examples:
  [source-surface-full.md#documentation-comments-and-doctests](source-surface-full.md#documentation-comments-and-doctests).
- Expression forms, constructors, records, dictionaries, vecs, matches,
  pipelines, and method-call diagnostics:
  [source-surface-full.md](source-surface-full.md#expressions).
- Contract predicate grammar:
  [source-surface-full.md](source-surface-full.md#contract-predicates).

## Read When

- Updating parser behavior, AST source shape, source metadata, or declaration
  rules.
- Checking whether a syntax feature is implemented rather than proposed.
- Aligning examples, diagnostics, or command behavior with accepted source
  syntax.

## Skip Unless Needed

- Do not read proposal or phase history before this page and the relevant
  section of [source-surface-full.md](source-surface-full.md).
- Use [source-decisions.md](source-decisions.md) only when rationale is needed
  after the implemented source behavior is clear.

## Grammar

See [source-surface-full.md#grammar](source-surface-full.md#grammar).

## Expressions

See [source-surface-full.md#expressions](source-surface-full.md#expressions).

## Contract Predicates

See
[source-surface-full.md#contract-predicates](source-surface-full.md#contract-predicates).
