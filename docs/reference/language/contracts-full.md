# Contracts Full

## Predicate Syntax And Validation

Implemented contract clauses are `require`, `ensure`, and `invariant` lines
attached to a function. The parser first checks a narrow contract predicate
syntax. It accepts literals, names, qualified names, grouping, field access
syntax, plain or qualified call syntax, prefix operators, arithmetic operators,
comparisons, equality, and boolean operators.

The parser rejects holes, `?`, pipelines, `match`, records, and lists in
contract predicates. Unsupported contract syntax in `require`, `ensure`, or
`invariant` reports `parse.contract_predicate`; unsupported syntax in a hole
`satisfy` predicate reports `parse.satisfy_predicate`.

After parsing, the checker validates a small pure boolean subset:

- `true` and `false`
- string literals, including equality and disequality comparisons with the
  string literal on either side
- boolean bindings visible to the clause
- `and`, `or`, and `not`
- arithmetic expressions over numeric literals and visible numeric bindings
- comparison and equality operators over compatible operands
- field access on record-typed bindings visible to the clause
- calls to discovered pure functions when arguments are assignable to the
  declared parameter types and the return type fits the predicate position
- field access on record-typed values returned by discovered pure functions
- qualified calls to discovered pure functions through `use` aliases
- pure prelude helper calls such as `list_len(items)` and
  `list_is_empty(items)`
- visible parameter bindings
- explicit result bindings in `ensure` clauses

Contract call arguments may use the same validated pure subset, including
numeric arithmetic, field access, and function declaration values passed where
the callee expects a function type. A bare arithmetic expression such as
`value + 1` is valid syntax but not a boolean predicate, so it reports a
non-boolean contract diagnostic instead of becoming a runtime obligation.
Pure function calls that return numeric values may participate in arithmetic
operands of comparison predicates, such as `next(value) + 1 > 0`. Pure
function calls that return records may feed field access, such as
`summary(value).ready`.
Prelude helper calls participate under the same predicate rules as pure
function calls: boolean-returning helpers may stand alone, and numeric-returning
helpers must be used in a boolean context such as a comparison.
Names, call-looking text, and field-looking text inside string literals are
literal text and do not participate in predicate name resolution, function-call
discovery, or field validation.

Contract predicates containing `stdio::`, effectful function calls,
unsupported call targets, empty predicates, missing record fields,
type-incompatible call arguments, non-boolean predicates, or unresolved names
produce diagnostics. Valid contracts are recorded and may contribute hole
repair constraints.

## Runtime Obligations

Valid contract clauses are runtime obligations for executable `run` and `test`
entry paths. `require` clauses are checked when a function is entered.
`ensure` clauses are checked before returning through the tail expression and
before `?` returns an error result early. `invariant` clauses are checked both
when a function is entered and before returning through the tail expression or
through a `?` early return. Runtime `require` failures blame the caller;
runtime `ensure` failures blame the implementation. Runtime `invariant`
failures blame the caller at entry and the implementation at return.

## Static Obligation Classification

The implemented obligation classification is conservative. Valid predicates
made from boolean literals, literal comparisons, parentheses, `not`, `and`,
and `or` are classified as `statically_proven` when they evaluate to `true`;
the runtime does not emit a check for them. Literal comparisons include
boolean and string equality or disequality and numeric equality, disequality,
and ordering, using exact decimal literal ordering. Balanced grouping around
literal operands does not prevent static literal comparison, so
`("ready") == "ready"` is statically proven. Numeric literal comparisons may
include pure literal `+`, `-`, and `*` subexpressions, such as `1 + 1 == 2`,
`10 - 4 == 6`, `0.5 + 2.0 == 2.5`, and `3 * 4 >= 12`.
They may also include `/` in comparison-only arithmetic, including divisions
that are not exactly representable as finite decimals, such as `8 / 4 == 2`,
`1 / 2 == 0.5`, and `1 / 3 < 0.34`.
Small boolean formulas over up to twelve otherwise unknown pure predicates are
also classified by exhaustive truth-table evaluation after literal and
comparison folding. This covers nested `and`, `or`, and `not` tautologies such
as
`not (flag and not ready) or not (not flag and not ready)`.
The same static truth folding also accepts boolean identity cases where one
side of `or` is provably
`true`, even if the other side is not itself statically known, and propagates
that result through literal-only `and` and `not` wrappers. Same-shape
comparisons are statically known after whitespace normalization:
`value + 1 == value + 1`, `output >= output`, and `not(value < value)` are
statically proven after validation. Boolean equality and disequality over
statically known boolean subexpressions are also classified statically, such
as `(1 < 2) == true`, `(not false) == true`, and
`(output == output) != false`.
Equality and disequality comparisons between complementary pure predicates are
also classified statically after validation, such as
`flag != not flag`, `(value == limit) != (value != limit)`, and
`output.ready != not(output.ready)`.
A top-level `or` between the same pure
boolean predicate and its negation is also statically true after validation,
such as `flag or not flag` or
`output.ready or not(output.ready)`. Boolean literal aliases participate in
the same identity, so `flag == true or not flag`, `false == flag or flag`, and
`flag != false or flag == false` are statically proven. The complementary
`or` identity may span
more than two top-level branches, so `flag or extra or not flag` is also
statically proven. Parenthesized nested `or` branches are flattened for this
identity, so `flag or (extra or not flag)` is also statically proven. Top-level
`or` also recognizes a branch that is repeated inside a negated `and`
conjunction, such as `flag or not (flag and extra)` and
`not(value < limit and ready) or value < limit`. Top-level `or` also
recognizes a negated disjunction whose non-static branches are repeated by
other top-level disjuncts, such as
`not (flag or ready) or flag or ready`.
Top-level `or` also
recognizes a conjunction whose non-static conjuncts are all covered by
complement disjuncts, such as
`(flag and ready) or not flag or not ready` and
`(value < limit and ready) or value >= limit or not ready`. It also recognizes
negated conjunctions where one disjunction branch is completely covered by
complement conjuncts, such as
`not ((flag or ready) and not flag and not ready)` and
`not ((value < limit or ready) and value >= limit and not ready)`.
It also recognizes a negated disjunction repeated by an outer conjunction,
such as `not (flag and not (flag or ready))` and
`not (value < limit and not (value < limit or ready))`.
It also recognizes resolved complementary disjunctions contradicted by another
conjunct, such as
`not (flag and (not flag or ready) and (not flag or not ready))`.
It also recognizes negated partial case-split conjunctions where top-level
`and` clauses are disjunctions that reject every assignment for the same
predicate set, such as
`not ((flag or ready) and (flag or not ready) and (not flag or ready) and (not flag or not ready))`.
It also recognizes
factored case splits when two conjunction branches differ only by one
complementary predicate and the remaining shared predicates are covered by
complement disjuncts, such as
`(flag and ready) or (not flag and ready) or not ready` and
`(value < limit and ready) or (value >= limit and ready) or not ready`.
It also recognizes partial case splits where shorter branches cover the
remaining assignments for the same predicate set, such as
`flag or (not flag and ready) or (not flag and not ready)` and
`value < limit or (value >= limit and ready) or (value >= limit and not ready)`.
This partial case-split rule may cover up to eleven non-static predicates, so
longer decision ladders with shorter branches are also statically proven when
their top-level `or` branches cover every assignment.
It also recognizes exhaustive pair case splits where four top-level
conjunction branches cover both polarities of two non-static predicates, such
as
`(flag and ready) or (flag and not ready) or (not flag and ready) or (not flag and not ready)`
and
`(value < limit and ready) or (value < limit and not ready) or (value >= limit and ready) or (value >= limit and not ready)`.
It also recognizes exhaustive triple case splits where eight top-level
conjunction branches cover both polarities of three non-static predicates.
It also recognizes exhaustive quad case splits where sixteen top-level
conjunction branches cover both polarities of four non-static predicates.
It also recognizes exhaustive quint case splits where thirty-two top-level
conjunction branches cover both polarities of five non-static predicates.
It also recognizes exhaustive sext case splits where sixty-four top-level
conjunction branches cover both polarities of six non-static predicates.
It also recognizes exhaustive sept case splits where one hundred twenty-eight
top-level conjunction branches cover both polarities of seven non-static
predicates.
It also recognizes exhaustive oct case splits where two hundred fifty-six
top-level conjunction branches cover both polarities of eight non-static
predicates.
It also recognizes exhaustive case splits with the same shape for nine, ten,
or eleven non-static predicates. These high-arity exhaustive case splits use
the dedicated case-split classifier rather than the smaller general
truth-table path.
Top-level `or` also recognizes complementary comparison pairs over the same
operands after whitespace normalization and commuted ordering normalization,
such as `value == limit or value != limit`,
`value < limit or value >= limit`, and `value < limit or limit <= value`.
It also recognizes top-level ordering trichotomy over the same operands, such
as `value < limit or value == limit or value > limit`, after whitespace
normalization and commuted ordering normalization.
Inclusive ordering totality over the same operands is statically proven, such
as `value <= limit or limit <= value` and
`value >= limit or limit >= value`, after whitespace normalization and
commuted ordering normalization.
Disequality over ordered operands also supports the corresponding strict-order
split, so `not (value != limit) or value < limit or value > limit` is
statically proven when the predicate has passed validation.
Top-level `or` also proves implications where a negated `and` of ordering
bounds transitively guarantees another ordering bound. For example,
`not (low <= mid and mid < high) or low < high` is statically proven because
the antecedent guarantees the strict endpoint bound, while an all-inclusive
path only proves an inclusive endpoint bound. Equality clauses in the
antecedent are treated as bidirectional non-strict edges for this transitive
check, so `not (low < mid and mid == high) or low < high` and
`not (low == mid and mid <= high) or low <= high` are also statically proven.
When a non-strict endpoint bound is written as strict ordering or equality,
the same transitive check also proves that disjunction. For example,
`not (low <= mid and mid <= high) or low < high or low == high` is statically
proven.
Numeric literal bounds inside the negated antecedent also prove weaker literal
bounds on the same subject, including through equality aliases in the
antecedent. For example,
`not (value > 10 and value < 20) or value > 5` and
`not (value >= 2 and value < 10) or value >= 1 + 1` are statically proven,
as is `not (value == alias and alias > 10) or value > 5`,
while a strict consequent is not proven from an inclusive bound with the same
literal.
Non-strict cycles in the antecedent also prove equality consequents, such as
`not (low == mid and mid == high) or low == high` and
`not (low <= mid and mid <= low) or low == mid`. Strict edges do not prove
equality consequents.
Equality paths plus an endpoint disequality in the antecedent also prove
disequality consequents, such as
`not (low == mid and mid != high) or low != high` and
`not (high != mid and mid == low) or high != low`.
Strict ordering paths also prove endpoint disequality consequents, such as
`not (low < mid and mid <= high) or low != high` and
`not (high >= mid and mid > low) or high != low`.
Negated conjunctions are also statically proven when transitive order facts
contradict equality relations inside the same conjunction. This includes
non-strict cycles combined with disequality, such as
`not (low <= mid and mid <= low and low != mid)`, and strict transitive paths
combined with equality, such as
`not (low < mid and mid <= high and low == high)`.
Top-level `or` also proves case-split predicates when one branch is the
complement of another branch and every other conjunct in that branch is
statically true. For example,
`flag or (not flag and true)`,
`flag or (not flag and 1 + 1 == 2)`, and
`value < limit or (value >= limit and true)` are statically proven after
validation. The same rule also applies when both branches are `and`
conjunctions with exactly one non-static variant each, such as
`(flag and true) or (not flag and 1 == 1)` and
`(value < limit and true) or (value >= limit and 1 + 1 == 2)`.
It also treats a negated top-level `and` between the same pure boolean
predicate and its negation as statically true, such as
`not (flag and not flag)` or
`not(output.ready and not output.ready)`. For example,
`not (flag and extra and not flag)` is statically proven because the inner
conjunction contains complementary branches. Parenthesized nested `and`
branches are flattened for this identity, so
`not (flag and (extra and not flag))` is also statically proven. The same
negated-`and` identity applies to complementary comparison pairs, such as
`not (value == limit and limit != value)` and
`not(output < limit and output >= limit)`. It also applies to mutually
exclusive ordering trichotomy relations over the same operands, such as
`not (value < limit and value == limit)`,
`not (value < limit and value > limit)`, and
`not(output == limit and output > limit)`. It also applies when opposite
inclusive and strict bounds cannot both hold, such as
`not (value <= limit and limit < value)`. Negated conjunctions of numeric
literal bounds over the same subject are also statically proven when the lower
bound is greater than the upper bound, or when equal bounds include at least
one strict side, such as `not (value > 10 and value < 5)` and
`not (value >= 10 and value < 10)`. Top-level `or` predicates over numeric
literal bounds on the same subject are statically proven when one lower bound
and one upper bound cover every value, such as
`value <= 10 or value >= 5` and `value > 2 or value <= 2`.
Negated conjunctions of equality clauses
that bind the same subject to distinct boolean, numeric, or string literals are
also statically proven, such as `not (name == "Ada" and name == "Grace")`.
For example,
`true or value > 0`, `(output >= value or true) and not false`, and
`1 < 2 and "ready" != "pending"` are statically proven after the predicate has
passed validation. Equality and disequality comparisons between small boolean
formulas are also evaluated by assignment when every assignment gives the same
comparison result, such as
`(flag and ready) == (ready and flag)` and
`(flag and not flag) != (ready or not ready)`. Other valid predicates are
classified as
`runtime_required`. Invalid predicates fail the static contract gate instead
of becoming runtime-only checks. No contract is currently classified as
statically disproven.

## Result Binding

An `ensure` clause may refer to the returned value only when the function return
position names it with `-> name: Type`. That name is not visible to `require`
or `invariant` clauses or the function body. The identifier `result` is
ordinary: without an explicit binding named `result`, it reports an
unresolved-name diagnostic.
