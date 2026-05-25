# Implemented Source Decisions

Status: implemented

These discussion results describe decisions that are represented by the current
implementation or by an intentional absence in the current reference. Read the
categorized files under `../language/` first when you need current behavior;
read these records only for rationale or compatibility context.

## Read When

- Use [Language Surface](#language-surface) for syntax, names, typing,
  contracts, holes, and effects.
- Use [Commands And Output](#commands-and-output) for CLI behavior, JSON
  schemas, test selection, and observable I/O.
- Use [Implementation Boundaries](#implementation-boundaries) for runtime,
  AST, architecture, mutability, and compatibility boundaries.
- Use [Process And Rationale](#process-and-rationale) for decision placement,
  comparison tasks, and repair policy.

## Language Surface

- [Block Structure](result-block-structure.md)
- [Compact Function Form](result-compact-function-form.md)
- [Contract Expression Language](result-contract-expression-language.md)
- [Contract Predicate Parsing](result-contract-predicate-parsing.md)
- [Contract Static Runtime Boundary](result-contract-static-runtime-boundary.md)
- [Effect Access Modes](result-effect-access-modes.md)
- [Effect Declaration Boundary](result-effect-declaration-boundary.md)
- [Error Type Inference](result-error-type-inference.md)
- [First-Slice Grammar](result-first-slice-grammar.md)
- [First-Slice Module Fields](result-first-slice-module-fields.md)
- [First-Slice Prelude Helpers](result-first-slice-prelude-helpers.md)
- [First-Slice Value Mutability](result-first-slice-value-mutability.md)
- [Hole Satisfy Constraint Grammar](result-hole-satisfy-constraint-grammar.md)
- [Hole Satisfy Source Syntax](result-hole-satisfy-source-syntax.md)
- [Method Call Boundary](result-method-call-boundary.md)
- [Minimum Type System for Holes](result-minimum-type-system-for-holes.md)
- [Named Hole Syntax](result-named-hole-syntax.md)
- [Pipeline Style](result-pipeline-style.md)
- [Postcondition Result Binding](result-postcondition-result-binding.md)
- [Public Function Type Boundaries](result-public-function-type-boundaries.md)
- [Scoping and Name Resolution](result-scoping-and-name-resolution.md)
- [Test Declaration Syntax](result-test-declaration-syntax.md)
- [User-Defined ADTs in the First Slice](result-user-defined-adts-first-slice.md)

## Commands And Output

- [Affected Test Selection](result-affected-test-selection.md)
- [Check JSON Details Fields](result-check-json-details-fields.md)
- [Doctest Error Type Fence Syntax](result-doctest-error-type-fence-syntax.md)
- [Doctest Expected Output Syntax](result-doctest-expected-output-syntax.md)
- [Doctest Result Propagation](result-doctest-result-propagation.md)
- [First Implementation Commands](result-first-implementation-commands.md)
- [First-Slice Observable I/O](result-first-slice-observable-io.md)
- [Hole Diagnostic JSON Shape](result-hole-diagnostic-json-shape.md)
- [JSON Diagnostic Schema Stability](result-json-diagnostic-schema-stability.md)
- [Minimal Project and Test Discovery](result-minimal-project-test-discovery.md)
- [Primary Check Command](result-primary-check-command.md)
- [Runtime Contract Failure Reporting](result-runtime-contract-failure-reporting.md)
- [Stdio API and Output Events](result-stdio-api-and-output-events.md)
- [Test JSON Shape](result-test-json-shape.md)

## Implementation Boundaries

- [AST Implementation Representation](result-ast-implementation-representation.md)
- [AST Phase Boundary](result-ast-phase-boundary.md)
- [Channel-First Concurrency Runtime](result-channel-first-concurrency-runtime.md)
- [Contract Blame Boundary](result-contract-blame-boundary.md)
- [First Implementation Architecture](result-first-implementation-architecture.md)
- [First Implementation Runtime Targets](result-first-implementation-runtime-targets.md)
- [Hole Runtime Boundary](result-hole-runtime-boundary.md)
- [Module Metadata Location](result-module-metadata-location.md)
- [Prelude Complexity Guarantees](result-prelude-complexity-guarantees.md)
- [Runtime Value Freeze Boundary](result-runtime-value-freeze-boundary.md)
- [Transitive Effect Diagnostics](result-transitive-effect-diagnostics.md)

## Process And Rationale

- [ADR-Lite Decision Location](result-adr-lite-decision-location.md)
- [Comparison Example Task](result-comparison-example-task.md)
- [Safe Repair Candidate Boundary](result-safe-repair-candidate-boundary.md)
- [Satisfy Unknown Severity](result-satisfy-unknown-severity.md)

## Boundary

If a decision record includes open details or future extensions, the
implemented reference still wins. Planned or incomplete decisions live under
`../../proposals/agent-language-spec-wall/`.
