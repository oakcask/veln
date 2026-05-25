# Agent-Oriented Language Spec Wall Design Brief

Status: open-proposal

Source: <https://oakcask.github.io/docs/202605-programming-language-for-agents/>

## Purpose

This note starts the specification discussion for the experimental Veln
implementation. It treats the source essay as a design brief, not as a frozen
specification.

The core premise is that an agent-friendly language should minimize total
working area: generated syntax, context to read, failed-edit repair size,
diagnostic interpretation cost, and verification scope. Initial generation
speed matters, but the more important target is the whole loop from generation
to green tests.

## Current Thesis

Veln should optimize for short repair loops rather than just short programs.

The language should combine a small readable surface syntax, local static checks
at important boundaries, executable specification fragments, typed holes,
structured diagnostics, and a standard toolchain. These parts should be treated
as one product surface. If they are designed separately, the agent still has to
infer too much from prose, project layout, and tool-specific output.

## Tentative Design Anchors

- Syntax should have a small number of standard forms. Reducing equivalent ways
  to express the same operation is expected to reduce generation variance and
  review noise.
- Small programs should run with minimal setup. Project structure should appear
  only when the code needs module boundaries, tests, packages, or generated
  docs.
- Public APIs, external I/O, persistence, network boundaries, and safety
  boundaries should carry stronger types, schemas, contracts, and effect
  information than short internal transformations.
- Recoverable failures should be represented as `Result`; absence should be
  represented as `Option`.
- `?` should propagate only from the current result-returning function or
  result-returning anonymous function. Fallible collection transforms should
  use `try_map` or an equivalent traversal primitive.
- Typed holes should be valid partial-program expressions. A hole should keep
  the file parseable and allow type, binding, contract, candidate, and related
  test queries.
- Contracts should be close to code and reusable for runtime checks,
  diagnostics, docs, examples, and test generation.
- Effects should be coarse at first. Labels such as `stdio`, `fs`, `net`, `db`,
  `time`, `random`, `process`, and `concurrency` are probably enough for the
  first design slice.
- The standard tool should expose human-readable output and JSON output for the
  same diagnostics.

## Proposed First Slice

The first slice should be deliberately small:

- Parser and formatter for `fn`, `end`, `let`-style bindings if needed,
  function calls, records, lists, `match`, `Result`, `Option`, and `_` holes.
- Type checker with enough inference to report expected types for holes and to
  require explicit signatures on public functions.
- `Result` propagation with `?` and explicit diagnostics for invalid `map` use.
- Contracts limited to boolean expressions in `require` and `ensure`.
- Resolved by
  [Names And Effects](../../reference/language/names-effects.md): coarse effect
  labels are parsed, unknown labels are reported, and first-slice enforcement
  remains shallow.
- `veln check --json` as the first agent-facing command.
- Golden tests for diagnostics, especially typed-hole output and `?` behavior.

This slice is enough to test whether the central loop is viable: write partial
code, get structured information, fill holes, and run focused checks.

## Decisions To Revisit

- `do ... end` is useful for partial generation, but the parser should be tested
  against indentation-only and brace variants before treating it as final.
- `Result` and `Option` should be in the core model, but the syntax for their
  constructors can remain conventional until examples show friction.
- Resolved by
  [Safe Repair Candidate Boundary](../../reference/source-decisions/result-safe-repair-candidate-boundary.md):
  `safe repair` should initially mean a machine-readable candidate with reason,
  evidence, limits, and verification hints, not automatic edit application.
- Test selection should prefer false positives over false negatives. If the
  graph is incomplete, the tool should say so and fall back to broader tests.

## Next Discussion Topics

1. Resolved by [First-Slice Grammar](../../reference/source-decisions/result-first-slice-grammar.md): pick the
   exact first-slice grammar.
2. Define the JSON shape for `veln check --json`.
3. Decide how holes, contracts, and effects appear in the AST.
4. Choose whether the first interpreter evaluates code with holes or only checks
   it.
5. Resolved by
   [Comparison Example Task](../../reference/source-decisions/result-comparison-example-task.md):
   write comparison examples against Ruby, Python, TypeScript, Elixir, and Rust
   using one line-item order summary task.
