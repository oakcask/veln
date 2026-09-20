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
string literal is literal data and is never resolved as a name.

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

| Clause | Entry failure blames | Return failure blames |
| --- | --- | --- |
| `require` | caller | — |
| `ensure` | — | implementation |
| `invariant` | caller | implementation |

The JVM backend does not correctly evaluate every predicate accepted by the
checker. In particular, a runtime-required `and` or `or` expression is not
lowered as a boolean operator. For example, `require value >= 0 and ready`
can fail with a JVM type-cast error even when both conditions hold. Separate
`require` clauses, as above, check both conditions correctly. Statically proven
boolean combinations do not encounter this limitation because they emit no
runtime check.

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
| Boolean identities | Literal folding, `not`, complementary predicates, boolean equality/disequality, same-shape comparisons, and tautological `and`/`or` forms. Whitespace, redundant atom parentheses, and commuted ordering are normalized where the rule says so. |
| Truth tables | Truth-table evaluation is bounded at 13 unknown predicates after folding and is skipped for 512 or more top-level disjuncts. Partial case-split search is bounded at 10 predicates; a separate exhaustive disjunction rule handles 7–11 predicates. Other specialized rules may still prove larger formulas. |
| Order reasoning | Complementary equality/order pairs, trichotomy, inclusive totality, strict splits from disequality, and transitive implications over `<`, `<=`, `==`, and their commuted forms. Non-strict paths can prove equality; strict paths do not. |
| Literal bounds | Contradictory bounds prove a negated conjunction; covering lower/upper bounds prove a disjunction. Equality aliases may connect bounds. An inclusive bound does not prove a strict consequent. |

Representative proven predicates include:

```text
1 / 3 < 0.34
flag or not flag
value == limit or value != limit
not (low <= mid and mid < high) or low < high
not (value > 10 and value < 5)
```

Same-shape expressions such as `value + 1 == value + 1` are proven after
normalization. Literal aliases and commuted forms participate only where the
corresponding comparison rule applies; arbitrary equivalent expressions do not
become proven. A valid predicate outside these classes remains
`runtime_required`.

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
