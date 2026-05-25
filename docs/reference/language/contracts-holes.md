# Contracts And Holes

This file specifies implemented contract predicate validation, holes, and
repair constraints.

## Contracts

Implemented contract clauses are `require` and `ensure` lines attached to a
function. The parser first checks a narrow contract predicate syntax. It
accepts literals, names, qualified names, grouping, field access syntax,
plain or qualified call syntax, prefix operators, arithmetic operators,
comparisons, equality, and boolean operators.

The parser rejects holes, `?`, pipelines, `match`, records, and lists in
contract predicates. Unsupported contract syntax in `require` or `ensure`
reports `parse.contract_predicate`; unsupported syntax in a hole `satisfy`
predicate reports `parse.satisfy_predicate`.

After parsing, the checker validates a small pure boolean subset:

- `true` and `false`
- boolean bindings visible to the clause
- `and`, `or`, and `not`
- arithmetic expressions over numeric literals and visible numeric bindings
- comparison and equality operators over compatible operands
- field access on record-typed bindings visible to the clause
- calls to discovered pure functions when arguments are assignable to the
  declared parameter types and the return type fits the predicate position
- qualified calls to discovered pure functions through `use` aliases
- visible parameter bindings
- explicit result bindings in `ensure` clauses

Contract call arguments may use the same validated pure subset, including
numeric arithmetic and field access. A bare arithmetic expression such as
`value + 1` is valid syntax but not a boolean predicate, so it reports a
non-boolean contract diagnostic instead of becoming a runtime obligation.
Pure function calls that return numeric values may participate in arithmetic
operands of comparison predicates, such as `next(value) + 1 > 0`.

Contract predicates containing `stdio::`, effectful function calls,
unsupported call targets, empty predicates, missing record fields,
type-incompatible call arguments, non-boolean predicates, or unresolved names
produce diagnostics. Valid contracts are recorded and may contribute hole
repair constraints.

Valid contract clauses are runtime obligations for executable `run` and `test`
entry paths. `require` clauses are checked when a function is entered.
`ensure` clauses are checked before returning through the tail expression and
before `?` returns an error result early. Runtime `require` failures blame the
caller; runtime `ensure` failures blame the implementation.

The implemented obligation classification is conservative: every valid
contract predicate is classified as `runtime_required`. Invalid predicates fail
the static contract gate instead of becoming runtime-only checks. No contract is
currently classified as statically proven or statically disproven.

An `ensure` clause may refer to the returned value only when the function return
position names it with `-> name: Type`. That name is not visible to `require`
clauses or the function body. The identifier `result` is ordinary: without an
explicit binding named `result`, it reports an unresolved-name diagnostic.

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
match quality.

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
clauses that all name the same binding. Top-level `or` may also join direct
branches when every branch names the same binding, such as
`candidate == fallback or candidate >= fallback`.
Literal `false` disjuncts do not affect direct repair matching, so
`false or candidate == fallback` has the same repair status as
`candidate == fallback`.
Wrapping each direct clause in balanced parentheses does not change this
repair match. Negated direct equality, disequality, and ordering clauses are
normalized before direct repair matching, so `not (candidate != fallback)` and
`not (candidate < fallback)` both mark `fallback` as safe. Literal `true`
conjuncts do not affect direct repair matching, so
`candidate == fallback and true` has the same repair status as
`candidate == fallback`. The accepted tautological clauses compare the satisfy
candidate with itself using `==`, `<=`, or `>=`, such as
`candidate == candidate`; their negated inverse forms, such as
`not (candidate != candidate)`, are also accepted. `and` may join only
tautological clauses or literal `true` clauses. Wrapping each tautological
clause in balanced parentheses does not change this repair match. Literal
`false` disjuncts do not affect tautological repair matching. A top-level
literal `true` disjunct makes the whole `satisfy` predicate tautological for
repair ranking, so every type-compatible visible binding is a safe repair
candidate.
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
same substitution. Negated ordering clauses normalize into their inverse
comparisons before matching; for example, `not (candidate < 0)` matches
`max >= 0` after substituting `max`, and `not (candidate <= 0)` matches
`max > 0`. Strict ordering requirements also discharge the matching inclusive
ordering predicate for the same operands. For example, a `require max > 0`
clause guarantees `candidate >= 0` after substituting `max`, and a
`require max < 10` clause guarantees `candidate <= 10`. Strict ordering
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
`max`. The same rule applies inside conjunctions:
`(max > 0 or max == 0) and max <= 10` guarantees
`candidate >= 0 and candidate <= 10` after substituting `max`. Nested `or`
branches inside `satisfy` conjunctions are accepted when at least one branch is
guaranteed; for example, `require max > 0 and max <= 10` guarantees
`(candidate > 0 or candidate == 0) and candidate <= 10` after substituting
`max`. Every type-compatible visible binding candidate for the tautological
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
