# User Function Tail Recursion Trampoline

Status: implemented

This page records the completed direct self-recursive tail-call execution
route. Current behavior is specified under `../../specification/`; this page
keeps the proposal history, completion evidence, and follow-up boundary.

## Read First

- Current execution boundary and JVM behavior:
  [../../specification/execution.md](../../specification/execution.md).
- Current source function and tail-expression syntax:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current function values, call resolution, and effects:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Narrow list helper stack-safety route:
  [iterative-list-helper-runtime.md](iterative-list-helper-runtime.md).

## Implemented Outcome

User-defined `fn` declarations whose direct self-recursive calls are all in
tail position execute deep self-recursive chains without consuming one host
stack frame per logical recursive step. The implementation is automatic and
syntax-free: no `tailrec` keyword, annotation, warning, or command JSON field
was added.

The JVM backend keeps typed IR runtime-neutral and lowers eligible direct tail
self calls to a backend-owned loop. Each step evaluates the next call
arguments before rebinding parameters, then re-enters the function boundary so
runtime `require` checks run for each logical invocation.

Functions with runtime `ensure` or `invariant` clauses stay on ordinary call
lowering because runtime `invariant` clauses include return checks. Non-tail
recursion, mutual recursion, indirect recursion through function-typed values,
and calls whose self target is not a direct resolved function call do not
receive the stack-safety guarantee.

## Original Target

The original proposal asked for stack-safe execution of direct tail-recursive
user functions without adding source syntax. Tail position included the final
function expression and nested `match` arm result expressions under a
tail-position `match`. The proposal also required runtime behavior, effects,
diagnostics, and command JSON to remain otherwise unchanged.

## Acceptance Review

- Deep direct tail recursion runs through `veln run` without host stack
  overflow.
- Tail-position self calls under nested `match` arm results are stack-safe.
- Non-tail recursive calls keep ordinary call lowering and receive no
  stack-safety guarantee.
- Recursive-call arguments are evaluated before parameter rebinding.
- Runtime `require` checks run for each logical recursive invocation.
- Runtime `ensure` and runtime `invariant` clauses exclude a function from
  this route.
- Existing diagnostics and command JSON for static blockers keep their current
  anchors because eligibility does not add diagnostics or output fields.

## Evidence

- `crates/veln-backend-jvm/src/classfile.rs` classifies eligible functions and
  lowers tail self calls without exposing trampoline classes as language
  behavior.
- `crates/veln-backend-jvm/src/tests.rs` covers deep tail recursion, nested
  match tail recursion, argument evaluation before rebinding, per-step
  `require` checks, and conservative eligibility exclusions.
- `examples/specification/run/tail-recursion-trampoline/` covers observable
  `veln run --json` behavior for deep eligible recursion.
- `examples/specification/check/recursive-call-shapes/` covers checked source
  shapes for direct, nested, non-tail, postcondition-bearing, and indirect
  recursive calls.

## Follow-Ups

- Optional source-level assertions for required tail recursion remain
  unimplemented.
- Mutual-recursion stack safety remains unimplemented.
- Runtime return-contract support for stack-safe recursion remains
  unimplemented.
- Tooling output that exposes eligibility remains unimplemented.

## Update When

- Eligibility, stack-safety guarantees, or return-contract support changes.
- Source-level tail-recursion assertions are added, rejected, or superseded.
- JVM lowering stops being the backend strategy for this behavior.
