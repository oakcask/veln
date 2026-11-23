---
role: proposal
update-when: The typed predicate grammar, scalar value identities, flow-fact transfer, reachability, bounded entailment outcomes, or prerequisite proof-model acceptance evidence changes.
---

# Typed Predicate Logic and Flow Facts

Provide a shared language-semantic model for facts about immutable `Bool` and
`Int` values. Consumers can ask whether assumptions establish a predicate
without confusing repeated source spellings, branch-local facts, or runtime
integer semantics. This is planned behavior, not implemented behavior or a
Ready target.

## Scope and dependency boundary

This proposal is the prerequisite authority for typed logic and flow facts
used by [Contract-bearing Callable Types](contract-bearing-callable-types.md)
and [Static Contracts for Composable Calls](static-contracts.md). It owns P1
and P2 from the latter proposal. It does not depend on either consumer for its
truth/order model or transfer rules.

A consumer supplies validated entry assumptions, call summaries, and proof
queries. This proposal defines their interpretation and transfer, not the
right to trust them. Static-clause syntax, call/return enforcement, body and
recursive-summary verification, diagnostics, and executable publication belong
to the static-contract consumer. Callable assignment belongs to the callable
type proposal. These links identify consumers, not reverse prerequisites.

There is no new standalone proof command or assertion primitive. The
independent model and sema tests are prerequisite feasibility evidence. A
user-visible implementation target must also select a complete consuming
feature and its observable acceptance cases; passing model tests alone cannot
publish a verified-language feature. Existing runtime clauses retain their
current classification and runtime fallback.

## Current behavior and investigated boundary

[Contracts](../specification/contracts.md#static-classification) owns current
text-based classification. Its ability to omit a proven runtime check is not
a binder-aware entailment interface. [Types](../specification/types.md) owns
current ADT refinement and alias behavior; it does not specify propagation of
integer inequalities through ordinary branches.

At the investigation base `9ea04547d`, `crates/veln-sema/src/contracts.rs`
classifies predicate text, while
`crates/veln-sema/src/analysis/body/control_flow_results.rs` joins ordinary
branch types. The JVM `emit_binary` and `emit_contract_binary` implementations
in `pattern_and_value_emission.rs` and `contract_emission.rs` under
`crates/veln-backend-jvm/src/classfile/` evaluate both operands left to right.
The proposed proof model preserves that evaluation behavior; it adds neither
short-circuit operators nor a numeric representation change.

## Proposed predicate and flow model

Examples mentioning `static`, `consume`, or `normalize` use the proposed
consumer syntax and definitions from
[Static Contracts for Composable Calls](static-contracts.md#proposed-source-surface-and-guarantee).
They do not introduce clause syntax in this proposal.

Predicates admitted to the shared proof language contain:

- Boolean literals, stable Boolean paths, `not`, `and`, and `or`;
- equality/disequality of matching `Bool` or `Int` operands;
- signed `Int` ordering between literals and stable scalar paths;
- parentheses and signed integer literals within the `Int` range;
- parameter and named-result references, including immutable record fields.

A stable path denotes one immutable value, using resolved identity rather
than its displayed name. An argument expression is evaluated once according
to ordinary language evaluation; substitution refers to that value. Returned
values have fresh identities. Two calls with identical spelling do not acquire
an equality fact. Boolean aliases retain a relation to their originating
admitted condition until scope ends.

The mandatory proof fragment includes literal comparison, Boolean identities,
copy equality, equality substitution, order transitivity, weakening strict
bounds to inclusive bounds, and complementary branch conditions. The checker
need not decide every Boolean formula. More complex supported formulas can
produce an unknown proof result; bounded search must never accept them by
default. Before Ready selection, fix deterministic work limits and mandatory-fragment
cases using the feasibility evidence described below.

| Program event | Facts available afterward |
| --- | --- |
| Enter a checked body | Validated entry assumptions supplied by its consumer; no assumptions from an unverified result promise. |
| `let y = x` for an immutable scalar | Equality of these values; shadowed bindings remain distinct. |
| Construct, alias, or destructure an immutable record | Admitted scalar projections denote their evaluated initializer or source-field values, including nested fields. No whole-record logical equality is required. |
| Enter the true/false branch of an admitted condition | The condition / its negation, scoped to that branch. |
| Join continuing branches | Only facts justified on every continuing path; an exited branch does not constrain the continuation. |
| Return a branch expression | That path's facts and its actual returned value for checking the consumer's result obligation. |
| Complete a call with a validated summary | The instantiated postcondition over evaluated arguments and the returned value. The consumer must establish summary validity. |
| Match an ADT | Existing variant refinement rules remain available; no arbitrary payload predicate is inferred. |

### Evaluation and fact availability

Ordinary expressions and executable predicates keep their existing evaluation
order. Both operands of `and` and `or` evaluate, left before right. Check a
call using facts established before that call executes. A preceding operand's
verified postcondition is available, but its Boolean result is not assumed
true or false while checking the other operand. A later operand supplies no
facts to an earlier call. Boolean simplification of a static predicate does
not permit skipping calls or other evaluation in ordinary code.

For example, suppose `positive_check(x)` returns `Bool` and requires static
`x > 0`. Neither `x > 0 and positive_check(x)` nor
`x <= 0 or positive_check(x)` establishes that call's requirement for an
unconstrained `x`. Reject both, including when they occur in an executable
contract. An enclosing `if x > 0` provides the missing fact. A future
short-circuit language change would need separate evaluation and proof rules.

An ordinary computation outside the proof language produces an opaque value
of its checked type. The ordinary computation remains permitted; writing that
computation directly in a static predicate remains unsupported. For example,
a saved arithmetic result can be compared with zero in an ordinary branch;
that branch proves a fact about the saved result, without proving an
arithmetic relation to its operands.

### Records and expression results

Record construction relates each admitted scalar field path to the value of
its evaluated initializer. For `let r = {value: normalize(x)}`, the verified
positive-result fact therefore applies to `r.value`. A transparent record
alias preserves the same field values. A record pattern binding denotes its
selected source field. These rules apply recursively to nested immutable
records and to parameters and returned records. An annotation may forget
fields but cannot change the identity of retained field values. Missing fields
remain ordinary type errors, and distinct roots or field sequences share no
facts without a demonstrated value relation.

When an `if`, `match`, or `begin` expression supplies a value to a binding or
return, relate that expression's result to the value produced on each
continuing path. Keep a result fact only when every continuing path justifies
it. Thus assigning the positive branches of `normalize` to a local before
returning it must not lose positivity. Facts about an escaping result and
still-visible values can survive lexical-scope exit; references to hidden
locals cannot escape. These are required scalar/record transfer rules, not
general refinement carriers for arbitrary ADT payloads.

### Reachability and contradictory assumptions

A path is proved unreachable when supported facts entail `false`, or when
ordinary checked control flow has no continuation. Failure to decide
reachability is not evidence that a path is unreachable. On a proved
unreachable path, call and return proof obligations are vacuously proven.
They take precedence over a refuted-predicate classification because no
execution reaches that obligation. No assumption from that path enters a
reachable sibling or continuation. If all incoming paths are unreachable,
the join has no reachable continuation.

Syntax, name resolution, ordinary type compatibility, static-predicate
admissibility, and declaration validation still apply in unreachable code.
Vacuity does not validate a hole, an unsupported predicate, or an invalid
dependency. Contradictory entry assumptions permit vacuous result obligations. A consumer
must not present that result as a termination or inhabitation guarantee.
Whether an assumption may be introduced, a callable may be assigned, or an
interface may be published belongs to the consuming feature.

For example, `consume(0)` inside both `if x > 0` and `if x <= 0` is vacuous,
but the same call after those branches is refuted on a reachable path. A
checked callee's `ensure static (false)` can make its normal continuation
unreachable. Merely detecting a recursive call cannot do so. The consuming
static-contract proposal uses partial correctness and permits a verified
non-returning function; it does not manufacture a returned value or justify
evaluating that function during proof search.

### Limits of the initial logic

Arithmetic can still occur in ordinary bodies. In the first slice an arithmetic
result supplies no arithmetic relationship to its operands unless represented
by a separately verified admitted summary. For example, `return x + 1` cannot
justify `ensure static (result > x)`. Returning a known positive literal can
justify `result > 0`. Neither mathematical-integer identities nor Float total
ordering may be assumed. Arithmetic inside a static predicate, Float facts,
collection measures such as length, quantifiers, arbitrary predicate calls,
logical lemmas, and mutable/host state observations are unsupported.

An effect-free function is not automatically total or safe to execute in a
proof. No arbitrary user function is unfolded or executed by the proof engine.
A verified Boolean-returning function may convey a relation through an admitted
postcondition, such as `result == (value > 0)`. This supports named validation
without introducing predicate calls into the logic.

## Iteration boundaries

Loop checking may retain established facts about unchanged outer immutable
values and the current iteration's supported guard. A binding or call result
evaluated anew in an iteration has a new value identity. A previous
iteration's branch facts cannot establish the next iteration's requirements
or a post-loop fact without a justified invariant. A query needing such a fact
reports unknown in the initial slice. Explicit loop invariants and termination
checking are outside this proposal.

## Entailment outcomes

For path facts `F` and obligation `P`, use the following decision model:

| Established observation | Outcome | Consumer interpretation |
| --- | --- | --- |
| `F` entails `false`, or checked flow has no continuation | Vacuously proven | No execution reaches the obligation. Do not export a value or contradictory facts to a reachable path. |
| On a reachable path, `F` implies `P` | Proven | The consumer may rely on `P` at that program point. |
| On a reachable path, `F` implies `not P` | Refuted | Every permitted execution violates the obligation. |
| Neither implication is established | Unknown | Do not claim either universal satisfaction or universal violation. |
| Deterministic work limit is exhausted before establishing an outcome | Unknown with budget reason | Never treat incomplete search as proof. |
| A predicate has invalid syntax, names, types, or unsupported logic | Invalid or unsupported | Do not add assumptions, including on a proved unreachable path. |

A single violating assignment is insufficient for a universally refuted
outcome. On an unknown obligation, a valid counterexample may illustrate why
the guarantee fails for some inputs without claiming failure for every input.
Check established unreachability before classifying a valid obligation as
refuted. A consumer decides which outcomes reject compilation; this proposal
does not replace runtime fallback for existing executable clauses.

## Planned acceptance and verification

These are planned model and sema cases, not passing tests:

| Input or transition | Required observation |
| --- | --- |
| `x > 0` in the true branch; its negation in the false branch | Prove the matching predicate only on its justified path. |
| An immutable scalar alias and a shadowed same-spelled binding | Transfer equality to the alias; transfer no fact to the shadowed value. |
| A saved value versus two separately evaluated calls | Retain saved-value identity; never infer equality from identical source spelling. |
| Nested record construction, root aliases, width annotations, and record patterns | Retain relations for surviving scalar fields; unrelated roots/fields supply no fact. |
| A positive result from every continuing branch, with one branch exiting | Preserve positivity through expression-result binding and scope exit; no hidden local identity escapes. |
| One reachable result branch changed to zero | Do not retain a positive fact at the join. |
| Eager Boolean operands, including a call requiring positivity | Do not assume one operand's truth while checking another. Earlier validated postconditions are available; later facts are unavailable. |
| `false`, or both `x > 0` and `x <= 0`, at an otherwise valid query | Report vacuity and keep it out of reachable siblings. Invalid names, types, predicates, and holes remain invalid. |
| `F` is `x >= 0`, and `P` is `x > 0` | Report unknown, not universally refuted. |
| Literal order, alias substitution, transitivity, strict-to-inclusive weakening, and minimum/maximum signed `Int` values | Establish the mandatory comparison cases using the independent order model. |
| Arithmetic directly in a predicate versus guarding a saved arithmetic result | Reject the former as unsupported; allow facts about the latter's evaluated value. |
| A backedge with a newly evaluated binding or call result | Do not carry an earlier iteration's local facts to the new value. |
| The same bounded query with cold/warm analysis and declaration order reversed | Preserve the logical outcome at the declared work policy; do not let cached authority or declaration order create proofs. |

Use an independent truth/order model for `Bool` and signed 64-bit comparisons,
and focused `veln-sema` transfer tests for identity, scope, record, branch, and
iteration behavior. The consuming proposal owns CLI, runtime, lowering, and
publication cases. Its eager-evaluation cases must observe skipped, duplicated,
or reordered operands, rather than checking only the final Boolean value.

Before Ready selection, define the accounting scope, unit, defaults, and
versioning of deterministic work limits and the exact mandatory-fragment
cases. Evaluate long Boolean formulas, deep aliases/record paths, and repeated
queries at doubled sizes against a contract-free control. Record retained
facts and proof steps and demonstrate a non-exponential work bound or a
deterministic unknown cutoff. Numerical limits require measured feasibility
evidence; they are not chosen here. Use `bash scripts/agent-run` and the
repository performance-audit workflow for generated analysis workloads.

## Related work and alternatives

| Inspected primary source | Relevant finding and limits | Design consequence |
| --- | --- | --- |
| F* project, [Boolean refinement types and refinement subtyping](https://fstar-lang.org/tutorial/book/part1/part1_getting_off_the_ground.html#boolean-refinement-types) | Refinement introduction proves a substituted predicate. Its mathematical integers differ from Veln's runtime representation. | Use value-aware substitution and implication; keep arithmetic outside the signed-comparison slice. |
| LiquidHaskell project, [Abstract and Bounded Refinements](https://ucsd-progsys.github.io/liquidhaskell/specifications/#abstract-and-bounded-refinements) and [Termination Metrics](https://ucsd-progsys.github.io/liquidhaskell/specifications/#termination-metrics) | Refinements use implication constraints; measures and termination metrics have separate mechanisms. These do not provide a Veln proof implementation. | Keep a bounded comparison model and explicit consumer summaries; defer reflection, measures, and total-correctness claims. |

Extending text normalization alone cannot distinguish shadowed binders or
separately evaluated calls. Adopting a solver first would still require the
same identity, scope, and runtime-semantic decisions. The selected shared model
therefore precedes solver adoption without promising completeness for every
Boolean formula. Vacuity and record-transfer rules are proposed Veln decisions;
source comparisons do not demonstrate their implementation.

## Readiness and lifecycle

Remain Design In Progress until the typed grammar, mandatory cases, bounded
work policy, and transfer model have reviewed feasibility evidence. Consumers
must not publish unvalidated facts while this prerequisite is incomplete.
During implementation, promote the checked behavior into the matching current
contract/type specifications and executable consumer evidence. Remove completed
scope and this catalog entry when no active scope remains, and update consumer
links to the current specification.
