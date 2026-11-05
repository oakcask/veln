---
role: proposal
update-when: Source-ADT variance inference, bounded generic inference, dependency variance metadata, variance presentation, nested incompatibility diagnostics, or analysis and language-service failure preservation change.
---

# Generic And Function Variance

## Summary

This proposal contains the remaining work for named generic variance. The
compiler now computes and retains one checked variance classification per
source-ADT type parameter. It includes private payloads, recursive declarations,
and checked classifications inherited from dependencies. This metadata is not
yet consumed by assignment or inference. Veln will add no declaration-site
variance modifiers. The remaining work includes assignment consumption,
bounded generic inference, dependency consumption, detailed nested
incompatibility context, presentation, and failure preservation.

Recursive assignment through records and functions is current behavior under
the [type specification](../specification/types.md#compatibility-and-limits).
That implemented kernel makes record fields and function results covariant,
function inputs contravariant, and concrete function effect sets covariant.
The compiler-owned immutable `Option<T>`, `Result<T, E>`, `List<T>`, `Vec<T>`,
and `Dict<K, V>` constructors already apply assignment covariance as described
by the current
[type specification](../specification/types.md#compatibility-and-limits).
Current context-free aggregate, control-flow, and constructor inference still
rejects unequal complete arguments even when assignment covariance relates
them. The bounded joins proposed below are remaining behavior; they would
replace that current limitation rather than describe behavior already
implemented.

## Assignment Relation And Identity

Write `A <: B` when an actual value of complete type `A` may satisfy expected
type `B`. Write `A = B` for resolved structural type equality, including nominal
owner identity, type arguments, variant sets, and function effect sets, while
ignoring alias spelling and field or effect ordering. Equality is stronger
than mutual assignability: irrelevant generic arguments remain part of type
identity even when they do not restrict assignment.

The current structural relation supplies the polarity of each occurrence. The
remaining relation applies the selected parameter classification when two
instances have the same resolved named constructor. It adds no unrelated
nominal subtyping, numeric assignment conversion, predicate-derived subtype,
or runtime coercion. `unknown` remains an inference/recovery placeholder, not
a top or bottom type and not evidence that a variance constraint is satisfied.
Existing completeness checks must resolve required slots before admitting a
declaration for execution.

| Actual and expected types | Assignment condition |
| --- | --- |
| Instances of one generic declaration | Compare each argument by that declaration's selected parameter variance. |
| Same-ADT refinements, possibly with different arguments | Compare arguments by variance and require the actual variant set to be a subset of the expected set. Treat the base ADT as its complete variant set. |
| Instances of different nominal declarations | Reject, including identically spelled declarations in different modules. |
| Alias-qualified types | Resolve identity before applying variance; preserve presentation separately. |

A refinement uses its entire ADT declaration's variance, not a more permissive
classification inferred from only its currently selected variants. Assignment
never changes a stored value, its runtime constructor, its equality behavior,
or its iteration order.

## Constructor Classifications

For one generic argument, `A` is actual and `B` is expected:

| Classification | Required comparison |
| --- | --- |
| Covariant, written `+` in this document | `A <: B`. |
| Contravariant, written `-` | `B <: A`. |
| Invariant, written `=` | `A = B`. |
| Irrelevant, written `*` | No argument comparison; both arguments must still be well-formed types. |

These symbols are explanatory notation, not Veln source syntax.

| Constructor | Parameter classifications | Reason |
| --- | --- | --- |
| Source ADT, including source-backed standard types | Infer each parameter from every payload in the declaration. | Visibility and call sites do not change the classification. |
| Opaque or imported generic type with no checked variance metadata | Every parameter `=`. | Representation safety cannot be inferred from a name or helper signature. |
| Nongeneric resource or other nongeneric type | No parameter variance. | Its nominal identity still controls assignment. |

The checked semantic descriptor producer now derives classifications from all
payloads, composes nested checked classifications, and solves recursive source
declarations independently of declaration order. Checked descriptors provide
covariant classifications for `Option`, `Result`, and `List`. Producer-local
metadata keyed by compiler identity and arity provides the same classifications
for `Vec` and `Dict` without publishing new ADT descriptors. Focused semantic
tests are the executable evidence for that internal producer. The current type
specification and executable specification examples remain unchanged because
assignment and inference do not consume the metadata yet.

The definition-based inference and recursive constraint treatment follow the
[Rust Compiler Development Guide's variance discussion](https://rustc-dev-guide.rust-lang.org/variance.html).
That source infers data-type variance from definitions and referenced
definitions, uses a four-way variance lattice, and resolves recursive
constraints. Veln adopts the least restrictive sound result and excludes
call-site-dependent variance. Rust's trait, lifetime, and compiler-query details
are not part of this proposal.

The [Rustonomicon's subtyping and variance chapter](https://doc.rust-lang.org/nomicon/subtyping.html)
explains the function input/output directions and distinguishes owning
collection covariance from mutation through an alias. Veln adopts these
polarity directions in the current structural kernel and the remaining
variance consumption design, with immutable source values replacing Rust's
ownership conditions. Rust's lifetime relation supplies comparative reasoning,
not the ADT-refinement relation specified here. Treating every payload as
covariant is rejected because a stored callback can consume its parameter.
Treating every source constructor as invariant is safe but needlessly rejects
immutable producer cases.

## Generic Inference And Joins

Assignment compatibility does not promise unrestricted subtype inference.
A concrete expected type supplies the target; every contribution must satisfy
it under the rules above. Without that context, inference uses the following
bounded, order-independent joins. It must inspect all successful contributions
before concluding that a required join is unavailable.

Write `J(S)` for the selected common supertype of a nonempty collection of
complete contributing types. Write `M(S)` for its bounded common subtype.
Normalize aliases and effect/field ordering before applying this table.

| Contributions | Selected result |
| --- | --- |
| All resolved types equal | That type. |
| Same nominal constructor, with refinements allowed | Join variant sets by union, treating a base as the complete set; compute arguments by the following variance rows. |
| Covariant argument | `J` of the argument contributions. |
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

A payload contributes all of its constraints or none. A rejected payload or
expression must not change previously admitted type facts or alias provenance.
Successful inference over the same
contributions has the same resolved type and presentation in every order.
Diagnostic recovery may report errors at their original source locations; it
must not use a failed contribution to justify another accepted assignment.

## Remaining Metadata Consumption, Presentation, And Failure

| Surface | Selected behavior |
| --- | --- |
| Dependency type metadata | Source-package analysis computes one classification per declared parameter, keyed by resolved declaration identity and parameter order. Importers already retain these checked semantic descriptors, including classifications derived from hidden payloads. Remaining consumers will use them for assignment and inference. No metadata-only package input or serialized interface format is introduced. |
| Missing or invalid variance metadata | Classification production already uses invariance for unavailable classifications and never infers covariance from a displayed signature. Remaining assignment and inference consumers must preserve that fallback. Malformed dependency metadata still follows the package loader's ordinary rejection policy. |
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

Classification production is implemented and checked by focused semantic
tests. The evidence below is planned, not implemented or passing. The tables
above remain the independent acceptance oracle for consumers; current
assignment and inference behavior cannot supply the remaining expected results.

| Contract | Planned evidence and required observation |
| --- | --- |
| Variance-aware assignment | Assignment directions, same-spelled distinct ADTs, and refinement sets use the checked classifications without changing type identity. |
| Bounded inference | Permuted branch, collection, and payload cases cover successful joins, annotation-required meets, irrelevant slots, exact constraints, and transactional exclusion of failed contributions. |
| Metadata and identity | Package fixtures compare local and imported behavior, hidden payloads, absent metadata, and invalidation after a variance-changing payload edit. |
| Presentation and diagnostics | Human and JSON fixtures cover nested paths, reversed input comparison, direct exclusions, aliases, modifier-free declarations, and stable package/LSP/MCP presentations. |
| Failure preservation | Analysis and language-service regression cases prove no partial signature escapes rejection, retained successful snapshots survive applicable failures, and rejected programs do not run. |

Compiler evidence belongs in the existing semantic test suites and
`examples/specification/check/` with diagnostic companions. Run the existing
executable specification harness and focused package, LSP, and MCP suites for
their respective contracts. Recursive generated-input coverage must follow
the repository's bounded-run and performance-audit skills; this proposal sets
no unmeasured timing target.

Once the remaining checks pass, update the named-generic variance and inference sections
in the current type specification, together with executable examples. Then
retire this proposal and its catalog entry.

## Scope And Implementation Basis

This proposal is independently ready. It does not depend on the remaining
refinement-flow, repair, or handler proposals. The current structural
record/function assignment kernel and compiler-owned immutable constructor
covariance are its implemented prerequisites.
It excludes mutable references, declaration-site variance syntax, general
unions/intersections, higher-kinded types, predicate-based subtyping, and
runtime representation changes solely for variance.

The checked ADT variance producer is the implementation anchor for
classification metadata. The inspected local implementation anchors are
`crates/veln-sema/src/type_relations.rs` for current structural polarity and the
future application of source-ADT parameter classifications, and
`crates/veln-sema/src/aggregate_type_join.rs` for current refinement joins.
These identify the affected implementation boundaries; the proposal tables
define the intended replacement behavior.
