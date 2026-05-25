# Contracts And Holes

This file specifies implemented contract predicate validation, holes, and
repair constraints.

## Contracts

Implemented contract clauses are `require`, `ensure`, and `invariant` lines
attached to a function. The parser first checks a narrow contract predicate syntax. It
accepts literals, names, qualified names, grouping, field access syntax,
plain or qualified call syntax, prefix operators, arithmetic operators,
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

Valid contract clauses are runtime obligations for executable `run` and `test`
entry paths. `require` clauses are checked when a function is entered.
`ensure` clauses are checked before returning through the tail expression and
before `?` returns an error result early. `invariant` clauses are checked both
when a function is entered and before returning through the tail expression or
through a `?` early return. Runtime `require` failures blame the caller;
runtime `ensure` failures blame the implementation. Runtime `invariant`
failures blame the caller at entry and the implementation at return.

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
They may also include `/` when the result is exactly representable as a finite
decimal, such as `8 / 4 == 2` and `1 / 2 == 0.5`.
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
`output.ready or not(output.ready)`. The complementary `or` identity may span
more than two top-level branches, so `flag or extra or not flag` is also
statically proven. Parenthesized nested `or` branches are flattened for this
identity, so `flag or (extra or not flag)` is also statically proven. Top-level
`or` also recognizes a branch that is repeated inside a negated `and`
conjunction, such as `flag or not (flag and extra)` and
`not(value < limit and ready) or value < limit`. Top-level `or` also
recognizes a conjunction whose non-static conjuncts are all covered by
complement disjuncts, such as
`(flag and ready) or not flag or not ready` and
`(value < limit and ready) or value >= limit or not ready`. It also recognizes
factored case splits when two conjunction branches differ only by one
complementary predicate and the remaining shared predicates are covered by
complement disjuncts, such as
`(flag and ready) or (not flag and ready) or not ready` and
`(value < limit and ready) or (value >= limit and ready) or not ready`.
It also recognizes partial case splits where shorter branches cover the
remaining assignments for the same predicate set, such as
`flag or (not flag and ready) or (not flag and not ready)` and
`value < limit or (value >= limit and ready) or (value >= limit and not ready)`.
It also recognizes exhaustive pair case splits where four top-level
conjunction branches cover both polarities of two non-static predicates, such
as
`(flag and ready) or (flag and not ready) or (not flag and ready) or (not flag and not ready)`
and
`(value < limit and ready) or (value < limit and not ready) or (value >= limit and ready) or (value >= limit and not ready)`.
It also recognizes exhaustive triple case splits where eight top-level
conjunction branches cover both polarities of three non-static predicates.
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
`not (value <= limit and limit < value)`. For example,
`true or value > 0`, `(output >= value or true) and not false`, and
`1 < 2 and "ready" != "pending"` are statically proven after the predicate has
passed validation. Other valid
predicates are classified as
`runtime_required`. Invalid predicates fail the static contract gate instead
of becoming runtime-only checks. No contract is currently classified as
statically disproven.

An `ensure` clause may refer to the returned value only when the function return
position names it with `-> name: Type`. That name is not visible to `require`
or `invariant` clauses or the function body. The identifier `result` is
ordinary: without an explicit binding named `result`, it reports an
unresolved-name diagnostic.

## Holes

Holes produce `hole.unfilled` diagnostics with severity `hint`. A check result
with only non-error hole diagnostics has top-level status `partial`.

Hole details include:

- `phase`
- `node_id`
- `label`
- `expected_type`
- `expected_type_source`
- `constraints`
- `local_bindings`
- `candidate_queries`

Candidate query records are unapplied repair records. Each query carries
`candidate_status: "query_only"` and an `application_policy` value. The default
policy is `manual_review_required`.

When a hole has a known expected type, the symbol query may include ranked
visible binding candidates. A ranked candidate records a binding name, rendered
binding type, rank, match reason, application policy, and a concrete replacement
edit for the hole span. Exact type matches rank before broader assignable
matches, and nearer visible bindings rank before older bindings with the same
match quality. The checker may bound ordinary manual-review candidates, but it
keeps statically satisfied `satisfy` repair candidates even when they fall after
that ordinary bound.

Named holes such as `_port` are diagnostic and repair labels, not bindings.
The `satisfy candidate => predicate` suffix contributes a repair constraint; it
does not bind `candidate` outside the suffix predicate.

For the implemented safe repair subset, a symbol candidate is marked
`application_policy: "safe_repair_candidate"` when substituting the candidate
symbol into every directly checked `satisfy` clause makes the predicate
reflexive or tautological. The accepted direct clauses are equality and
inclusive comparison between the satisfy candidate and the same visible
binding, such as
`candidate == fallback`, `fallback == candidate`, `candidate <= fallback`, and
`fallback >= candidate`. Field-access forms with the same suffix on both sides
are also accepted, such as `candidate.count == fallback.count`. `and` may join
clauses that all name the same binding. Top-level `or` also checks each direct
branch independently. A visible binding is safe when at least one branch
becomes reflexive after substituting that binding, such as `fallback` in
`candidate == fallback or candidate == other`.
Literal `false` disjuncts do not affect direct repair matching, so
`false or candidate == fallback` has the same repair status as
`candidate == fallback`.
Wrapping each direct clause in balanced parentheses does not change this
repair match. Negated direct equality, disequality, and ordering clauses are
normalized before direct repair matching, so `not (candidate != fallback)` and
`not (candidate < fallback)` both mark `fallback` as safe. Double negation is
also normalized, so `not (not (candidate == fallback))` has the same direct
repair status as `candidate == fallback`. Literal `true` conjuncts do not
affect direct repair matching, so
`candidate == fallback and true` has the same repair status as
`candidate == fallback`. Same-shape expression equality and inclusive
comparison clauses are accepted when replacing the satisfy candidate with one
visible binding makes both sides textually identical after whitespace
normalization, such as `candidate + 1 == fallback + 1` and
`candidate + 1 <= fallback + 1`. A nested `or` clause inside a direct `and`
conjunction is ignored when it contains a literal `true` branch, so
`candidate == fallback and (candidate > fallback or true)` has the same direct
repair status as `candidate == fallback`. A nested complementary `or` clause
rooted at the satisfy candidate is also ignored inside direct `and`
conjunctions, so `candidate.ready == fallback.ready and
(candidate.ready or not candidate.ready)` has the same direct repair status as
`candidate.ready == fallback.ready`. A negated disjunction of direct
comparison clauses is normalized before direct repair matching, so
`not (candidate != fallback or candidate < fallback)` has the same direct
repair status as `candidate == fallback and candidate >= fallback`. A negated
disjunction may include literal `false` branches without changing direct
repair matching, so `not (false or candidate != fallback)` has the same
direct repair status as `candidate == fallback`. A negated
conjunction of direct comparison clauses is normalized into disjunctive direct
branches before matching, so
`not (candidate != fallback and candidate < fallback)` has the same direct
repair status as `candidate == fallback or candidate >= fallback`. Literal
boolean branches created by this negated-conjunction normalization are folded,
so `not (true and candidate != fallback)` has the same direct repair status as
`candidate == fallback`, and `not (false and candidate != fallback)` ranks as
a tautological repair constraint. The
accepted tautological clauses
compare the satisfy candidate with itself using `==`, `<=`, or `>=`, such as
`candidate == candidate`; their negated inverse forms, such as
`not (candidate != candidate)`, are also accepted. The same rule applies to
matching field-access paths rooted at the candidate, such as
`candidate.count == candidate.count`. Same-shape expression tautologies that
reference the satisfy candidate are also accepted after whitespace
normalization, such as `candidate + 1 == candidate + 1`. `and` may join only
tautological clauses or literal `true` clauses. Wrapping each tautological
clause in balanced parentheses does not change this repair match. Literal
`false` disjuncts do not affect tautological repair matching. A top-level
literal `true` disjunct makes the whole `satisfy` predicate tautological for
repair ranking, so every type-compatible visible binding is a safe repair
candidate. Literal boolean negation is normalized before this check, so
`not false` is treated like `true` and `not true` is treated like `false`.
A top-level disjunct that is itself tautological for the satisfy candidate has
the same effect; for example,
`candidate == candidate or candidate == fallback` ranks every type-compatible
visible binding as a safe tautology repair candidate.
Top-level complementary disjuncts over the same candidate-referencing
predicate also rank as tautological. For example,
`candidate.ready or not candidate.ready` ranks every type-compatible visible
binding as a safe tautology repair candidate.
Complementary comparison disjuncts that reference the candidate also rank as
tautological after whitespace normalization and commuted ordering
normalization. For example,
`candidate == limit or candidate != limit` and
`candidate < limit or limit <= candidate` rank every type-compatible visible
binding as a safe tautology repair candidate. The same rule covers the
reversed inclusive spelling, such as `candidate <= limit or candidate > limit`.
Top-level ordering trichotomy disjuncts that reference the candidate are also
tautological after whitespace normalization and commuted ordering
normalization. For example,
`candidate < limit or candidate == limit or candidate > limit` ranks every
type-compatible visible binding as a safe tautology repair candidate.
Inclusive ordering totality disjuncts that reference the candidate are also
tautological after whitespace normalization and commuted ordering
normalization. For example,
`candidate <= limit or limit <= candidate` ranks every type-compatible visible
binding as a safe tautology repair candidate.
A negated top-level `and` with complementary candidate-referencing branches is
also tautological for repair ranking. For example,
`not (candidate.ready and limit.ready and not candidate.ready)` ranks every
type-compatible visible binding as a safe tautology repair candidate.
Parenthesized nested `and` branches are flattened for this identity, so
`not (candidate.ready and (limit.ready and not candidate.ready))` also ranks
every type-compatible visible binding as a safe tautology repair candidate.
Negated top-level `and` predicates with mutually exclusive ordering
trichotomy clauses rooted at the candidate are also tautological for repair
ranking. For example, `not (candidate < limit and candidate == limit)` ranks
every type-compatible visible binding as a safe tautology repair candidate.
The same repair tautology applies when opposite inclusive and strict bounds
rooted at the candidate cannot both hold, such as
`not (candidate <= limit and limit < candidate)`.
After predicate validation, `satisfy` repair tautology also reuses the
implemented static truth identities from contract obligation classification.
For example,
`candidate.ready or (not candidate.ready and true)` ranks every type-compatible
visible binding as a safe tautology repair candidate.
Nested `or` clauses with a literal `true` branch are ignored inside
tautological `and` clauses, so
`candidate == candidate and (candidate > candidate or true)` is ranked as a
tautology. Nested complementary `or` clauses rooted at the satisfy candidate
are ignored the same way, so
`candidate.ready == candidate.ready and
(candidate.ready or not candidate.ready)` is ranked as a tautology.
A candidate is also safe when replacing the
satisfy candidate binding with the visible symbol makes every non-`true` `and`
clause match a valid `require` clause already in force for the function; such
candidates use `reason: "satisfy_require_match"`. This includes clauses with
string literals, such as matching `candidate != ""` against `name != ""` after
substituting `name`. Simple direct and commuted comparison clauses are treated
as the same requirement, such as matching
`candidate > 0` against `0 < max` after substituting `max`; wrapping these
simple clauses or the whole `and` conjunction in parentheses does not change
the repair match. Negated equality and disequality clauses are normalized
before matching; for example, `not (candidate == 0)` matches `max != 0` after
substituting `max`, and `not (max == 0)` guarantees `candidate != 0` after the
same substitution. Double negation is also normalized during requirement
matching; for example, `require not (not (max > 0))` guarantees
`candidate > 0` after substituting `max`. Negated ordering clauses normalize
into their inverse comparisons before matching; for example,
`not (candidate < 0)` matches `max >= 0` after substituting `max`, and
`not (candidate <= 0)` matches `max > 0`. Strict ordering requirements also
discharge the matching inclusive ordering predicate for the same operands. For
example, a `require max > 0` clause guarantees `candidate >= 0` after
substituting `max`, and a `require max < 10` clause guarantees
`candidate <= 10`. Strict ordering
requirements also discharge disequality for the same operands, such as
`require max > 0` guaranteeing `candidate != 0` after substituting `max`.
Equality requirements also discharge inclusive ordering predicates over the
same operands in either direction; for example, `require max == 0` guarantees
both `candidate <= 0` and `candidate >= 0` after substituting `max`. If a
binding has valid inclusive bounds in both directions, those combined
requirements also discharge equality after substituting the binding; for
example, `require max <= 10` together with `require max >= 10` guarantees
`candidate == 10` after substituting `max`. If a substituted `satisfy`
predicate names an operand that is equated by a valid `require` clause, other
valid non-disjunctive `require` clauses may discharge the substituted clause
through that alias. For example, `require max == fallback` together with
`require fallback > 0` guarantees `candidate > 0` after substituting `max`.
Alias discharge also preserves the strict-ordering-to-disequality rule and the
paired-inclusive-bounds-to-equality rule. For example, those same requirements
guarantee `candidate != 0` after substituting `max`; `require fallback <= 10`
together with `require max >= 10` and `require max == fallback` guarantees
`candidate == 10` after substituting either `max` or `fallback`.
Disjunctive alias requirements are checked branch by branch with the other
valid `require` clauses in force. For example,
`require max == fallback or max == backup` together with
`require fallback > 0` and `require backup > 0` guarantees
`candidate > 0` after substituting `max`.
Valid non-disjunctive ordering requirements may also be chained transitively
for repair discharge. A chain with at least one strict edge guarantees a
strict ordering predicate and disequality for the endpoints; an all-inclusive
chain guarantees an inclusive ordering predicate. Inclusive chains in both
directions also guarantee endpoint equality. For example,
`require low < mid` together with `require mid <= max` guarantees
`candidate > low` after substituting `max`, and `require low <= mid` together
with `require mid < max` guarantees `candidate != low` after substituting
`max`. `require low <= mid` together with `require mid <= max` and
`require max <= low` guarantees `candidate == low` after substituting `max`.
Disjunctive `require` clauses can contribute a transitive ordering edge when
every branch guarantees the same weaker comparison. For example,
`require low < mid or low == mid` contributes the inclusive edge
`low <= mid`, which can combine with `require mid < max` to guarantee
`candidate > low` after substituting `max`.
Disjunctive `require` clauses also contribute a common boolean atom when every
branch guarantees that same atom. For example, `require max.ready or
max.ready` guarantees `candidate.ready` after substituting `max`.
If a substituted `satisfy` predicate has top-level `or` clauses, a candidate
is also safe when any one `or` branch is fully guaranteed by valid `require`
clauses.
For example, `candidate > 0 or candidate == 0` is guaranteed for a visible
binding `max` when the function already has `require max > 0`. Literal
`false` disjuncts do not affect this match on either side, so
`false or candidate > 0` may be guaranteed by `false or max > 0` after
substituting `max`. A `require`
predicate with top-level `or` also guarantees a substituted `satisfy` clause
when every `or` branch discharges that clause, such as
`require max > 0 or max == 0` guaranteeing `candidate >= 0` after substituting
`max`. A `require` predicate with top-level `or` also guarantees a substituted
top-level `satisfy` `or` predicate when every `require` branch discharges at
least one `satisfy` branch, so `require max > 0 or max == 0` guarantees
`candidate > 0 or candidate == 0` after substituting `max`.
Equality branches against distinct boolean, integer, or string literals
also discharge disequality against another literal; for example,
`require max == 1 or max == 2` guarantees `candidate != 0` after substituting
`max`. The same literal-disequality rule participates through equality
aliases, so `require max == fallback` together with `require fallback == 1`
guarantees `candidate != 0` after substituting either binding. The same rule
applies inside conjunctions:
`(max > 0 or max == 0) and max <= 10` guarantees
`candidate >= 0 and candidate <= 10` after substituting `max`. Nested `or`
branches inside `satisfy` conjunctions are accepted when at least one branch is
guaranteed; for example, `require max > 0 and max <= 10` guarantees
`(candidate > 0 or candidate == 0) and candidate <= 10` after substituting
`max`. A nested `or` branch with literal `true` is ignored during
`require`-matched repair discharge, so `(candidate > 0 or true) and
candidate <= 10` is guaranteed by `require max <= 10` after substituting
`max`. Negated top-level `or` predicates in valid `require` clauses are
normalized through their direct comparison branches before repair matching.
For example, `require not (max < 0 or max > 10)` guarantees both
`candidate >= 0` and `candidate <= 10` after substituting `max`. Literal
`false` branches inside the negated disjunction do not change this matching,
so `require not (false or max <= 0)` guarantees `candidate > 0` after
substituting `max`. Negated disjunctions also expose negated boolean atom
branches, so `require not (max.ready or other.ready)` guarantees
`not candidate.ready` after substituting either `max` or `other`. Negated
top-level `and` predicates in `satisfy` clauses are normalized into their
inverted `or` branches before `require` matching. For example,
`not (candidate <= 0 and candidate > 10)` is guaranteed by `require max > 0`
after substituting `max`, because one inverted branch is guaranteed. Negated
top-level `and` predicates in valid `require` clauses can also guarantee a
disjunctive `satisfy` predicate when every inverted branch discharges one
top-level `satisfy` branch. For example,
`require not (max <= 0 and max > 10)` guarantees
`candidate > 0 or candidate <= 10` after substituting `max`. Literal boolean
branches created by this normalization are folded before matching; for example,
`require not (true and max <= 0)` guarantees `candidate > 0` after
substituting `max`. Every
same-shape expression operand in `require`-matched repair is compared after
whitespace normalization, so `require max + 1 <= fallback + 1` guarantees
`candidate+1 <= fallback+1` after substituting `max`. Equality requirements
also apply inside same-shape expression operands before that comparison, so
`require max == fallback` plus `require fallback + 1 <= limit + 1` guarantees
`candidate + 1 <= limit + 1` after substituting `max`. The same expression
operand aliasing applies while chaining transitive ordering evidence, so
`require max == fallback`, `require fallback + 1 <= mid + 1`, and
`require mid + 1 <= limit + 1` guarantee
`candidate + 1 <= limit + 1` after substituting `max`. Equality requirements
also apply to boolean atom clauses, so `require max == fallback` plus
`require fallback.ready` guarantees `candidate.ready` after substituting
`max`. Boolean atoms and literal boolean comparisons discharge each other:
`require flag` guarantees `candidate == true` and `candidate != false` after
substituting `flag`, `require not flag` guarantees `candidate == false`, and
`require flag.ready == true` guarantees `candidate.ready` after substitution.
Equivalent literal boolean comparisons also discharge each other, so
`require flag != false` guarantees `candidate == true`, and
`require flag == true` guarantees `candidate != false` after substituting
`flag`.
Inclusive transitive ordering plus endpoint disequality guarantees a
strict comparison in repair matching, so `require low <= mid`,
`require mid <= max`, and `require max != low` guarantee `candidate > low`
after substituting `max`. A disequality between two operands on the inclusive
path also makes the endpoint comparison strict, so `require low <= mid`,
`require mid <= max`, and `require low != mid` also guarantee
`candidate > low` after substituting `max`. Numeric literal bounds also
discharge weaker numeric literal bounds over the same subject. For example,
`require max >= 10` guarantees `candidate > 0` after substituting `max`,
`require ratio >= 10.5` guarantees `candidate > 0.5` after substituting
`ratio`, `require ratio >= -0.5` guarantees `candidate > -1.5` after
substituting `ratio`, and `require min <= 10` guarantees `candidate < 20`
after substituting `min`. Numeric literal equalities also discharge weaker
numeric literal bounds over the same subject, so `require max == 10`
guarantees `candidate > 0` after substituting `max`, and `require min == 10`
guarantees `candidate < 20` after substituting `min`. Numeric literal ordering
uses exact decimal literal ordering rather than binary floating-point rounding.
Numeric literal bounds in repair matching may include pure literal `+`, `-`,
`*`, and exactly representable `/` subexpressions, so
`require max > 1 + 1` guarantees `candidate > 2` after substituting `max`.
Numeric literal bounds also discharge disequality against excluded numeric
literals over the same subject. For example, `require max > 10` guarantees
`candidate != 0` after substituting `max`, and `require ratio <= -0.5`
guarantees `candidate != 0.5` after substituting `ratio`.
Equal inclusive bounds do not discharge strict bounds, so `require max >= 10`
does not guarantee `candidate > 10`. They also do not discharge disequality
against the endpoint, so `require max >= 10` does not guarantee
`candidate != 10`.
Numeric disequality requirements also discharge strict ordering disjunctions
around the excluded literal, including through equality aliases. For example,
`require max != 0` guarantees
`candidate < 0 or candidate > 0` after substituting `max`, and
`require max == fallback` together with `require fallback != 0` guarantees the
same predicate after substituting either binding. Every
type-compatible visible binding candidate for the tautological
subset uses `reason: "satisfy_tautology"`. A statically accepted candidate also
uses `satisfy_status: "statically_satisfied"`. Other candidates for a
satisfy-constrained hole remain unapplied, use
`application_policy: "manual_review_required"`, and carry
`satisfy_status: "blocked_until_discharged"`.

`satisfy` suffixes must include one candidate binding and `=>`. Missing
candidate bindings report `parse.satisfy_candidate`; missing arrows report
`parse.satisfy_arrow`.

The candidate binding is scoped only to the suffix predicate. It must not
shadow visible local bindings, parameters, explicit result bindings, or
compiler-known prelude helper names. Shadowing reports
`hole.satisfy_candidate_shadow`.

The predicate must reference the candidate binding at least once. A predicate
that omits the candidate reports `hole.satisfy_candidate_unused`.

After parsing, the checker validates a `satisfy` predicate against the same
small pure boolean subset used by contracts. The candidate binding is visible
inside that validation with the hole expected type when one is known. Invalid
satisfy predicates report hole diagnostics for non-boolean predicates,
unsupported constructs, or missing record fields, and report unresolved names
with the `satisfy_predicate` namespace.
