---
role: proposal
update-when: The proposed literal-expression folding scope, integer evaluation semantics, diagnostic preservation rules, or compiler acceptance gates change.
---

# Literal-expression Constant Folding

Support compile-time folding of closed `Int` and `Bool` operator expressions
in executable function bodies. For example, `1 + 2 * 3` produces the value
`7` without runtime arithmetic calls for that expression. This reduces work
in generated code without adding source syntax or making source acceptance
depend on optimization.

This proposal is design work, not implemented behavior. Promotion to Ready
requires the feasibility evidence in [Delivery gates](#delivery-gates).

## Current boundary

The [operator specification](../specification/types.md#operators-and-diagnostics)
defines typing, fixed-width bit operations, and failure boundaries. The
[execution specification](../specification/execution.md)
owns runtime behavior. Ordinary binary expressions currently survive
core lowering (`crates/veln-sema/src/lowering/expression_lowering.rs`),
IR lowering (`crates/veln-ir/src/lowering.rs`), and
JVM emission (`crates/veln-backend-jvm/src/classfile/pattern_and_value_emission.rs`)
as runtime operations. The
runtime operators (`crates/veln-backend-jvm/src/runtime/operators.java.inc`)
use Java `long` arithmetic for integer operations.

The contract evaluator (`crates/veln-sema/src/contracts/static_literals.rs`)
already evaluates some literal arithmetic for proof and repair reasoning.
It also supports exact-number and rational reasoning. That proof domain is
not the execution domain: reusing it as an optimizer without an explicit
fixed-width interpretation could change results.

The proposed [numeric constants API](numeric-constants.md) and
[exponent literals](floating-point-literals.md) are independent work. Folding
does not admit arithmetic in module-value initializers or supply a construction
mechanism for those constants. Their admission and representation gates remain
in their own proposals.

## Proposed contract

### Eligible expressions

Folding applies by default to executable bodies compiled for `run` and `test`,
including local initializers, arguments, returned values, and expressions inside
control-flow and cleanup regions. There is no new command flag or `const`
keyword. The compiler checks the original source before folding. Formatting,
source documentation, name resolution, type checking, effect checking, hole
blocking, and contract proof outcomes retain their existing source behavior.

A closed expression contains only admitted literal leaves and the operators
below. Parentheses preserve eligibility. Operand and result types are the
resolved execution types; an integer-looking literal in a selected `Float`
operation is not sufficient evidence for integer folding.

| Operands | Operators | Result |
| --- | --- | --- |
| `Int` | unary `-`, `~` | `Int` |
| `Int`, `Int` | `+`, `-`, `*`, `/`, `%`, `&`, `^`, `\|`, `<<`, `>>`, `>>>` | `Int` |
| `Int`, `Int` | `==`, `!=`, `<`, `<=`, `>`, `>=` | `Bool` |
| `Bool` | `not` | `Bool` |
| `Bool`, `Bool` | `and`, `or`, `==`, `!=` | `Bool` |

Every successfully evaluable closed subtree in this domain is replaced by a
constant in generated executable code. An enclosing dynamic expression remains
dynamic. For `x + (2 * 3)`, the multiplication disappears and the addition
remains. For `let x = 2; x + 3`, this proposal does not propagate the value of
`x`. Traversing a call argument does not authorize evaluating the call.

The initial scope excludes:

- Float values, mixed numeric operations, and cross-type equality;
- string operations, aggregate construction folding, and schema expressions;
- local or module reference propagation, field projection, and function calls;
- algebraic identities involving dynamic operands, reassociation, branch
  elimination, and compile-time execution of functions;
- changes to textual contract predicates or the static proof engine.

These boundaries keep this delivery independent of data-flow analysis and of
the proposed typed predicate facts. Float folding needs separate decisions for
rounding, signed zero, NaN, and representation before extending this domain.

### Integer evaluation and failures

The following evaluation model is normative for the proposed optimizer. It
matches the existing JVM integer runtime and makes future backends use the
same results. It does not expand the source literal range.

| Operation | Evaluation rule |
| --- | --- |
| `+`, `-`, `*`, unary `-` | Retain the low 64 bits and interpret them as a signed two's-complement integer. |
| `/` with nonzero divisor | Truncate toward zero. Minimum `Int` divided by `-1` produces minimum `Int`. |
| `%` with nonzero divisor | Signed remainder with the dividend's sign; minimum `Int` remainder by `-1` is zero. |
| `&`, `^`, `\|`, `~` | Operate on all 64 bits. |
| `<<`, `>>`, `>>>` | Accept only counts `0..63`; use the existing discard, sign-extension, and zero-fill rules. |
| Integer ordering | Compare signed values. |
| Same-type equality and Boolean operations | Produce the existing scalar runtime truth value. |

If a node would fail at runtime, folding leaves that operation executable.
It may fold successful children, but it does not introduce a compile-time
diagnostic, throw from the compiler, or remove the failure. A failing child
prevents folding its ancestors. Division or remainder by computed zero and a
computed invalid shift count retain their original runtime failure behavior.
Existing source diagnostics for literal zero remainder divisors and literal
invalid shift counts still arise from the original source.

The distinction is intentional: `7 % 0` retains its source error, while
`7 % (1 - 1)` retains its runtime failure. Rewriting the latter's child to zero
does not cause the compiler to reclassify it as a source-level literal error.
Unchanged failing operations retain the original operand and operation spans
for human and JSON diagnostics, including related context.

No rewrite removes, duplicates, reorders, or executes dynamic operands.
In particular, `0 * f()`, `false and f()`, and `true or f()` remain dynamic.
Current JVM Boolean binary emission evaluates both arguments; this proposal
does not introduce short-circuit evaluation. Cleanup, earlier output, and
failure timing remain observable in the same order. Both branches of source
control flow remain subject to ordinary checking even with a folded condition.

### Generated-code observation

For a function returning only an eligible closed expression, its emitted
method loads the computed scalar value without calling the runtime helpers
for the folded operators. This requirement concerns the method instructions,
not whether the shared runtime class still contains those helpers. Runtime
output alone cannot demonstrate folding because unfolded execution has the
same output.

Materializing a folded result must preserve every signed 64-bit value,
including minimum `Int`, even when that value cannot be written as one
accepted positive source literal. An internal constant representation must not
round-trip through source-literal admission or silently substitute zero when
parsing a folded value fails.

## Acceptance model and planned verification

The following cases are planned evidence, not tests supplied or passing with
this document. Expected values come from the fixed-width model above and
independently checked vectors, not from the folding implementation itself.

| Case | Required observation | Intended evidence |
| --- | --- | --- |
| Nested arithmetic | `1 + 2 * 3` returns `7`; all three constants reduce to one result with no arithmetic helper call. | Typed-IR assertions and JVM method-instruction inspection. |
| Wrapping | `9223372036854775807 + 1` produces minimum `Int`; negating that computed minimum retains minimum `Int`. | Independent 64-bit vectors, IR and JVM materialization checks. |
| Division and remainder | `-7 / 3` is `-2`; `-7 % 3` is `-1`; computed minimum divided or remaindered by `-1` matches the model. | Table-driven evaluator and backend execution checks. |
| Bits and shifts | `~0` is `-1`, `-1 >>> 63` is `1`, `1 << 63` is minimum `Int`; every admitted operator has a success vector. | Independent bit vectors and emitted-method checks. |
| Boolean results | `(1 + 2 == 3) and not false` is `true`; same-type equality and each ordering operator match the truth table. | Typed-IR constant assertions and backend execution. |
| Dynamic boundary | `x + (2 * 3)` retains only the addition; a local binding reference or ordinary call is not treated as a constant. | IR and emitted-method inspection. |
| Failure timing | Computed zero `/` and `%`, and computed shift counts `-1` and `64`, remain runtime failures; successful children may fold. | Backend tests plus CLI failure projection cases. |
| Diagnostic preservation | Literal invalid shift counts and literal zero `%` keep their IDs and spans; computed `%` zero keeps divisor span, operation related note, prior output, and cleanup. | Checked diagnostic and run JSON examples, including human related-note coverage. |
| Dynamic evaluation | `0 * f()`, `false and f()`, and `true or f()` retain the call exactly once and preserve failure ordering. | Backend tests with a local observable state change or deliberate runtime failure. |
| Source checking | A hole, unresolved name, or type error remains blocking inside a branch with a closed condition. | Checked source diagnostic examples. |
| Excluded domains | Float arithmetic and mixed equality retain their existing behavior and are not folded by this delivery. | IR and JVM checks, including a `Float` result context with integer-looking operands. |
| Bounded compiler work | Doubling independent closed expression trees adds at most proportional evaluator work; deep admitted trees do not add optimizer stack overflow. | Guarded generated-input tests with visit counters and explicit input limits. |

Compiler tests belong beside the chosen core/IR transformation. JVM tests
verify constant materialization and instruction removal. New CLI cases under
`examples/specification/` must target compiler diagnostics, generated-code
integration, or failure projection under the
[test placement policy](../reference/toolchain-test-harness.md).
This proposal adds no stdlib API. Any later stdlib numeric API behavior belongs
in its `crates/veln-stdlib/veln/` companion tests.

Generated-input verification uses `bash scripts/agent-run` or
`bash scripts/agent-test`. Work counters, not elapsed-time thresholds on shared
CI, establish scaling. No runtime speedup percentage is claimed.

## Design direction and alternatives

Perform the transformation on typed executable expressions after semantic
validation and before JVM emission. The exact core/IR ownership is an
implementation choice to settle with the feasibility evidence. Keep checked
source facts available to tools and preserve provenance for residual failures.
Evaluate each candidate with explicit target integer operations; do not rely
on Rust debug overflow behavior or unbounded proof arithmetic.

Two inspected primary sources inform these choices:

- [Java Language Specification, Java SE 25, section 15.29](https://docs.oracle.com/javase/specs/jls/se25/html/jls-15.html#jls-15.29)
  defines a bounded constant-expression language and excludes abrupt
  completion. This supports a closed, successful-expression domain. Veln
  adopts that boundary as optimization eligibility, not Java's additional
  effects on source typing, constant-variable lookup, or string interning.
- [LLVM's Analysis and Transform Passes, SCCP](https://llvm.org/docs/Passes.html#sccp-sparse-conditional-constant-propagation)
  describes constant propagation that also proves branches unconditional.
  This is a broader alternative. Veln defers propagation and branch removal
  so the first delivery can preserve source checking and dynamic evaluation
  without introducing control-flow proof machinery. LLVM behavior is comparative
  evidence, not authority for Veln arithmetic or diagnostics.

Retaining runtime-only operations avoids compiler work but leaves even
literal arithmetic in generated methods. Depending only on JVM JIT optimization
does not satisfy the emitted-code acceptance condition and offers no shared
compiler boundary for later backends. Reusing the current textual predicate
evaluator would couple runtime optimization to a different arithmetic domain.
An unrestricted compile-time interpreter would require termination, resource,
effect, and call-contract policies that this bounded transformation does not
need.

## Delivery gates

Before Ready promotion, demonstrate full-range integer constant materialization
in the current core/IR and JVM backend, and retain computed-failure source
spans after a child fold. These are feasibility gates because the existing
literal-shaped representation and source admission must not constrain runtime
values. Record the selected transformation boundary and confirm all table
rows can be checked with existing harnesses or a focused instruction-inspection
helper.

Implementation completion requires the acceptance evidence above and an audit
of generated-input cost under the performance-regression skill. Promote the
observable folding and preservation rules to the focused execution and operator
specifications with checked examples. Keep proof semantics and module-value
admission unchanged. Remove this proposal and its catalog entry once the
current specification and tests cover the delivered scope. Any extension to
Float, reference propagation, schema expressions, or branch elimination needs
its own evaluation and preservation acceptance cases.
