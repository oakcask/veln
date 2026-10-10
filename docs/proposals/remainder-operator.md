---
role: proposal
update-when: Integer remainder syntax, typing, runtime failures, contract reasoning, or the remainder operator acceptance cases are implemented or revised.
---

# Integer remainder operator

Add `a % b` for integer remainder. This supports divisibility tests and periodic
integer calculations without repeating division and multiplication. This page
specifies planned behavior; `%` is not currently implemented.

## Current boundary and scope

The [operator specification](../specification/types.md#operators-and-diagnostics)
currently supports `+`, `-`, `*`, and `/` with integer and floating-point
selection. The parser and JVM runtime have no remainder operation. The runtime
implements integer division using Java `long` division.

This proposal adds `%` to ordinary expressions and function/test contract
predicates, including `require`, `ensure`, `invariant`, and `satisfy` where
integer arithmetic is already accepted. It includes parsing, formatting,
checking, typed lowering, JVM execution, and operator presentation in editor
and published language-reference surfaces.

The following are outside this target:

- Floating-point remainder and implicit conversion of `%` operands to `Float`.
- Euclidean modulo, a nonnegative-modulo helper, and operator overloading.
- Compound assignment such as `%=`.
- Extension of the separate schema count, length, and validation expression
  languages. Those retain their current accepted operator sets and reject `%`
  with a source diagnostic when unsupported; they must not accept it and fail
  later during lowering or execution.
- Changes to existing division behavior or a general arithmetic error redesign.

## Proposed contract

### Syntax and types

`%` is a binary operator with the same precedence as `*` and `/`. These three
operators associate to the left. Prefix operators bind more tightly; addition
and subtraction bind less tightly. The formatter writes one space on each side
of `%` and preserves parentheses needed to retain the expression tree.

Both operands and the result have type `Int`. Integer literals use the `Int`
context of `%`. A surrounding `Float` context does not select floating-point
remainder or widen the operands. A `Float`, `Bool`, or `String` operand reports
`type.mismatch` at that operand. Assigning the integer result to `Float` uses
the existing assignment mismatch rule. A surrounding numeric operator may
accept the `Int` result under its existing widening rule; it does not change
the type of `%` itself.

Proposed usage:

```veln
fn is_even(value: Int) -> Bool
  value % 2 == 0
end

fn remainder(value: Int, divisor: Int) -> Int
  require divisor != 0
  value % divisor
end
```

These examples are design inputs, not executable evidence yet.

### Numeric result and evaluation

For signed 64-bit integers `a` and nonzero `b`, define the result using
mathematical integers: `q = trunc(a / b)` and `r = a - q * b`, where `trunc`
rounds toward zero. `%` returns `r` as `Int`. The quotient and product in this
model are unbounded mathematical values, not Veln intermediate expressions.
This avoids making overflowing multiplication part of the definition.

A nonzero remainder has the sign of `a`, and `abs(r) < abs(b)`. These absolute
values are also mathematical values, so the rule includes the minimum signed
integer. In particular, the minimum `Int` remainder by `-1` is `0`; it does not
fail with overflow. No nonzero divisor can cause remainder overflow.

Operands evaluate once, left before right. Failure of the left operand prevents
evaluation of the right operand. Failure of the right operand prevents the
remainder operation. A zero-divisor failure preserves completed operand effects
and prior output; it does not evaluate expressions after the failed operation.
Registered cleanup follows the existing runtime failure rules.

### Zero divisor and diagnostics

After removing parentheses and any unary `-` wrappers, a divisor that is an
integer literal with value zero reports `type.remainder_by_zero` at the divisor
expression. This includes `0`, `0x0`, `0b0`, and `(-0)`. The primary message is
`remainder divisor is zero`. A related note identifies the `%` operation span.
This check applies in ordinary expressions and the included contract predicates.

Other divisor expressions are checked at runtime, including a variable holding
zero and `(1 - 1)`. Static reasoning must not convert their failure into a
successful constant result. Execution with a zero divisor fails with a Veln
runtime diagnostic whose primary message is `remainder divisor is zero` at the
divisor expression and whose related note identifies the operation span.
The failure details include `id: "runtime.remainder_by_zero"`,
`operator: "%"`, and integer `divisor: 0`. The public run JSON projection uses
the existing runtime error envelope and preserves these details and captured
output. Human output renders the primary location and related context. A raw
Java exception message is not the public contract.

### Contract reasoning

Contract typing and execution use the same integer-only and zero-divisor rules.
Static reasoning evaluates literal-only `%` subexpressions when their operands
have known signed 64-bit runtime values and the divisor is nonzero. The folded
result must equal the numeric model above, including the minimum-`Int` / `-1`
case. Existing exact rational reasoning for `/` must not be reused as integer
remainder semantics. If an operand's runtime value cannot be established
soundly, leave the predicate runtime-checked.

Nonliteral remainder predicates remain runtime-checked. This proposal does not
add symbolic modular arithmetic or infer an input repair from a congruence.
A `%` predicate alone must not produce an advisory repair that claims to prove
or satisfy it. Existing repairs for independent supported constraints may
remain available, but the remainder predicate must still be checked.

## Acceptance model and planned evidence

The mathematical definition and the following cases are the acceptance
oracle. Tests must compare Veln results with explicit expected values and, for
generated pairs, an independent arbitrary-precision truncating-division model.
Using the backend remainder helper to compute expectations is insufficient.
All evidence in this section is planned.

Let `MIN = -9223372036854775808` and `MAX = 9223372036854775807`. These names are
notation for the table, not proposed prelude constants. Fixtures can construct
`MIN` through accepted source arithmetic such as `-9223372036854775807 - 1`.

| Concern | Input or condition | Required observation | Planned verification boundary |
| --- | --- | --- | --- |
| Signs | `7 % 3`, `-7 % 3`, `7 % -3`, `-7 % -3` | `1`, `-1`, `1`, `-1` | Compiler/backend result tests |
| Exact multiples | `0 % 3`, `6 % 3`, `-6 % 3` | All return `0` | Compiler/backend result tests |
| Endpoints | `MIN % -1`, `MIN % 1`, `MIN % 3`, `MAX % 3` | `0`, `0`, `-2`, `1` | Backend and constant-reasoning tests |
| Large divisor | `1 % MIN`, `MIN % MAX`, `MIN % MIN` | `1`, `-1`, `0` | Backend and independent model tests |
| Associativity | `20 % 6 * 2`, `20 / 3 % 2`, `20 % 6 % 3` | `4`, `0`, `2` | Parser tree and executable compiler tests |
| Precedence | `2 + 7 % 3 * 4`, `(2 + 7) % 3`, `-(7 % 3)` | `6`, `0`, `-1` | Parser and formatter round-trip tests |
| Types | `7 % 3`, `7.0 % 3`, `7 % 3.0`, `true % 2`, `7 % "3"` | First has `Int`; remaining cases report operand mismatches | Semantic diagnostic tests |
| Expected type | `let x: Float = 7 % 3` | Assignment mismatch; `%` remains integer-only | Semantic diagnostic tests |
| Literal zero | `7 % 0`, `7 % (-0)`, `7 % 0x0`, `7 % 0b0` | `type.remainder_by_zero` with divisor span and related operation | Check JSON and human diagnostic fixtures |
| Computed zero | `7 % (1 - 1)` and `value % divisor` with a zero-valued parameter | Runtime failure with the specified details; no host exception leakage | Run JSON and human diagnostic fixtures |
| Evaluation | Two operand calls record their order; left fails, right fails, or right returns zero | Each reached operand runs once; later evaluation stops at failure; prior output survives | Compiler/backend evaluation and CLI output fixtures |
| Cleanup | A dynamic zero failure inside a region with registered cleanup | Existing cleanup ordering and primary-failure rules hold | Backend failure integration fixture |
| Contracts | Literal sign and endpoint cases in predicates; dynamic `value % 2 == 0` | Literal conclusions agree with runtime; even input passes and odd input fails | Contract reasoning and execution tests |
| Repair | Nonliteral `value % 3 == 1` constraint | No modular repair or false proof; unresolved constraint remains runtime-checked | Repair analysis tests |
| Excluded surfaces | `%` in unsupported schema expression languages, `%=` | Source diagnostic before lowering or runtime | Parser/semantic rejection tests |
| Presentation | Format `a%b`, tokenization, published operator grammar | `a % b`; `%` recognized as an operator; published grammar includes it | Formatter, editor, and language catalog checks |

Generated numeric checks must be bounded in time and memory and include both
operand signs, zero dividends, and signed endpoints. Skip zero divisors in
numeric property checks and verify their failure separately. Record seeds or
use a fixed deterministic input set. The invariant `a = q*b + r` and the sign
and magnitude rules must be checked with arbitrary-precision values.

Operator semantics belong in compiler and backend tests. Public diagnostic
spans, runtime envelopes, and formatter commands belong in focused
`examples/specification/` cases checked through the existing CLI harness.
Those cases test language operators and toolchain contracts; any standard
library calls used for setup do not replace companion API tests. Follow the
[test placement policy](../reference/toolchain-test-harness.md#test-placement-policy).

## Implementation and completion route

Audit lexer tokens, AST operators, parser binding powers, formatter precedence,
semantic and contract operator dispatch, checked core, typed IR, JVM lowering,
source span transport, editor tokenization, and the generated language catalog.
A Rust evaluator must handle `MIN % -1` explicitly rather than allowing Rust's
native signed remainder overflow behavior to define the result.

Add executable evidence before promoting this contract into current behavior.
Update the smallest matching authorities: [source surface](../specification/source-surface.md),
[operator typing](../specification/types.md#operators-and-diagnostics),
[contracts](../specification/contracts.md), and
[run JSON](../specification/run-json.md). Keep the executable source grammar and
published language catalog consistent. Discover the concrete test targets in
the existing harness; use bounded local runs for generated or broad suites.

The proposal is complete only when the acceptance cases have executable
coverage, the current specifications explain usage and failures, and the
published/operator presentation checks recognize `%`. Then remove this page
and its catalog entry. This documentation PR adds no executable evidence and
does not change current behavior.

## Related work and alternatives

- [Java SE 21 Language Specification, §15.17.3](https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.17.3)
  specifies dividend-signed integer remainder, zero-divisor failure, and zero
  for the minimum integer remainder by `-1`. This proposal adopts those integer
  results because they fit the existing JVM integer division implementation.
  Java also accepts floating-point `%`; Veln defers that separate contract.
  Java host exceptions do not determine Veln diagnostic spans or JSON details.
- [The Rust Reference, operator expressions](https://doc.rust-lang.org/reference/expressions/operator-expr.html#arithmetic-and-logical-binary-operators)
  describes truncating integer division and dividend-signed remainder. Its
  [overflow rules](https://doc.rust-lang.org/reference/expressions/operator-expr.html#overflow)
  classify minimum signed integer `% -1` as overflow. Veln adopts the sign rule
  but rejects that endpoint failure: the mathematical remainder is representable
  and the JVM produces zero. This difference requires explicit Rust-side
  constant-evaluation coverage.
- Euclidean modulo would make a positive-divisor result nonnegative, which is
  useful for cyclic indexing. It would change `-7 % 3` from `-1` to `2` and
  depart from the chosen truncating quotient model. Defer a separately named
  helper rather than giving `%` two meanings.
- Keeping only `a - (a / b) * b` avoids a token but repeats operands unless
  callers introduce bindings and exposes intermediate overflow. A dedicated
  operator gives one evaluation per operand and a direct numeric contract.

The source review covered the official Java and Rust operator specifications,
the current Veln operator specification, parser dispatch, and JVM arithmetic
helper. External sources support numeric alternatives, not the proposed Veln
error identifiers, integer-only scope, or repair boundary; those are design
choices verified by the acceptance model. No claim of novelty or prior Veln
proposal consensus is made.
