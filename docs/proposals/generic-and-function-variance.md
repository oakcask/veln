---
role: proposal
update-when: Checked source-ADT variance classification or compiler-owned covariant inference, their assignment or inference consumption, dependency variance metadata, variance presentation, nested incompatibility diagnostics, or analysis and language-service failure preservation change.
---

# Generic And Function Variance

## Summary

This proposal contains the remaining work for named generic variance. Current
assignment and implemented covariant inference behavior is defined by the
[type specification](../specification/types.md#compatibility-and-limits) and
its [inference rules](../specification/types.md#inference-rules). The remaining
work adds bounded inference beyond source-ADT and compiler-owned covariant
joins, dependency invalidation, detailed nested incompatibility context,
presentation, and failure preservation. It does not add declaration-site
variance modifiers.

## Related Work For Remaining Inference

The inspected primary source is the Rust Project's
[variance discussion in the Rustonomicon](https://doc.rust-lang.org/nomicon/subtyping.html#variance).
Its variance table classifies function inputs as contravariant and function
results as covariant. Its function examples explain that a replacement may
return a more specific result but must accept every input accepted by the
expected function. The same table shows that an owning collection can be
covariant while mutation through an alias can require invariance.

Veln adopts the input and output polarity finding for the remaining inference
design, but not Rust's type relation as a whole. Rust uses lifetime subtyping
and ownership or aliasing properties that are not Veln's nominal ADT identity,
finite variant-refinement subset relation, or immutable source-value model.
The source therefore supports the direction in which constraints compose; it
does not establish which Veln refinement is a join or meet, nor does it make
Rust's mutation rules an authority for Veln assignment.

Consequently, the design below collects lower bounds from positive occurrences
and selects their bounded join `J`, collects upper bounds from negative
occurrences and selects their bounded meet `M`, and requires equality for an
invariant occurrence. An irrelevant occurrence contributes no inference
constraint, but context-free joining still requires equal irrelevant arguments
because each complete argument remains part of nominal type identity.
The same polarity makes function inputs use `M` and results use `J`. When a
required bounded join or meet is not available among Veln's contributed types,
the design requires explicit context instead of importing a lifetime,
ownership, union, intersection, or bottom-type relation from Rust.

## Generic Inference And Joins

Assignment compatibility does not promise unrestricted subtype inference.
A concrete expected type supplies the target; every contribution must satisfy
it under the rules above. Without that context, the current type specification
defines equal-type and same-constructor source-ADT or compiler-owned covariant
joins. Inference uses the following remaining bounded, order-independent rules
when a recursive join goes beyond those implemented cases. It must inspect all
successful contributions before concluding that a required join is unavailable.

Write `J(S)` for the selected common supertype of a nonempty collection of
complete contributing types. Write `M(S)` for its bounded common subtype.
Normalize aliases and effect/field ordering before applying this table.

| Contributions | Selected result |
| --- | --- |
| Covariant argument beyond the implemented source-ADT and compiler-owned cases | `J` of the argument contributions. |
| Contravariant argument | `M` of the argument contributions. |
| Invariant or irrelevant argument | Require equal arguments for context-free inference. Otherwise require an explicit expected type; do not invent an argument or erase its identity. |
| Records with identical field names | Join each corresponding field with `J`. |
| Functions with identical fixed count and variadic presence | Use `M` for each input, `J` for results, and set union for concrete effects. |
| Other positive contributions, including differently shaped records | Use a contributed type that all contributions satisfy and that satisfies every other such contributed candidate. Require a unique resolved identity. Otherwise the join is unavailable. |
| `M(S)` | Use a contributed type that satisfies every contribution and that every other such contributed candidate satisfies. Require a unique resolved identity. Otherwise the meet is unavailable. |
| A required recursive `J` or `M` is unavailable | Require a concrete annotation; do not synthesize a general union, intersection, or bottom type. |

The structural rows take precedence over the fallback candidate row. They
must succeed in every required component; failure does not silently discard a
field or replace a component by `unknown`. This is intentionally bounded
inference, not a claim to compute every representable least upper bound.
Record width remains valid for assignment even when context-free joining of
incomparable record shapes requires an annotation.

For example, functions accepting `State::Ready` and `State` can join with the
`State::Ready` input, because that contributed input satisfies both. Functions
accepting disjoint singleton refinements cannot infer an input intersection;
an explicit compatible context is required. With callback inputs differing
only by pure and `stdio` effects, `M` chooses the pure callback type. With
record inputs `{id: Int}` and `{id: Int, label: String}`, `M` chooses the latter.
These choices apply regardless of contribution order.

Generic argument inference collects lower, upper, and equality constraints
from positive, negative, and invariant occurrences respectively. Irrelevant
occurrences contribute no inference constraint. An exact constraint fixes the
argument and all other bounds must accept it. Otherwise select `J` of the lower
bounds when any exist and check every upper bound. With only upper bounds,
select `M`. With no concrete constraint, or an unavailable join/meet, require
explicit context using the existing ambiguity diagnostics. Omitted private
signatures and local bindings remain monomorphic; variance does not turn a
single inferred binding into an independently instantiated generic value.

A remaining structural or meet contribution must be transactional. A rejected
payload or expression must not change previously admitted type facts or alias
provenance. Successful inference over the same contributions must have the
same resolved type and presentation in every order.
Diagnostic recovery may report errors at their original source locations; it
must not use a failed contribution to justify another accepted assignment.

## Remaining Invalidation, Presentation, And Failure

| Surface | Selected behavior |
| --- | --- |
| Declaration changes | Recompute affected variance contracts and invalidate dependent analysis before accepting downstream assignments. A stale classification cannot survive a payload change. |
| Source, formatter, package declaration display | Keep `type Name<T...>`; emit no inferred variance modifiers or replacement type names. |
| Diagnostics, LSP, MCP | Show ordinary types and existing alias preferences. Equal target identity with differing aliases is not a mismatch. Mixed alias preferences in inferred joins use the canonical target name permanently for that inference. |
| Nested incompatibility | Use `type.mismatch` at the failed value boundary with the actual and expected outer types. Put the failing nested path, comparison direction, and expected declaration location in related context, with corresponding JSON detail. |
| Direct refinement exclusion | Retain `type.variant_mismatch` and its existing excluded-variant facts. Do not report a nested incompatibility as a false outer variant exclusion. |
| Existing schema-specific diagnostics | Retain the generated-encode boundary's more specific refinement diagnostic; variance does not remove boundary provenance. |
| Failed analysis or assignment | Publish applicable diagnostics without executing the rejected program or exposing a partial successful signature. Preserve any retained successful language-service snapshot under its existing failure rules. |

Nested diagnostic paths distinguish named argument index, record field,
fixed input index, variadic element, result, and effect set. They describe the
first failed comparison in declared parameter order, field-name order, and
input-before-result-before-effects order, so alias spelling and inference
traversal cannot choose different facts. Related notes explain polarity and
repair context; the primary message remains the specific assignment failure.

## Acceptance Model

The producer performance regression uses
a nested covariant `Pair` comb at depths 128 and 256. When depth doubles, the
tracked factor-evaluation and dependency-edge counts may grow by at most twice
the smaller count plus eight. Elapsed time is diagnostic output and is not an
acceptance threshold.

The assignment lookup regression measures 256 and 512 public aliases against
source-ADT nesting depths 256 and 512. Doubling only the alias count can at
most double registry-construction candidate scans plus eight and does not
increase assignment index lookups. Doubling only the nesting depth can at most
double assignment index lookups plus two and does not increase construction
scans. Doubling both dimensions keeps each count equal to its corresponding
single-dimension result, and the combined work can at most double the baseline
plus ten. Elapsed time is diagnostic output and is not an acceptance threshold.

The evidence below is planned, not implemented or passing. The tables above
remain the independent acceptance oracle only for the remaining behavior.

| Contract | Planned evidence and required observation |
| --- | --- |
| Bounded inference | Permuted cases cover annotation-required contravariant meets, structural joins, and transactional exclusion of failures outside the implemented source-ADT and compiler-owned joins. |
| Metadata invalidation | Package fixtures prove that a variance-changing payload edit invalidates dependent analysis before subsequent inference. |
| Presentation and diagnostics | Human and JSON fixtures cover nested paths, reversed input comparison, direct exclusions, aliases, modifier-free declarations, and stable package/LSP/MCP presentations. |
| Failure preservation | Analysis and language-service regression cases prove no partial signature escapes rejection, retained successful snapshots survive applicable failures, and rejected programs do not run. |

Compiler evidence belongs in the existing semantic test suites and
`examples/specification/check/` with diagnostic companions. Run the existing
executable specification harness and focused package, LSP, and MCP suites for
their respective contracts.

Once the remaining checks pass, update the named-generic variance and inference
sections in the current type specification, together with executable examples.
Then retire this proposal and its catalog entry.

## Scope And Implementation Basis

This proposal is independently ready. It does not depend on the remaining
refinement-flow, repair, or handler proposals. The current structural
record/function assignment kernel, compiler-owned immutable constructor joins,
and source-ADT covariant joins are its implemented prerequisites.
It excludes mutable references, declaration-site variance syntax, general
unions/intersections, higher-kinded types, predicate-based subtyping, and
runtime representation changes solely for variance.

The inspected local implementation anchor is
`crates/veln-sema/src/aggregate_type_join.rs` for current refinement and
compiler-owned constructor joins. It identifies the shared inference boundary.
The proposal tables define only the intended behavior that remains beyond the
implemented covariant joins.
