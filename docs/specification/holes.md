---
role: specification
authority: normative
specification-coverage: usage=#usage; behavior=#candidate-records-and-ranking; limits=#requirement-matching-limits
update-when: Hole diagnostics, expected-type propagation, satisfy constraints, candidate ranking, or repair candidate behavior changes.
---

# Holes

A hole marks an expression that still needs a value. Checking reports its
expected type and advisory repairs; a reachable hole blocks execution.

## Usage

```veln
fn choose(fallback: Int) -> Int
	_ satisfy candidate => candidate == fallback
end
```

`veln check --json` reports a partial result and suggests replacing `_` with
`fallback`. The source remains unchanged. A named hole such as `_port` labels
the missing expression but does not introduce a binding.

## Hole diagnostics and expected types

A hole produces `hole.unfilled` with severity `hint`. A check result containing
only non-error hole diagnostics has top-level status `partial`. The detail
record contains:

| Field | Meaning |
| --- | --- |
| `phase`, `node_id` | Analysis phase and source node. |
| `label` | The written hole label, including `_`, or `null` for an unnamed hole; it is not a binding. |
| `expected_type` | The rendered required type, or `unknown`. |
| `expected_type_source` | Declared boundary or inferred expression context. |
| `constraints` | Type and repair constraints discovered for the hole. |
| `local_bindings` | Visible names considered by candidate lookup. |
| `candidate_queries` | Advisory lookup and repair records. |

Concrete expected types flow from function returns, call arguments, record
fields, `if` branches, `match` arms, and ADT constructor payloads. The human
diagnostic and `details.expected_type` use the same rendered type. A hole with
no concrete context retains `expected_type: "unknown"` and the unknown-type
message.

## Candidate records and ranking

Every candidate query is advisory and has `candidate_status: "query_only"`.
Its default `application_policy` is `manual_review_required`. A known expected
type may produce ranked visible-binding candidates with these fields:

| Field | Meaning |
| --- | --- |
| `candidate_id` | `symbol-N`, where `N` is the one-based rank. |
| `name`, `type` | Binding name and rendered type. |
| `rank`, `reason` | Ranking position and type-match or satisfy-discharge reason. |
| `application_policy`, `application_status` | Review policy and `"unapplied"`. |
| `edits` | Replacement edits with `kind`, source `span`, and `replacement`. |
| `target` | Hole `node_id` and source `span`. |
| `edit_summary` | Human explanation of the replacement. |
| `evidence`, `known_limits`, `blocking_obligations` | Type/ranking evidence and outstanding verification or review. |
| `verification_hint` | Check `command` and scope `"after_applying_candidate_edit"`. |
| `satisfy_status` | Present when the checker constructed a static repair constraint; see the limits below. |

Exact type matches rank before broader assignable matches. Equal-quality matches
rank nearer visible bindings before older ones. A broader match discharged by a
`satisfy` constraint reports that discharge reason instead of the broad type
reason. Candidate generation retains the first five ranked candidates and any later
statically discharged `satisfy` candidates.
Each candidate retains the branch that discharged it when different branches
select different symbols. Safe candidates remain unapplied and retain a
verification obligation until the edit is applied and the verification hint is
run.

Named holes such as `_port` are labels only. They do not bind a name in the
expression or alter name resolution.

## Satisfy syntax and binding

The optional suffix is `satisfy candidate => predicate`. It contributes a
repair constraint and does not bind `candidate` outside the suffix. The suffix
must contain a candidate name and `=>`; missing parts report
`parse.satisfy_candidate` or `parse.satisfy_arrow`. The candidate cannot shadow
a visible binding, parameter, explicit result binding, or compiler-known
prelude helper (`hole.satisfy_candidate_shadow`), and the predicate must refer
to it (`hole.satisfy_candidate_unused`).

The predicate uses the same pure boolean subset as contracts. The candidate has
the hole's expected type when known. Unsupported constructs, non-boolean
predicates, missing fields, and unresolved names are hole diagnostics; names
unresolved within the predicate use the `satisfy_predicate` namespace.

## Safe repair classes

A visible symbol is a `safe_repair_candidate` only when substituting it for the
satisfy candidate makes the directly checked predicate reflexive, tautological,
or guaranteed by valid function-entry `require` or `invariant` clauses. All other candidates stay
manual-review candidates.

| Class | Accepted behavior |
| --- | --- |
| Direct clauses | Equality and inclusive comparison with one visible binding, in either operand order; matching field-access suffixes are allowed. `and` joins clauses for the same binding. |
| Branches | Top-level `or` checks each direct branch. Nested direct `or` sets are intersected across an `and`; a symbol is safe only when it is safe in every required branch. Literal `false` branches and statically false branches are ignored. |
| Normalization | Balanced parentheses, redundant atom parentheses, double negation, inverse equality/order negation, literal `true` conjuncts, and negated conjunction/disjunction forms are normalized before matching. Same-shape expressions compare after whitespace normalization. |
| Tautologies | Candidate self-equality or inclusive comparison, complementary boolean/comparison/order branches, trichotomy, inclusive totality, contradictory bounds, and contract static-truth identities make every type-compatible visible symbol safe. |
| Requirement discharge | Substitution is checked against valid `require` and `invariant` clauses, excluding `ensure`. Direct and commuted comparisons, field atoms, aliases, conjunctions, disjunctions, negated forms, and transitive order evidence may discharge the substituted predicate. The reason is `satisfy_require_match`. |

Representative safe predicates are:

```text
satisfy candidate => candidate == fallback
satisfy candidate => candidate.count <= fallback.count
satisfy candidate => candidate == fallback or candidate == other
satisfy candidate => candidate == candidate
```

The same-shape rule can accept `candidate + 1 == fallback + 1`. A nested
branch such as `(candidate == fallback or candidate == max) and
(candidate == fallback or candidate == spare)` selects only the intersection,
`fallback`. A top-level tautology such as `candidate.ready or not candidate.ready`
makes every type-compatible visible binding safe.

## Requirement matching limits

Requirement discharge reuses contract static truth and preserves its limits:

- equality aliases may connect operands, including aliases inside same-shape
  expressions and boolean field atoms;
- strict order implies the corresponding inclusive order and disequality;
- non-strict paths in both directions imply equality, while strict paths do not;
- transitive chains preserve strictness when any edge is strict;
- disjunctive requirements contribute only facts common to every branch;
- numeric literal bounds use exact decimal/rational ordering and pure literal
  `+`, `-`, `*`, and comparison-only `/` expressions;
- `Int` bounds include adjacent integer facts (`max > 0` implies `max >= 1`,
  while `max >= 10` does not imply `max > 10`); and
- equal inclusive bounds do not imply endpoint disequality.

For example, `require max > 0` can discharge `candidate >= 0` or
`candidate != 0` after substitution, while `require max >= 10` cannot discharge
`candidate > 10` or `candidate != 10`. Conflicting evidence leaves the
candidate under manual review. A statically discharged candidate has
`satisfy_status: "statically_satisfied"`. Its `reason` is
`satisfy_equality_match`, `satisfy_reflexive_match`, `satisfy_tautology`, or
`satisfy_require_match`, according to the matching rule. Candidates outside a
constructed static repair constraint have
`satisfy_status: "blocked_until_discharged"` and require manual review.
When no static repair constraint can be constructed, candidate records omit
`satisfy_status`; the hole's `constraints` record still reports
`repair_status: "blocked_until_discharged"`. Absence of candidate-level status
does not authorize application.

## References

- Hole diagnostics: `crates/veln-sema/src/holes.rs`.
- Candidate fields, ranking, and entry-clause selection:
  `crates/veln-sema/src/analysis/body/diagnostics_and_repairs.rs` and
  `crates/veln-sema/src/analysis/repair_reasoning/`.
- Predicate normalization and requirement discharge:
  `crates/veln-sema/src/contracts/`.
- JSON field definitions and applying-command gates:
  [diagnostics-json.md](diagnostics-json.md),
  [repair-candidates.md](repair-candidates.md), and
  [repair-application.md](repair-application.md).
