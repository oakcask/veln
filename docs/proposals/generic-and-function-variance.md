---
role: proposal
update-when: Constructor generic constraint inference, dependency variance metadata, variance presentation, nested incompatibility diagnostics, or analysis and language-service failure preservation change.
---

# Generic And Function Variance

## Summary

This proposal contains the remaining work for named generic
variance. The remaining work adds constructor generic constraint inference,
dependency invalidation, detailed nested incompatibility context, stable
formatter, package, LSP, and MCP presentation, and whole-analysis and
language-service failure preservation. Complete contributed types already use
the bounded joins and meets defined by the
[type specification](../specification/types.md#bounded-joins-and-meets); this proposal
does not reclassify those rules as future work. This proposal does not add
declaration-site variance modifiers.

## Related Work For Generic Constraints

The inspected primary source is the Rust Project's
[variance discussion in the Rustonomicon](https://doc.rust-lang.org/nomicon/subtyping.html#variance).
Its variance table classifies function inputs as contravariant and function
results as covariant. Its function examples explain that a replacement can
return a more specific result but must accept every input accepted by the
expected function.

Veln adopts that polarity for constraints produced by nested constructor
payloads, but not Rust's type relation as a whole. Rust's lifetime subtyping and
ownership model do not determine joins or meets for Veln's nominal ADTs and
finite variant refinements. Veln therefore selects only bounded results
available under the current type specification and requires explicit context
when no such result exists.

## Remaining Constructor Generic Constraint Inference

Constructor generic argument inference must collect constraints from each
nested occurrence of a type parameter. This is separate from joining complete
contributed types after their generic arguments are known.

Constraint polarity starts positive at each constructor payload template. It
stays unchanged through record fields and function results, reverses through
function inputs, and composes through resolved named-constructor parameters
using the current
[source-ADT parameter classifications](../specification/types.md#source-adt-parameter-classifications).
Missing or unusable classification metadata uses the current invariant
fallback.

| Nested occurrence | Planned constraint |
| --- | --- |
| Positive | Add a lower bound for selection by bounded join `J`. |
| Negative | Add an upper bound for selection by bounded meet `M`. |
| Invariant | Add an equality constraint. |
| Irrelevant | Add no inference constraint. |

Equality constraints must resolve to one identity and fix the argument `E`.
Every lower bound must be assignable to `E`, and `E` must be assignable to
every upper bound. Without an equality constraint, inference selects `J` from
the lower bounds when any lower bound exists, then requires `J` to be
assignable to every upper bound. With only upper bounds, inference selects
`M`. With no concrete constraint, or when the required bounded result is
unavailable, inference requires explicit context through the existing
ambiguity behavior. It does not synthesize a union, intersection, bottom type,
or `unknown` argument. Omitted private signatures and local bindings remain
monomorphic; variance does not independently instantiate one inferred binding
at each use.

Constraint collection and selection must be transactional. A rejected payload
must not change admitted bounds, the selected resolved identity, or alias
presentation. Permuting successful payload contributions must produce the same
resolved constructor argument and presentation.

The acceptance model uses constructors whose type parameters occur inside
positive results, negative function inputs, invariant input-and-result
positions, and irrelevant payloads. It observes the inferred complete
constructor type, the existing ambiguity diagnostic when `J` or `M` is
unavailable, and unchanged later inference after a rejected contribution.

## Remaining Invalidation, Presentation, And Failure

| Surface | Selected behavior |
| --- | --- |
| Declaration changes | Recompute affected variance contracts and invalidate dependent analysis before accepting downstream assignments. A stale classification cannot survive a payload change. |
| Formatter, package, LSP, MCP | Keep ordinary type presentation stable across these surfaces without exposing inferred variance modifiers or replacement type names. |
| Nested incompatibility | Use `type.mismatch` at the failed value boundary with the actual and expected outer types. Put the failing nested path, comparison direction, and expected declaration location in related context, with corresponding JSON detail. |
| Direct refinement exclusion | Retain `type.variant_mismatch` and its existing excluded-variant facts. Do not report a nested incompatibility as a false outer variant exclusion. |
| Existing schema-specific diagnostics | Retain the generated-encode boundary's more specific refinement diagnostic; variance does not remove boundary provenance. |
| Rejected analysis or assignment beyond a local join contribution | Publish applicable diagnostics without executing the rejected program or exposing a partial successful signature. Preserve any retained successful language-service snapshot under its existing failure rules. |

Nested diagnostic paths distinguish named argument index, record field,
fixed input index, variadic element, result, and effect set. They describe the
first failed comparison in declared parameter order, field-name order, and
input-before-result-before-effects order, so alias spelling and inference
traversal cannot choose different facts. Related notes explain polarity and
repair context; the primary message remains the specific assignment failure.

The remaining evidence below is planned, not implemented or passing. The
constructor constraint model and remaining-behavior table above are its
independent acceptance oracles.

| Contract | Planned evidence and required observation |
| --- | --- |
| Constructor generic constraints | Semantic tests and checked executable specification cases cover ordinary constructor-body inference and private call/signature inference with nested positive, negative, invariant, and irrelevant occurrences; mixed lower, upper, and equality constraints; explicit context and the existing context-free ambiguity diagnostic; complete rendered results in every contribution order; and transactional rejection. |
| Metadata invalidation | Package fixtures prove that a variance-changing payload edit invalidates dependent analysis before subsequent inference. |
| Presentation and diagnostics | Human and JSON fixtures cover nested paths, reversed input comparison, direct exclusions, and stable formatter, package, LSP, and MCP presentations. |
| Failure preservation | Whole-analysis and language-service regression cases prove no partial signature escapes rejection, retained successful snapshots survive applicable failures, and rejected programs do not run. |

Compiler evidence belongs in the existing semantic test suites and
`examples/specification/check/` with diagnostic companions. Run the existing
executable specification harness and focused package, LSP, and MCP suites for
their respective contracts.

Once the remaining checks pass, update the matching current specifications and
executable examples. Constructor constraint evidence must demonstrate nested
constraint collection rather than only joining already-complete types. Then
retire this proposal and its catalog entry.

## Scope And Implementation Basis

This proposal is independently ready. It does not depend on the remaining
refinement-flow, repair, or handler proposals.
It excludes mutable references, declaration-site variance syntax, general
unions/intersections, higher-kinded types, predicate-based subtyping, and
runtime representation changes solely for variance.

The current ordinary inference path is
`crates/veln-sema/src/analysis/body/adt_and_match.rs`; private call/signature
inference has the corresponding path in
`crates/veln-sema/src/types/private_inference/call_inference.rs`. Both send
nested payload contributions to
`crates/veln-sema/src/aggregate_type_join.rs` as exact constraints. These are
implementation anchors for the remaining solver, not current-behavior
authority.
