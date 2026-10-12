---
role: specification
authority: normative
specification-coverage: usage=#predicate-usage-and-validation; behavior=#runtime-obligations; limits=#static-classification
update-when: Contract predicate syntax, validation, runtime checks, static classification, result bindings, or contract examples change.
---

# Contracts

Contracts attach `require`, `ensure`, or `invariant` clauses to a function.
They are validated before they become runtime obligations.

## Predicate usage and validation

The source predicate grammar accepts literals, visible names and qualified
names, grouping, field access, plain or qualified calls, prefix operators,
arithmetic, comparisons, equality, and `and`/`or`. A valid predicate must be a
pure boolean expression. The checker can use visible parameters and bindings,
record fields, discovered pure functions, qualified imports, and pure prelude
helpers. Call arguments use the same typed pure subset, including a function
value where the declared parameter expects a function type.

```veln
fn accepted(value: Int, ready: Bool) -> ()
	require value >= 0
	require ready
	ensure true
	()
end
```

Pure calls returning numbers may occur inside comparisons, and pure calls
returning records may feed field access. A numeric expression alone is not a
predicate: `require value + 1` is a non-boolean diagnostic. Text inside a
string literal is literal data and is never resolved as a name. Parentheses
and commas inside a string literal do not close a grouped predicate or split a
call's argument list.

Integer remainder uses the same operator contract in predicates as in ordinary
expressions: `%` has multiply-level precedence and left associativity, and both
operands and the result are `Int`. A `Float` context does not widen either
operand or the result. A non-`Int` operand reports `type.mismatch`. A literal
integer zero divisor reports `type.remainder_by_zero` at the divisor, with the
operation as related context, and the invalid predicate does not become a
runtime obligation. A non-`Int` zero such as `0.0` reports only the operand
`type.mismatch`; it does not also report `type.remainder_by_zero`. The complete
operator contract is specified in
[Operators and diagnostics](types.md#operators-and-diagnostics).

Grouping, numeric prefix operators, comparisons, Boolean composition, and pure
call arguments preserve this typing. They do not reassociate or reject an
otherwise valid remainder chain. One predicate can contain independent
remainder chains on both sides of a comparison, in separate Boolean branches,
or inside pure call arguments.

Each operand keeps the most specific diagnostic available for its expression.
For example, a missing record field reports `contract.field_missing`, and a
prefixed `Bool` operand reports `type.mismatch` for the required `Int`. When a
composite arithmetic operand contains a non-`Int` leaf, the diagnostic selects
that leaf instead of the complete operand. A surrounding remainder comparison
does not replace any of these diagnostics. When a remainder occurs inside a
pure call argument, its invalid operand is diagnosed before the enclosing call
result is checked as the Boolean predicate. Neither context replaces the
specific failure with `contract.type_mismatch` or
`contract.unsupported_construct`.

The parser rejects holes, `?`, pipelines, `match`, records, and lists in
contract predicates. `perform Effect::operation(...)` can parse as a candidate
predicate but fails the effect-free checker. Unsupported syntax reports
`parse.contract_predicate`; unsupported syntax in a hole's `satisfy` predicate
reports `parse.satisfy_predicate`.

Validation fails for effectful calls, `stdio::` calls, unresolved names,
unsupported call targets, missing fields, incompatible arguments, empty
predicates, and non-boolean results. Invalid predicates never become
runtime-only checks and never contribute repair constraints.

## Runtime obligations

Valid obligations run on executable `run` and `test` paths. `require` runs when
the function is entered. `ensure` runs before normal return and before a `?`
error return. `invariant` runs at entry and before normal or `?` return.
`satisfy` is a hole repair constraint, not a runtime contract obligation; its
nonliteral remainder expressions are not executed.

Runtime-required obligations in a call-site-aware function can read the same
hidden `callsite` value as the function body. A direct call from such an
obligation to another call-site-aware function forwards that value without an
explicit source argument. The forwarding behavior is specified by
[Call-site Declarations](call-site-declarations.md#declaration-behavior).
A call-site-aware callee can use fixed and variadic source parameters. The
call must supply every fixed argument, and each variadic tail argument must
match the declared element type. A variadic contract call to an ordinary
function remains rejected until its runtime ABI is supported. A contract on
an ordinary function does not construct call-site context and cannot call a
call-site-aware function because it has no hidden context to forward. This
deliberate call-site boundary is specified under
[Limits and diagnostics](call-site-declarations.md#limits-and-diagnostics).
A statically proven obligation emits no runtime call. Direct calls in ordinary
function bodies remain executable.

Runtime-required predicates evaluate `not` before `and`, and `and` before
`or`, as in ordinary source expressions. Parentheses can override that
precedence.

For a nonzero divisor, runtime integer remainder computes `q = trunc(a / b)`
and `r = a - q * b` over mathematical integers, then returns `r`. Minimum
`Int` remainder by `-1` is `0`. A computed zero divisor reports
`runtime.remainder_by_zero` at the divisor instead of reporting a violated
contract or exposing a host exception.

The shared multiply-level precedence and left associativity also determine the
numeric type of a mixed expression. For example, `1 % 2 / 2` groups as
`(1 % 2) / 2`. Both operands of `/` are `Int`, so the expression evaluates to
`0`, not `0.5`. Therefore, `require 1 % 2 / 2 == 0.5` fails at runtime.

| Clause | Entry failure blames | Return failure blames |
| --- | --- | --- |
| `require` | caller | — |
| `ensure` | — | implementation |
| `invariant` | caller | implementation |

## Static classification

After validation, a valid obligation is either `statically_proven` or
`runtime_required`. A statically proven true predicate emits no runtime check.
The implementation does not classify a contract as statically disproven; a
predicate that cannot be proven true remains runtime-required.

The classifier combines these rule classes:

| Class | Supported rule |
| --- | --- |
| Literal values | Boolean and string equality/disequality; numeric equality, disequality, and ordering. Parentheses do not change the result. |
| Exact arithmetic | Literal `+`, `-`, `*`, `/`, bitwise, and shift expressions in comparison-only positions. Decimal and rational comparisons use exact values; shift counts outside `0..63` are not folded as valid operations. |
| Integer remainder | A literal-only `%` subexpression is folded when both signed 64-bit operands are known and the divisor is nonzero. Integer arithmetic inside either operand uses runtime `Int` behavior, so `/` truncates toward zero before `%` is evaluated. Remainder uses the runtime signed result, including `0` for minimum `Int` remainder by `-1`. A nonliteral remainder subexpression remains `runtime_required`. |
| Boolean identities | Literal folding, `not`, complementary predicates, boolean equality/disequality, same-shape comparisons, and tautological `and`/`or` forms. Whitespace, redundant atom parentheses, and commuted ordering are normalized where the rule says so. |
| Truth tables | Truth-table evaluation is bounded at 13 unknown predicates after folding and is skipped for 512 or more top-level disjuncts. Partial case-split search is bounded at 10 predicates; a separate exhaustive disjunction rule handles 7–11 predicates. Other specialized rules may still prove larger formulas. |
| Order reasoning | Complementary equality/order pairs, trichotomy, inclusive totality, strict splits from disequality, and transitive implications over `<`, `<=`, `==`, and their commuted forms. Non-strict paths can prove equality; strict paths do not. |
| Literal bounds | Contradictory bounds prove a negated conjunction; covering lower/upper bounds prove a disjunction. Equality aliases may connect bounds. An inclusive bound does not prove a strict consequent. |

Each independent flat remainder chain follows its source left associativity
during classification. A literal-only chain folds each operation from left to
right before the surrounding comparison and Boolean expression are classified.
If any chain contains a nonliteral operand, the complete obligation remains
`runtime_required`; classification does not use a partially folded predicate
to prove it.

Representative proven predicates include:

```text
1 / 3 < 0.34
(5 / 2) % 2 == 0
5 % (3 / 2) == 0
flag or not flag
value == limit or value != limit
not (low <= mid and mid < high) or low < high
not (value > 10 and value < 5)
```

Same-shape expressions such as `value + 1 == value + 1` are proven after
normalization. Literal aliases and commuted forms participate only where the
corresponding comparison rule applies; arbitrary equivalent expressions do not
become proven. Static classification does not reinterpret integer division in a
remainder-containing expression as exact rational division. A false literal
predicate such as `1 % 2 / 2 == 0.5` remains `runtime_required`, because the
classifier does not reject statically false contracts. A valid predicate
outside these classes also remains `runtime_required`.

## Result binding

An `ensure` clause can refer to the returned value only when the return
annotation names it: `-> result: Type`. The binding is visible to `ensure` only,
not to `require`, `invariant`, or the function body. `result` is an ordinary
identifier when no explicit result binding exists, so an unbound use reports an
unresolved-name diagnostic.

## References

- Predicate parsing: `crates/veln-syntax/src/parser/contract_predicates.rs`.
- Validation and classification: `crates/veln-sema/src/contracts/`.
- Runtime contract examples and diagnostics: `examples/specification/test/` and
  `examples/specification/run/` contract cases.
