# First-Slice Gap Review

Scope:

- [First-Slice Implementation Memo](../phases/first-slice-implementation.md)
- [Agent-Language Spec Wall Proposals](../proposals/agent-language-spec-wall/README.md)
- [Implemented Source Decisions](../reference/source-decisions/README.md)
- Current Rust workspace and sample commands

## Summary

The crate scaffold broadly matches the first-slice module shape, and the
standard edit-loop commands exist. The implementation is not yet complete for
the documented first-slice gate. The main gaps are shared pipeline consistency,
runtime contract enforcement, first-slice grammar and prelude coverage, test
module ownership, and captured stdio event fidelity.

Treat this review as the current correction list before relying on the
completion claims in
[First-Slice Implementation Memo](../phases/first-slice-implementation.md).

## Implemented Shape

- The workspace has the intended first-slice crates:
  `veln-cli`, `veln-source`, `veln-syntax`, `veln-ast`, `veln-project`,
  `veln-sema`, `veln-core`, `veln-ir`, `veln-backend-jvm`,
  `veln-diagnostics`, and `veln-test`.
- `veln fmt`, `veln check --json`, `veln run <entry>`, and
  `veln test [--json]` exist.
- Source discovery, hand-written lexing/parsing, lossless syntax, AST
  `NodeId`s, JSON diagnostics, basic semantic diagnostics, checked core,
  typed IR, Java source generation, and JVM execution paths are present.
- `stdio::print`, `stdio::println`, `stdio::eprint`, and
  `stdio::eprintln` are recognized as compiler-known calls with the `stdio`
  effect.

## Gaps To Fix

### 1. Shared Analysis Pipeline

Status: partially resolved
Severity: high

Expected:

- `check`, `run`, and `test` share one project context and one analysis
  pipeline.
- `run` and `test` reuse checked facts instead of inventing command-specific
  parsing, discovery, or diagnostics.

Observed:

- `check` now combines parse-clean selected files into one surface module before
  semantic analysis, while retaining parse diagnostics from files that could not
  be lowered.
- `run` and `test` load all parse-clean files into one `SurfaceModule`, then
  run a separate module-level analysis and lowering path.
- `SurfaceModule` currently carries functions only; module declarations, use
  declarations, import roots, and source module identity are not retained in the
  shared AST boundary.

Why it matters:

- Multi-file name resolution and import behavior can differ between `check` and
  `run`/`test`.
- A program can plausibly pass one command and fail another for reasons outside
  the documented phase boundary.

Remaining fix direction:

- Add one shared project analysis entry point that returns parse diagnostics,
  surface modules, semantic diagnostics, checked core, and typed IR readiness.
- Make `check`, `run`, and `test` call that shared entry point directly, then
  apply only command-specific output and execution policy.
- Preserve source module/import facts in the project or AST boundary before
  cross-file resolution grows further.

Acceptance checks:

- A multi-file `check --json` fixture where one file calls an imported function
  in another file resolves without isolated-file name diagnostics.
- A fixture with a parse error in one file and semantic facts in another keeps
  the parse-clean file's semantic diagnostics.

### 2. Runtime Contract Enforcement

Severity: high

Expected:

- Valid unknown contract obligations are classified as runtime-required rather
  than ignored.
- `run` and `test` enforce runtime-required `require` and `ensure` clauses at
  function boundaries.
- Runtime contract checks are lowered from validated contract facts, not
  reparsed from source text.

Observed:

- Contract validation diagnostics exist for some invalid predicates.
- Checked core stores contract text, but typed IR has no contract-check node and
  the JVM backend has no runtime contract failure path.
- No evidence shows runtime-required obligations being enforced during `run` or
  `test`.

Why it matters:

- A checked program with valid but not statically discharged contracts can
  execute without enforcing the documented contract boundary.

Fix direction:

- Add contract obligation records to checked facts with classification such as
  `proven`, `disproven`, and `runtime_required`.
- Lower runtime-required obligations into checked core and typed IR.
- Add JVM runtime contract failure objects and map failures to non-zero `run`
  exits and failed test cases.

Acceptance checks:

- A function with a runtime-required `require` that fails at execution should
  fail `veln run`.
- A selected test case that violates a runtime-required contract should become
  a failed test case with structured failure details.

### 3. First-Slice Grammar Coverage

Status: resolved
Severity: medium

Expected:

- The first-slice grammar includes `match` expressions and patterns.
- Hole expected-type flow includes match branches once match exists.

Observed:

- `match` is represented through parser, AST, checked core, typed IR, and the
  JVM backend for the implemented first-slice pattern subset.
- Match arm expected-type flow and constructor payload bindings are covered by
  semantic checks.

Why it matters:

- First-slice grammar examples and diagnostics can rely on the implemented
  `match` subset.

Fix direction:

- Keep future pattern extensions behind explicit reference updates.

Acceptance checks:

- Parser, semantic, IR, backend, and executable gate coverage should include
  the implemented `match` subset.

### 4. Prelude Helper Coverage

Status: resolved

Expected:

- First-slice prelude helpers are compiler-known ordinary functions with type
  shape, value semantics, effect metadata, and diagnostic behavior.
- Required names include list, dictionary, `Option`, and `Result` helpers such
  as `list_try_map`, `dict_get`, `option_map`, and `result_and_then`.

Observed:

- The required helper names are compiler-known calls with semantic signatures,
  typed IR targets, and JVM runtime helpers.
- Generic helper signatures use `unknown` type arguments where the local
  expected type does not identify a concrete type yet.

Why it matters:

- Repair-loop examples can now use `list_try_map` as a first-class first-slice
  feature.

Implemented direction:

- The semantic prelude signature table is separate from the JVM backend.
- Typed IR carries a prelude builtin call target without exposing backend
  layout.
- Runtime semantics cover list, dictionary, `Option`, and `Result` helpers.

Acceptance checks:

- `check --json` accepts a `list_try_map(lines, parse_line)` example with the
  intended `Result(List(T), E)` shape.
- Wrong callback result types produce structured type diagnostics through the
  normal call-argument path.

### 5. `veln-test` Crate Boundary

Severity: medium

Expected:

- `veln-test` owns test discovery, test JSON, and captured events.

Observed:

- `veln-test` is a placeholder crate.
- Discovery, run reporting, test JSON, and captured stdio event construction
  live in the CLI command implementation.

Why it matters:

- The CLI currently owns behavior that should be reusable and testable as a
  library boundary.

Fix direction:

- Move test selection, report structs, JSON rendering, and event construction
  into `veln-test`.
- Keep CLI code responsible for argument parsing, process exit behavior, and
  human output only.

Acceptance checks:

- `veln-cli` calls a `veln-test` API to build reports.
- Unit tests for report JSON and selection rules live with `veln-test`.

### 6. Captured Stdio Event Fidelity

Severity: medium

Expected:

- `veln test` captures each output operation as a structured event.
- Events preserve operation names such as `println`, terminator values such as
  `newline`, monotonic sequence, call `node_id`, and the call span.

Observed:

- Test events are reconstructed from process-level stdout/stderr after
  execution.
- Each nonempty stream becomes one event.
- Events currently use `operation: "print"` and `terminator: "none"` even for
  `println` and `eprintln`.
- Event `node_id` and span point to the test function, not the stdio call.

Why it matters:

- Agents cannot reliably map output back to the source operation that produced
  it.

Fix direction:

- Route stdio through an implementation-owned capture handler instead of
  reconstructing events from process output.
- Lower stdio calls with enough source metadata for the test handler to emit
  source-linked events.

Acceptance checks:

- A test with two stdout calls and one stderr call emits three events in source
  execution order.
- `println` and `eprintln` events use `terminator: "newline"`.
- Event spans point at the call expression.

### 7. Executable Blockers Missing From `check`

Status: partially resolved

Severity: medium

Expected:

- `check` is the primary repair-loop command and should surface blockers that
  will prevent `run` or `test`.

Observed:

- Some blockers, such as call arity mismatch, are represented during lowering
  as core blockers.
- `check` lowers parse-clean, semantically error-free files far enough to report
  missing-expression, call-arity, and constructor-arity blockers as stable
  diagnostics.
- Broader shared-pipeline behavior remains follow-up work.

Why it matters:

- The repair loop is weaker if an agent must run executable commands to
  discover static blockers.

Remaining fix direction:

- Have the shared analysis entry point include checked-core readiness.
- Keep future executable blockers as stable diagnostics for `check --json`, or
  document a separate non-error readiness section if they remain distinct.

Acceptance checks:

- A missing tail expression in a parse-clean file produces a stable diagnostic
  in `check --json`.
- Calling a known function with the wrong arity produces a stable diagnostic in
  `check --json`.
- `run` and `test` reuse the same diagnostic instead of creating a different
  message.

## Verification Notes

Commands run during this review:

```text
cargo test
timeout 10s cargo run -q -p veln-cli -- check --json samples/demo
timeout 10s cargo run -q -p veln-cli -- test --json samples/demo
```

Results:

- `cargo test` passed.
- `veln check --json samples/demo` returned within the timeout with exit code
  1 and reported the expected demo diagnostics.
- `veln test --json samples/demo` returned within the timeout with exit code 1
  and reported a blocked run due to static gate diagnostics.

The previous `veln check` hang was not reproduced by this command.

## Documentation Follow-Up

- Revisit the "Current Implementation Position" section in
  [First-Slice Implementation Memo](../phases/first-slice-implementation.md)
  after the gaps above are fixed.
- Until then, avoid treating the existing "items 1 through 9 are complete"
  wording as the current implementation status.
