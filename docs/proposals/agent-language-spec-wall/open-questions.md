# Agent-Oriented Language Spec Wall Open Questions

Status: open-proposal

This file keeps the unresolved question set separate from decision records.
When a question is resolved, keep only the short pointer here and put the
decision body in a `result-<topic-slug>.md` file.

## Implementation Readiness

These questions should be resolved before the first slice is implemented, or
the parser, checker, runtime, and golden diagnostics will need early rework.

- Resolved by [AST Phase Boundary](../../reference/source-decisions/result-ast-phase-boundary.md): use a
  source-backed surface AST with stable node IDs, plus phase-specific analysis
  tables for holes, contracts, effects, public boundaries, and diagnostics.
- Resolved by
  [AST Implementation Representation](../../reference/source-decisions/result-ast-implementation-representation.md):
  implement that boundary with arena-allocated source nodes, session-stable
  `NodeId` handles, and phase-specific side tables rather than a
  phase-parameterized AST in the first slice.
- Resolved by [Check JSON Details Fields](../../reference/source-decisions/result-check-json-details-fields.md):
  first-slice parse, type, contract, effect, and hole diagnostics should use
  small always-present prototype `details` payloads keyed by phase, node
  identity, expected/actual facts, recovery or provenance evidence, and repair
  context.
- Resolved by [Contract Predicate Parsing](../../reference/source-decisions/result-contract-predicate-parsing.md):
  parse `require`, `ensure`, and future `invariant` clauses through a narrow
  contract predicate production from the start, while keeping ordinary
  expression-like spelling and later semantic validation.
- Resolved by
  [Runtime Contract Failure Reporting](result-runtime-contract-failure-reporting.md):
  use a structured runtime contract error as the common representation; map it
  to a non-zero `run` failure and to a failed test result when it occurs inside
  a selected test case, preserving contract blame metadata.
- Resolved by
  [Hole Satisfy Source Syntax](../../reference/source-decisions/result-hole-satisfy-source-syntax.md): attach
  first-slice `satisfy` predicates with a hole-only suffix,
  `satisfy <candidate> => <predicate>`, where the candidate binding is scoped
  only to that predicate.
- Resolved by [Satisfy Unknown Severity](../../reference/source-decisions/result-satisfy-unknown-severity.md):
  unknown but well-formed `satisfy` predicates remain `hint` diagnostics during
  ordinary `check`, while automatic repair application is blocked until hard
  constraints are discharged or explicitly accepted by a future user-confirmed
  workflow.
- Resolved by
  [Minimal Project and Test Discovery](../../reference/source-decisions/result-minimal-project-test-discovery.md):
  `check`, `run`, and `test` should share one project context based on
  explicit targets, source-relative local imports, explicit run entries, and
  conservative test discovery before manifests and `graph` exist. This
  reconciles
  [First Implementation Commands](../../reference/source-decisions/result-first-implementation-commands.md),
  [Affected Test Selection](../../reference/source-decisions/result-affected-test-selection.md), and
  [Module Metadata Location](result-module-metadata-location.md).
- Resolved by
  [Stdio API and Output Events](../../reference/source-decisions/result-stdio-api-and-output-events.md): provide
  four string-only stdio output functions and capture deterministic
  source-linked stdout/stderr events for tests.
- Resolved by
  [Scoping and Name Resolution](result-scoping-and-name-resolution.md): use
  lexical scope with explicit namespaces, reject duplicate declarations in the
  same scope and namespace, resolve nearest lexical value declarations
  deterministically, and keep named-hole labels outside semantic name
  resolution.
- Resolved by
  [First-Slice Prelude Helpers](../../reference/source-decisions/result-first-slice-prelude-helpers.md): first
  examples and golden tests may rely on a small prefix-named prelude for
  immutable list and dictionary updates, ordinary traversal, `Option`/`Result`
  composition, and `list_try_map` as the concrete fallible traversal helper.
- Resolved by
  [Prelude Complexity Guarantees](../../reference/source-decisions/result-prelude-complexity-guarantees.md):
  first-slice prelude helpers specify value semantics, source-order traversal,
  and `Result` short-circuiting, but leave asymptotic complexity
  non-normative until concrete container representations are chosen.

## Comparative Evaluation

- Resolved by [Comparison Example Task](result-comparison-example-task.md): the
  first comparison examples across Ruby, Python, TypeScript, Elixir, Rust, and
  Veln should use one dependency-free line-item order summary task that
  exercises parsing, validation, `Result`, collection traversal, tests, stdout,
  and Veln typed-hole diagnostics.

## Surface Syntax

- Resolved by [Block Structure](../../reference/source-decisions/result-block-structure.md): first-slice blocks
  should use explicit keyword delimiters closed by `end`; indentation is
  formatting, braces are not statement-block delimiters, and `do` is restricted
  to forms that need an explicit body separator.
- Resolved by [Compact Function Form](../../reference/source-decisions/result-compact-function-form.md): named
  single-expression functions should not get a separate compact syntax in the
  first slice; named `fn` bodies always close with `end`.
- Resolved by [Pipeline Style](../../reference/source-decisions/result-pipeline-style.md): pipeline should be
  preferred only for multi-step data flow where the intermediate value is the
  main subject; plain calls remain the default for simple composition.
- Resolved by [Method Call Boundary](../../reference/source-decisions/result-method-call-boundary.md): use
  function calls as the only general first-slice call form; delay method calls
  and report targeted diagnostics for method-call-shaped syntax when possible.
- Resolved by [First-Slice Grammar](result-first-slice-grammar.md): the first
  slice should use a small line-oriented, keyword-delimited,
  expression-centered grammar with explicit public signatures, public effect
  declarations, records, lists, `match`, holes, plain and qualified calls,
  pipelines, and `end`-closed blocks.
- Resolved by [Test Declaration Syntax](../../reference/source-decisions/result-test-declaration-syntax.md):
  user-authored executable test cases should use a dedicated top-level `test`
  declaration instead of being inferred from ordinary zero-argument `fn`
  declarations.

## Types and Inference

- Resolved by
  [Minimum Type System for Holes](../../reference/source-decisions/result-minimum-type-system-for-holes.md):
  use a small local, mostly monomorphic type system with built-in parametric
  `Option`, `Result`, collection, record, and function types; defer user
  generics, traits, subtyping, implicit conversions, and generalized
  let-polymorphism.
- Resolved by
  [User-Defined ADTs in the First Slice](../../reference/source-decisions/result-user-defined-adts-first-slice.md):
  user-defined ADTs are deferred; the first slice starts with built-in
  `Result`, `Option`, records, lists, dictionaries, function types, primitive
  types, and opaque named types for signatures and diagnostics.
- Resolved by
  [Public Function Type Boundaries](../../reference/source-decisions/result-public-function-type-boundaries.md):
  public functions should require explicit parameter and return types from the
  beginning; private helpers may rely on inference.
- Resolved by [Error Type Inference](../../reference/source-decisions/result-error-type-inference.md): public
  `Result` return types must explicitly name their error type; private helpers
  may infer one concrete propagated error type, but mixed incompatible errors
  require explicit conversion or an explicit helper return type.

## Runtime and Memory

- Resolved by
  [First-Slice Value Mutability](../../reference/source-decisions/result-first-slice-value-mutability.md):
  first-slice bindings and aggregate values should be immutable, container
  updates should produce new values, and the language should require automatic
  memory management without specifying a concrete GC strategy.
- Resolved by
  [First Implementation Runtime Targets](../../reference/source-decisions/result-first-implementation-runtime-targets.md):
  lower runnable code through a backend-neutral typed IR, use the JVM as the
  first reference execution target, and keep Node-hosted WebAssembly
  experimental with JavaScript glue instead of an early custom Veln heap.
- Resolved by
  [First Implementation Architecture](../../reference/source-decisions/result-first-implementation-architecture.md):
  implement the first CLI, frontend, typed IR, and initial JVM lowering in
  Rust, pair it with a small Java or Kotlin JVM runtime library, and defer a
  separate Kotlin JVM backend module until the typed IR has enough examples.
- Resolved by
  [Channel-First Concurrency Runtime](result-channel-first-concurrency-runtime.md):
  use a parallel-capable runtime without a global interpreter lock, prefer
  structured tasks and bounded MPSC channels for concurrent data flow, and
  expose concurrency at public boundaries through a coarse effect label.
- Resolved by
  [Runtime Value Freeze Boundary](../../reference/source-decisions/result-runtime-value-freeze-boundary.md):
  ordinary Veln values must be frozen, transitively immutable, and safely
  published before they cross task or channel boundaries; backend-owned
  resources require explicit send-safety metadata.

## Contracts

- Resolved by
  [Contract Expression Language](../../reference/source-decisions/result-contract-expression-language.md):
  `require`, `ensure`, and future `invariant` clauses should use a restricted
  pure boolean expression subset rather than arbitrary executable core-language
  expressions.
- Resolved by
  [Contract Static Runtime Boundary](result-contract-static-runtime-boundary.md):
  all contracts are statically validated and exposed to diagnostics, but only a
  conservative subset is statically discharged; valid unknown obligations become
  runtime-required checks.
- Resolved by [Contract Blame Boundary](../../reference/source-decisions/result-contract-blame-boundary.md):
  failed `require` clauses default to caller blame and failed `ensure` clauses
  default to implementation blame, with blame treated as repair-routing
  metadata rather than proof of fault.
- Resolved by
  [Postcondition Result Binding](../../reference/source-decisions/result-postcondition-result-binding.md):
  postconditions should use an explicit result binding in the return type
  position rather than a magic bare `result` name.

## Effects

- Resolved by [Effect Declaration Boundary](../../reference/source-decisions/result-effect-declaration-boundary.md):
  public functions must declare coarse effects; private helpers may omit effect
  annotations and rely on inference, with diagnostics reporting undeclared
  public-boundary effects.
- Resolved by [Effect Access Modes](../../reference/source-decisions/result-effect-access-modes.md): first-slice
  public effect declarations should stay coarse; access modes such as database
  read/write may appear only as advisory diagnostic metadata.
- Resolved by
  [First-Slice Observable I/O](../../reference/source-decisions/result-first-slice-observable-io.md): first-slice
  programs should have built-in stdio output under a coarse `stdio` effect, with
  source-level effect handler syntax deferred and internal operation/handler
  representation used by `run` and `test`.
- Resolved by
  [Transitive Effect Diagnostics](result-transitive-effect-diagnostics.md):
  transitive effects should be grouped by missing coarse label and displayed as
  bounded provenance slices rather than full transitive call graphs.

## Typed Holes

- Resolved by [Named Hole Syntax](../../reference/source-decisions/result-named-hole-syntax.md): first-slice
  syntax should support both anonymous `_` holes and named expression holes
  such as `_config_parser`, with names acting as diagnostic and repair labels
  rather than variable bindings.
- Resolved by
  [Hole Satisfy Constraint Grammar](../../reference/source-decisions/result-hole-satisfy-constraint-grammar.md):
  `satisfy` constraints on holes should share the contract predicate grammar,
  while using a hole-local candidate binding and repair-diagnostic semantics.
- Resolved by [Hole Runtime Boundary](../../reference/source-decisions/result-hole-runtime-boundary.md):
  files with holes are always checkable, but `run` should execute only when
  holes are outside the selected entry point's conservative reachable code.
- Resolved by
  [Hole Diagnostic JSON Shape](../../reference/source-decisions/result-hole-diagnostic-json-shape.md):
  hole diagnostics should use the stable envelope with `kind: "hole"` and a
  prototype `details` payload containing repair context.

## Toolchain

- Resolved by
  [First Implementation Commands](../../reference/source-decisions/result-first-implementation-commands.md):
  require `check`, `fmt`, `run`, and `test` in the first implementation; defer
  `doc`, `graph`, `explain`, and `repair`.
- Resolved by [Test JSON Shape](../../reference/source-decisions/result-test-json-shape.md): `veln test --json`
  should emit one run-level native JSON result with deterministic summary
  counts, top-level gate diagnostics, suite errors, per-case records, and
  captured events.
- Resolved by [Test Declaration Syntax](../../reference/source-decisions/result-test-declaration-syntax.md):
  `veln test` should select explicit `test` declarations and executable
  doctest examples, not ordinary `fn` declarations by arity alone.
- Resolved by [Primary Check Command](../../reference/source-decisions/result-primary-check-command.md):
  `check` should be the primary read-only agent command that combines parse,
  type, contract, effect, lint, doc drift, and hole diagnostics.
- Resolved by
  [Safe Repair Candidate Boundary](result-safe-repair-candidate-boundary.md):
  `safe repair` should initially mean an unapplied, machine-readable candidate
  with reason, evidence, limits, and verification hints, while automatic
  application waits for explicit gates and must not treat passing tests alone
  as a correctness guarantee.
- Resolved by
  [JSON Diagnostic Schema Stability](../../reference/source-decisions/result-json-diagnostic-schema-stability.md):
  JSON diagnostics should use a stable top-level envelope while allowing
  prototype `details` payloads to churn.
- Resolved by
  [Affected Test Selection](../../reference/source-decisions/result-affected-test-selection.md): before a full
  dependency graph exists, automatic affected-test selection should prefer
  false positives over false negatives, widen when evidence is incomplete, and
  report selection confidence.

## Module and Documentation Model

- Resolved by [Module Metadata Location](result-module-metadata-location.md):
  module metadata should live in both source and a package manifest, with
  package/tool metadata owned by the manifest, compiler-semantic module
  metadata owned by source, and duplicated facts rejected as drift.
- Resolved by
  [First-Slice Module Fields](../../reference/source-decisions/result-first-slice-module-fields.md):
  require
  only machine-checkable boundary fields in first-slice source modules: module
  identity, source-level imports or module dependencies, public API boundaries,
  and public function effect declarations; keep purpose, invariants, examples,
  tests, and ADR-lite decisions optional.
- Resolved by
  [ADR-Lite Decision Location](result-adr-lite-decision-location.md):
  ADR-lite decisions should be optional structured source documentation
  comments attached to modules, package documentation, or public API
  declarations, with generated docs as derived views rather than canonical
  language syntax.
- Resolved by
  [Doctest Result Propagation](result-doctest-result-propagation.md):
  executable doctest examples may use `?` only inside a result-returning
  doctest context; the first slice should infer one local error type when
  unambiguous and otherwise require an explicit doctest error type.
- Resolved by
  [Doctest Error Type Fence Syntax](result-doctest-error-type-fence-syntax.md):
  write an explicit doctest error type as a fenced-code info-string attribute,
  `error=<TypePath>`, on the executable Veln doctest block.
- Resolved by
  [Doctest Expected Output Syntax](result-doctest-expected-output-syntax.md):
  compare stdout and stderr with adjacent `veln-output stream=...` fences
  attached to the immediately preceding executable Veln doctest block.
