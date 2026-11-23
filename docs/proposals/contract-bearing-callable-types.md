---
role: proposal
update-when: Contract-bearing callable syntax, binder scope, alpha-equivalence, implication-based assignment, joins, public interface retention, or callable prerequisite acceptance evidence changes.
---

# Contract-bearing Callable Types

Retain a callable's preconditions and postconditions when it becomes a value,
is passed as a callback, or appears in a public type. This prevents ordinary
function assignment from erasing caller obligations. This is a language design
proposal, not current behavior or a Ready implementation target.

## Scope and dependencies

This proposal owns P3 from
[Static Contracts for Composable Calls](static-contracts.md). It depends on
[Typed Predicate Logic and Flow Facts](typed-predicate-facts.md) for typed
predicates, substitution, value identity, and implication outcomes. That model
does not depend on callable types. The dependency graph is predicate facts,
then callable interfaces, then the complete static-contract feature.

This proposal owns callable binder syntax, pre/post type identity, assignment,
contextual typing, joins, and public retention. The static-contract consumer
owns declaration-clause enforcement, verification of source bodies and
recursive summaries, static diagnostics, executable adapters, and publication
of verified summaries. Examples use its proposed `static` clause mode. A
contract in a type describes the obligation and promise; its presence does
not prove that an arbitrary implementation satisfies the promise.

The type model and grammar prototype may provide prerequisite evidence before
the consuming verifier is complete. They must not expose callable values whose
static requirements can be bypassed or whose ensures are trusted without
verification. No user-visible implementation target is complete without the
consumer's call, return, and trust-boundary acceptance cases.

## Current behavior and investigated boundary

[Types](../specification/types.md#compatibility-and-limits) currently defines
ordinary function compatibility, and
[Contracts](../specification/contracts.md) defines executable declaration
clauses. At the investigation base `9ea04547d`, `Type::Function` in
`crates/veln-sema/src/semantic_model.rs` and `FunctionSignature` in
`crates/veln-sema/src/types/signatures.rs` carry ordinary types and effects,
but not bound pre/post predicates. Remembering only the originating named
function cannot preserve contracts through records and indirect callbacks.

## Proposed callable interfaces

A candidate type spelling is:

```text
(fn(value: Int) -> result: Int
    require static (value > 0)
    ensure static (result > 0))
```

Names in this form bind within the callable type. Parameter renaming does not
change type identity. Outer parentheses delimit the complete callable type in
records, generic arguments, and nested functions. Declaration clauses and type
clauses denote the same pre/post pair. Effect rows retain their existing role.
The spelling is subject to this proposal's grammar/formatter gate; it is not an accepted
source grammar. The grammar prototype must settle clause ordering, nesting,
line breaks, and the interaction with effect rows before Ready selection.

For compatible ordinary input/result types and effects, let a source callable
have `(Ps, Qs)` and a target callable have `(Pt, Qt)`. Assignment requires:

- `Pt` implies `Ps`: the source cannot demand more than the target allows.
- Under `Pt`, `Qs` implies `Qt`: the source must deliver the target's promise.

The existing input contravariance, result covariance, and effect compatibility
rules also apply. Logical implication failure or unknown implication rejects
assignment. A source requiring `x > 0` cannot become plain `fn(Int) -> Int`,
whose static requirement is `true`. A source requiring `x >= 0` can serve a
target requiring `x > 0`. A stronger postcondition can be forgotten; a stronger
requirement cannot be erased. Function joins retain only a contributed
callable type justified by these rules; they do not drop requirements to make
a join succeed. Otherwise an explicit compatible annotation is required.

The first slice supports monomorphic, fixed-arity callable contracts over the
admitted scalar paths. Generic proof abstraction, variadic contract binders,
and effect-row polymorphic contract entailment remain outside the slice.
Unsupported boundaries receive an explicit unsupported-static-contract error,
not a contract-free approximation.

## Binder and contextual boundaries

Each parameter and named result belongs to its callable binder. Resolve
predicate references before substitution; equal spelling in another callable
or imported declaration does not identify the same value. Alpha-renaming
bound parameters and results preserves the contract. The result binder is
visible only to postconditions. In the initial monomorphic form, predicates
may refer only to that callable's own parameters/result and their admitted
record fields. Capturing an outer callable's binders or an implementation
local is unsupported; do not publish a type with escaped local identities.

An expected callable type supplies a compatibility target. A function value
with no declared verified ensure cannot acquire that ensure merely from its
expected type. Any future contextual body verification must explicitly prove
the promise under the expected requirements; an annotation alone cannot mint
facts. Nested callable nodes are checked in the structural direction of their
position, including ordinary input contravariance and result covariance.

Joins consider contributed callable candidates using the assignment relation.
Select a unique candidate modulo alpha-equivalence when every contribution
assigns to it. If multiple inequivalent candidates remain, or no candidate is
admissible, require compatible explicit context. A join does not synthesize a
contract-free approximation or depend on branch order. Type identity includes
bound contracts; semantic implication establishes compatibility without
claiming that all logically equivalent predicate spellings have identical
type identity.

## Public retention and trust boundary

Retain the complete bound contract recursively through aliases, parameters,
results, records, and supported containers and imported interfaces. Apply
visibility checks to every referenced type identity. A public type cannot
expose private local proof assumptions. A mode/predicate-only interface change
must invalidate compatibility and dependent checking even when ordinary input,
result, and effect types are unchanged.

Generated documentation must render nested callable contracts and their mode
without binder capture. The static-contract consumer owns the concrete
Markdown/JSON schema extensions and semantic publication gate. Retaining a
type in documentation is not independent verification of its implementation.

## Planned acceptance and verification

These cases are planned evidence, not existing passing fixtures:

| Input or change | Required observation |
| --- | --- |
| Rename parameter and result binders consistently | Preserve type identity modulo alpha-equivalence and assignment. |
| Reuse binder spelling in nested or imported callables | Keep identities distinct; reject outer-binder capture and result references in preconditions. |
| Source requires `x > 0`, target accepts every `Int` | Reject requirement erasure. |
| Source requires `x >= 0`, target requires `x > 0` | Accept if the ordinary types/effects also satisfy compatibility. |
| Source has a stronger verified ensure than the target | Allow forgetting the excess promise; reject or report unresolved implication when the target demands an unestablished promise. |
| An uncontracted function is assigned an expected positive-result type | Do not create a verified result fact from contextual type syntax alone. |
| Nested callable input/output positions and incompatible effects | Respect structural variance and effect compatibility at each node; proof compatibility cannot override an ordinary mismatch. |
| Branch-order reversal with one uniquely admissible contributed candidate | Select the same alpha-equivalent result. Ambiguous candidates require explicit context. |
| Alias, record field, container, returned callback, and imported callback | Retain the bound contract recursively; test indirect calls through the static-contract consumer. |
| Change only a public mode or predicate | Recheck dependent compatibility and regenerate nested contract documentation; no stale interface acceptance. |
| Generic proof abstraction, variadic contract binders, or polymorphic contract entailment | Report explicit unsupported use; never approximate it with an uncontracted callable. |

Before Ready selection, provide accepted/rejected source-surface fixtures and
formatter round trips settling clause order, nesting, parentheses, line breaks,
and effect-row placement. Use focused type-model tests for alpha-equivalence,
substitution, assignment, contextual boundaries, and joins. Test nested
interfaces and failed compatibility without leaking tentative facts. The
static-contract proposal additionally owns observable direct/indirect/imported
call diagnostics, source verification, lowering, and documentation/publication
cases; passing the type tests alone does not establish those guarantees.

## Related work and alternatives

| Inspected primary source | Relevant finding and limits | Design consequence |
| --- | --- | --- |
| Dafny community, [Arrow types](https://dafny.org/dafny/DafnyRef/DafnyRef.html#sec-arrow-subset-types) | Arrow subsets distinguish precondition-bearing functions and heap-read boundaries. Veln does not adopt Dafny's heap verification model. | Preserve requirements through callable values; use explicit bound pre/post interfaces while deferring heap predicates. |
| LiquidHaskell project, [Abstract and Bounded Refinements](https://ucsd-progsys.github.io/liquidhaskell/specifications/#abstract-and-bounded-refinements) | Refinement subtyping uses implication constraints; its abstraction infrastructure differs from Veln. | Use the shared predicate model for pre/post compatibility and defer generic predicate abstraction. |

Direct-call-only tracking fails when a callback loses its originating
declaration. Rejecting every contracted function value would avoid erasure
but prevent the higher-order composition required by the consumer. The chosen
language change retains contracts in types, with acceptance evidence required
before exposing them as a verified feature. Sources motivate the distinction;
they do not establish Veln grammar or verification correctness.

## Readiness and lifecycle

Remain Design In Progress until the predicate prerequisite, callable grammar,
assignment/join decisions, public-interface rules, and feasibility evidence are
reviewed. Do not add either prerequisite or this consumer to Ready solely
because a prototype exists. During implementation, promote checked type
behavior to the current type/source-surface specifications and executable
consumer cases. Remove completed scope and this catalog entry when no active
scope remains; route dependents to the implemented specification.
