# Agent-Oriented Language Spec Wall

Date: 2026-05-24

Source: <https://oakcask.github.io/docs/202605-programming-language-for-agents/>

This is the short routing page for the agent-oriented language specification
discussion. Keep it small enough to read before choosing a detail file.

## Read First

- [../reference/implemented-first-slice.md](../reference/implemented-first-slice.md)
  is the fixed reference for behavior implemented in the current workspace.
  Read it before mining discussion results for current behavior.
- [design-brief.md](2026-05-24-agent-language-spec-wall/design-brief.md)
  contains the current thesis, design anchors, first slice, and next topics.
- [open-questions.md](2026-05-24-agent-language-spec-wall/open-questions.md)
  lists unresolved questions and links resolved questions to their decision
  records.
- Read a discussion result only when you need the decision, rationale, or
  compatibility rule for that topic.

## Current Thesis

Veln should optimize for short repair loops rather than just short programs.

The language should treat syntax, local static checks, executable specification
fragments, typed holes, structured diagnostics, and the standard toolchain as
one agent-facing product surface.

## Discussion Result Rules

- Do not append full `Discussion Result` bodies to this index.
- Add each result as one file under
  `2026-05-24-agent-language-spec-wall/` named
  `result-<topic-slug>.md`.
- Start each result file with `# Discussion Result: <Topic>` and keep the
  picked question, decision, rationale, applicable first-slice rules, open
  details, and consequence in that file.
- Update the decision log below with a one-line link whenever a result file is
  added.
- Update `open-questions.md` so resolved questions point to the result file
  instead of using an inline "resolved" note with no link.
- If a result grows beyond one focused decision, split evidence or examples into
  a separate topic file and link it from the result.
- Promote stable decisions to a future `reference/` document instead of
  expanding this discussion index.

## Decision Log

- [Block Structure](2026-05-24-agent-language-spec-wall/result-block-structure.md):
  use explicit keyword-delimited first-slice blocks closed by `end`, with
  indentation as formatting and braces excluded as statement-block delimiters.
- [Compact Function Form](2026-05-24-agent-language-spec-wall/result-compact-function-form.md):
  do not include a separate compact syntax for named single-expression
  functions in the first slice; keep named `fn` bodies closed by `end`.
- [Method Call Boundary](2026-05-24-agent-language-spec-wall/result-method-call-boundary.md):
  use function calls as the only general first-slice call form, delaying method
  calls while keeping field access and future module qualification separate.
- [First-Slice Grammar](2026-05-24-agent-language-spec-wall/result-first-slice-grammar.md):
  use a small line-oriented, keyword-delimited, expression-centered grammar with
  explicit public signatures, public effect declarations, records, lists,
  `match`, holes, plain and qualified calls, pipelines, and `end`-closed
  blocks. The current consolidated grammar is
  [proposals/grammar-target.md](../proposals/grammar-target.md).
- [JSON Diagnostic Schema Stability](2026-05-24-agent-language-spec-wall/result-json-diagnostic-schema-stability.md):
  use a stable top-level diagnostic envelope while allowing prototype
  kind-specific `details` payloads to change.
- [Public Function Type Boundaries](2026-05-24-agent-language-spec-wall/result-public-function-type-boundaries.md):
  require explicit parameter and return types on public functions from the
  first slice, while allowing private helper inference.
- [Minimum Type System for Holes](2026-05-24-agent-language-spec-wall/result-minimum-type-system-for-holes.md):
  use local, mostly monomorphic inference plus built-in parametric `Option`,
  `Result`, collection, record, and function types for first-slice hole
  diagnostics.
- [User-Defined ADTs in the First Slice](2026-05-24-agent-language-spec-wall/result-user-defined-adts-first-slice.md):
  defer user-defined ADT declarations while allowing built-in `Result` and
  `Option`, records, collections, function types, primitives, and opaque named
  types to support first-slice signatures and diagnostics.
- [Primary Check Command](2026-05-24-agent-language-spec-wall/result-primary-check-command.md):
  make `veln check` the primary read-only command for parse, type, contract,
  effect, lint, doc drift, and hole diagnostics.
- [First Implementation Commands](2026-05-24-agent-language-spec-wall/result-first-implementation-commands.md):
  require `check`, `fmt`, `run`, and `test` in the first implementation, while
  deferring `doc`, `graph`, `explain`, and `repair`.
- [Test JSON Shape](2026-05-24-agent-language-spec-wall/result-test-json-shape.md):
  make `veln test --json` one run-level native JSON result with deterministic
  summary counts, gate diagnostics, suite errors, per-case records, and captured
  events.
- [Test Declaration Syntax](2026-05-24-agent-language-spec-wall/result-test-declaration-syntax.md):
  mark user-authored executable test cases with a dedicated top-level `test`
  declaration instead of promoting ordinary zero-argument `fn` declarations.
- [Affected Test Selection](2026-05-24-agent-language-spec-wall/result-affected-test-selection.md):
  before a full dependency graph exists, automatic test selection should widen
  when evidence is incomplete and report selection confidence.
- [Minimal Project and Test Discovery](2026-05-24-agent-language-spec-wall/result-minimal-project-test-discovery.md):
  share one project context across `check`, `run`, and `test`, using explicit
  targets, source-relative local imports, explicit run entries, and
  conservative test discovery before manifests and `graph` exist.
- [Hole Runtime Boundary](2026-05-24-agent-language-spec-wall/result-hole-runtime-boundary.md):
  allow files with holes to be checked, but block `run` when a hole may be
  reachable from the selected entry point.
- [Hole Diagnostic JSON Shape](2026-05-24-agent-language-spec-wall/result-hole-diagnostic-json-shape.md):
  report unfilled holes as stable-envelope `kind: "hole"` diagnostics with a
  prototype repair-context `details` payload.
- [Named Hole Syntax](2026-05-24-agent-language-spec-wall/result-named-hole-syntax.md):
  support both anonymous `_` holes and named expression holes, treating names
  as diagnostic and repair labels rather than bindings.
- [Error Type Inference](2026-05-24-agent-language-spec-wall/result-error-type-inference.md):
  require public `Result` return types to name their error type, while allowing
  private helpers to infer only one concrete propagated error type before
  explicit conversion or annotation is needed.
- [First-Slice Value Mutability](2026-05-24-agent-language-spec-wall/result-first-slice-value-mutability.md):
  use immutable bindings and immutable aggregate values, model container
  updates as new values, and leave the concrete GC strategy unspecified.
- [First Implementation Runtime Targets](2026-05-24-agent-language-spec-wall/result-first-implementation-runtime-targets.md):
  use a backend-neutral typed IR, make the JVM the first reference execution
  target, and treat Node-hosted WebAssembly as an experimental target with
  JavaScript glue rather than a custom Veln heap.
- [First Implementation Architecture](2026-05-24-agent-language-spec-wall/result-first-implementation-architecture.md):
  implement the first toolchain in Rust through typed IR and initial JVM
  lowering, use a small Java or Kotlin JVM runtime library, and defer a separate
  Kotlin JVM backend module until the IR and examples stabilize.
- [Channel-First Concurrency Runtime](2026-05-24-agent-language-spec-wall/result-channel-first-concurrency-runtime.md):
  use a parallel-capable runtime without a global interpreter lock, with
  structured tasks and bounded MPSC channels as the default coordination model.
- [Runtime Value Freeze Boundary](2026-05-24-agent-language-spec-wall/result-runtime-value-freeze-boundary.md):
  require ordinary Veln values to be frozen, transitively immutable, and safely
  published before they cross task or channel boundaries.
- [Pipeline Style](2026-05-24-agent-language-spec-wall/result-pipeline-style.md):
  prefer pipeline only for multi-step data flow over one subject; keep plain
  function calls as the default for simple composition.
- [Contract Blame Boundary](2026-05-24-agent-language-spec-wall/result-contract-blame-boundary.md):
  failed `require` clauses default to caller blame and failed `ensure` clauses
  default to implementation blame, using blame as repair-routing metadata.
- [Contract Expression Language](2026-05-24-agent-language-spec-wall/result-contract-expression-language.md):
  restrict first-slice contract clauses to pure boolean specification
  expressions rather than arbitrary executable core-language expressions.
- [Contract Predicate Parsing](2026-05-24-agent-language-spec-wall/result-contract-predicate-parsing.md):
  parse contract clauses with a narrow predicate production from the start,
  while keeping ordinary expression-like spelling and semantic validation.
- [Contract Static Runtime Boundary](2026-05-24-agent-language-spec-wall/result-contract-static-runtime-boundary.md):
  statically validate every contract while discharging only conservative local
  obligations and treating valid unknown obligations as runtime-required checks.
- [Runtime Contract Failure Reporting](2026-05-24-agent-language-spec-wall/result-runtime-contract-failure-reporting.md):
  use a structured runtime contract error as the common representation, mapping
  it to a non-zero `run` failure and to a failed test result when it occurs
  inside a selected test case.
- [Effect Declaration Boundary](2026-05-24-agent-language-spec-wall/result-effect-declaration-boundary.md):
  require public functions to declare coarse effects, while allowing private
  helpers to rely on inferred direct and transitive effects.
- [Effect Access Modes](2026-05-24-agent-language-spec-wall/result-effect-access-modes.md):
  keep first-slice public effect declarations coarse and treat read/write
  access modes as advisory diagnostic metadata rather than required syntax.
- [First-Slice Observable I/O](2026-05-24-agent-language-spec-wall/result-first-slice-observable-io.md):
  include built-in stdio output in the first slice as a coarse `stdio` effect,
  while keeping effect handlers internal to the implementation.
- [Stdio API and Output Events](2026-05-24-agent-language-spec-wall/result-stdio-api-and-output-events.md):
  provide four string-only stdio output functions and capture deterministic
  source-linked stdout/stderr events for tests.
- [Transitive Effect Diagnostics](2026-05-24-agent-language-spec-wall/result-transitive-effect-diagnostics.md):
  report missing public effects by coarse label with bounded provenance slices
  instead of full transitive call graphs.
- [Postcondition Result Binding](2026-05-24-agent-language-spec-wall/result-postcondition-result-binding.md):
  use an explicit contract-facing result binding for postconditions instead of
  a magic bare `result` identifier.
- [Hole Satisfy Constraint Grammar](2026-05-24-agent-language-spec-wall/result-hole-satisfy-constraint-grammar.md):
  share the contract predicate grammar for hole `satisfy` constraints, while
  keeping them scoped to a hole-local candidate binding and repair diagnostics.
- [Hole Satisfy Source Syntax](2026-05-24-agent-language-spec-wall/result-hole-satisfy-source-syntax.md):
  attach first-slice `satisfy` predicates with a hole-only suffix,
  `satisfy <candidate> => <predicate>`, where the candidate binding is scoped
  only to that predicate.
- [Satisfy Unknown Severity](2026-05-24-agent-language-spec-wall/result-satisfy-unknown-severity.md):
  report unknown but well-formed `satisfy` predicates as `hint` diagnostics
  during ordinary `check`, but block automatic repair application until hard
  constraints are discharged or explicitly accepted by a future user-confirmed
  workflow.
- [Module Metadata Location](2026-05-24-agent-language-spec-wall/result-module-metadata-location.md):
  keep package and tool metadata in a manifest, compiler-semantic module
  metadata in source, and report duplicated facts as metadata drift.
- [First-Slice Module Fields](2026-05-24-agent-language-spec-wall/result-first-slice-module-fields.md):
  require only machine-checkable module boundary fields in the first slice:
  identity, source imports or dependencies, public API boundaries, and public
  function effects; leave purpose, invariants, examples, tests, and decisions
  optional.
- [ADR-Lite Decision Location](2026-05-24-agent-language-spec-wall/result-adr-lite-decision-location.md):
  use optional structured source documentation comments as the canonical
  ADR-lite record, with generated decision docs as derived views rather than
  first-slice language syntax.
- [Doctest Result Propagation](2026-05-24-agent-language-spec-wall/result-doctest-result-propagation.md):
  allow `?` in executable doctest examples only through a result-returning
  doctest context, inferring one local error type when unambiguous and
  requiring an explicit doctest error type otherwise.
- [Doctest Error Type Fence Syntax](2026-05-24-agent-language-spec-wall/result-doctest-error-type-fence-syntax.md):
  write an explicit doctest error type as a fenced-code info-string attribute,
  `error=<TypePath>`, on the executable Veln doctest block.
- [Doctest Expected Output Syntax](2026-05-24-agent-language-spec-wall/result-doctest-expected-output-syntax.md):
  compare stdout and stderr with adjacent `veln-output stream=...` fences
  attached to the immediately preceding executable Veln doctest block.
- [AST Phase Boundary](2026-05-24-agent-language-spec-wall/result-ast-phase-boundary.md):
  use a source-backed surface AST with stable node IDs, plus phase-specific
  analysis tables for holes, contracts, effects, public boundaries, and
  diagnostics.
- [AST Implementation Representation](2026-05-24-agent-language-spec-wall/result-ast-implementation-representation.md):
  use arena-allocated source nodes with session-stable `NodeId` handles and
  phase-specific side tables, deferring phase-parameterized AST machinery.
- [Check JSON Details Fields](2026-05-24-agent-language-spec-wall/result-check-json-details-fields.md):
  require small always-present prototype `details` payloads for parse, type,
  contract, effect, and hole diagnostics, keyed by phase, node identity,
  expected/actual facts, recovery or provenance evidence, and repair context.
- [Safe Repair Candidate Boundary](2026-05-24-agent-language-spec-wall/result-safe-repair-candidate-boundary.md):
  treat `safe repair` initially as an unapplied, machine-readable candidate
  with reason, evidence, limits, and verification hints, not as automatic edit
  application or a correctness guarantee from passing tests.
- [Scoping and Name Resolution](2026-05-24-agent-language-spec-wall/result-scoping-and-name-resolution.md):
  use lexical scope with explicit namespaces, reject same-scope duplicate
  declarations, resolve nearest lexical value declarations deterministically,
  and keep named-hole labels outside semantic name resolution.
- [First-Slice Prelude Helpers](2026-05-24-agent-language-spec-wall/result-first-slice-prelude-helpers.md):
  allow first examples and golden tests to rely on a small prefix-named prelude
  for immutable container updates, ordinary traversal, `Option`/`Result`
  composition, and `list_try_map` as the concrete fallible traversal helper.
- [Prelude Complexity Guarantees](2026-05-24-agent-language-spec-wall/result-prelude-complexity-guarantees.md):
  keep first-slice prelude helper complexity non-normative until concrete
  persistent container representations are chosen.
- [Comparison Example Task](2026-05-24-agent-language-spec-wall/result-comparison-example-task.md):
  use one dependency-free line-item order summary task for the first comparison
  examples across Ruby, Python, TypeScript, Elixir, Rust, and Veln.

## History

This page used to contain the full design discussion and all discussion result
bodies. It now acts as the stable entry point so agents can choose the smallest
relevant context file first.
