---
role: proposal
update-when: Generic named-type assignability, aggregate covariance, function subtyping, effect-subset composition, or variance verification changes.
---

# Generic And Function Variance

## Summary

This proposal selects recursive assignment compatibility for immutable
aggregates, source ADTs, and functions. The compiler infers source-ADT parameter
variance from payload declarations. Veln adds no declaration-site variance
modifiers. Compiler-owned immutable containers are covariant; function inputs
are contravariant and results are covariant.

All rules below are planned behavior. The current
[type specification](../specification/types.md#compatibility-and-limits)
allows direct ADT refinement widening but keeps refinement-bearing nested
positions identical. This proposal replaces that restriction only when its
acceptance evidence lands. It has no unresolved design dependency and is ready
for implementation.

## Assignment Relation And Identity

Write `A <: B` when an actual value of complete type `A` may satisfy expected
type `B`. Write `A = B` for resolved structural type equality, including nominal
owner identity, type arguments, variant sets, and function effect sets, while
ignoring alias spelling and field or effect ordering. Equality is stronger
than mutual assignability: irrelevant generic arguments remain part of type
identity even when they do not restrict assignment.

The relation applies uniformly at direct and nested boundaries. It includes
same-ADT variant-set inclusion, immutable record width and depth compatibility,
and function compatibility. It adds no unrelated nominal subtyping, numeric
assignment conversion, predicate-derived subtype, or runtime coercion.
`unknown` remains an inference/recovery placeholder, not a top or bottom type
and not evidence that a variance constraint is satisfied. Existing completeness
checks must resolve required slots before admitting a declaration for execution.

| Actual and expected types | Assignment condition |
| --- | --- |
| Primitive or nongeneric nominal types | Resolved identity is equal. `Int` does not satisfy `Float`; `Path` does not satisfy `String`. |
| Instances of different nominal declarations | Reject, including identically spelled declarations in different modules. |
| Instances of one generic declaration | Compare each argument by that declaration's selected parameter variance. |
| Same-ADT refinements, possibly with different arguments | Compare arguments by variance and require the actual variant set to be a subset of the expected set. Treat the base ADT as its complete variant set. |
| Records | Every expected field exists in the actual record and its actual type satisfies its expected type. Extra actual fields are permitted. |
| Functions | Apply every shape, input, result, and effect condition in the callable table below. |
| Alias-qualified types | Resolve identity before comparison; preserve presentation separately. |

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
| `Option<T>` | `T: +` | A present payload produces a `T`. |
| `Result<T, E>` | `T: +`, `E: +` | Both payload alternatives produce their parameter. |
| `List<T>` | `T: +` | Immutable heads and recursive tails produce elements. |
| `Vec<T>` | `T: +` | Immutable elements are produced; updates return another vector. |
| `Dict<K, V>` | `K: +`, `V: +` | Stored entries produce keys and values; updates return another dictionary. |
| Source ADT, including source-backed standard types | Infer each parameter from every payload in the declaration. | Visibility and call sites do not change the classification. |
| Opaque or imported generic type with no checked variance metadata | Every parameter `=`. | Representation safety cannot be inferred from a name or helper signature. |
| Nongeneric resource or other nongeneric type | No parameter variance. | Its nominal identity still controls assignment. |

Dictionary key covariance is deliberate. The inspected JVM implementation of
`dictGet` uses the stored map's ordinary key equality; `dictInsert` and
`dictRemove` copy the map and freeze the result. These operations receive no
key type annotation or stored comparison callback. A widened dictionary may
therefore be queried with an additional key type without invoking a callback
that accepts only the original narrower type. Inserting a wider key produces
a widened result and cannot change the original dictionary. This reasoning
would not establish covariance for a different container that stores a
key-consuming comparator or exposes mutation through an alias. The acceptance
model must confirm preservation through the supported runtime boundaries.

## Source-ADT Inference

A payload starts positive. Record fields and function results preserve the
position; fixed and variadic function inputs reverse it. A nested named type
composes the enclosing position with its corresponding parameter's variance.
An alias neither declares nor overrides variance.

The complete composition table follows. The row is the enclosing position;
the column is the nested parameter classification.

| Enclosing / nested | `+` | `-` | `=` | `*` |
| --- | --- | --- | --- | --- |
| `+` | `+` | `-` | `=` | `*` |
| `-` | `-` | `+` | `=` | `*` |
| `=` | `=` | `=` | `=` | `=` |
| `*` | `*` | `*` | `*` | `*` |

Invariance requires exact argument identity. Consequently an occurrence below
an invariant argument remains invariant even if a further nested constructor
would otherwise ignore it. An irrelevant enclosing argument ignores its
entire argument for assignment.

| Occurrences across all payloads | Inferred result |
| --- | --- |
| Positive only, with any number of irrelevant occurrences | `+`. |
| Negative only, with any number of irrelevant occurrences | `-`. |
| Positive and negative, or any invariant occurrence | `=`. |
| No effective occurrence, including occurrences only through irrelevant parameters | `*`. |

Recursive and mutually recursive declarations receive the least restrictive
classification satisfying all their declaration constraints together. The
answer must not depend on declaration order or a recursive edge being visited
first. For example, a recursive list with a direct `T` head is covariant. A
recursive chain that carries `fn(T) -> ()` is contravariant. A chain carrying
both a `T` and `fn(T) -> ()` is invariant. A cycle in which `T` is only passed
around recursive type arguments and never observed in any payload is
irrelevant. This specifies the result, not a required solver architecture.

The definition-based inference and recursive constraint treatment follow the
[Rust Compiler Development Guide's variance discussion](https://rustc-dev-guide.rust-lang.org/variance.html).
That source infers data-type variance from definitions and referenced
definitions, uses a four-way variance lattice, and resolves recursive
constraints. Veln adopts the least restrictive sound result and excludes
call-site-dependent variance. Rust's trait, lifetime, and compiler-query details
are not part of this proposal.

## Callable Composition

Let `N <: W` with `N != W`. For functions, compare their resolved effect sets
at each function node after instantiating call-bound effect rows.

| Aspect or example | Selected result |
| --- | --- |
| Fixed parameters | Counts must match. For each position, the expected parameter type must satisfy the actual parameter type. |
| Variadic marker | Present on both sides or absent on both sides. A fixed function and a variadic function are incompatible in either direction. |
| Two variadic functions | Fixed counts must match; compare fixed inputs contravariantly and the variadic element contravariantly. |
| Result | Actual result must satisfy expected result. |
| Effects | Actual effects must be a subset of expected effects. A pure callable can satisfy an effectful expectation. |
| `fn(W) -> N` to `fn(N) -> W` | Accept when effects satisfy the subset condition. |
| `fn(N) -> W` to `fn(W) -> N` | Reject: input and result directions both fail. |
| `fn(...W) -> N` to `fn(...N) -> W` | Accept when effects satisfy the subset condition. |
| `fn(W, ...W) -> N` to `fn(...N) -> W` | Reject: fixed counts differ. |
| `fn(fn(N) -> ()) -> ()` to `fn(fn(W) -> ()) -> ()` | Accept for equal effects at corresponding nodes; two input reversals restore the original direction. |
| A function accepting `{id: Int}` to one accepting `{id: Int, label: String}` | Accept with compatible results and effects: the actual callable requires fewer fields. Reverse assignment fails. |
| A pure function to the otherwise identical `effects [stdio]` function type | Accept. Reverse assignment fails. |
| An actual function accepting a `stdio` callback to one accepting a pure callback | Accept if outer results and effects match. Reverse assignment fails because a callback supplied by the caller could perform `stdio`. |
| Any failed shape, component, or effect condition | Reject the whole assignment; success in another component cannot compensate. |

A function nested in a parameter is compared in the reversed direction as a
whole, including its own effect subset. The enclosing function's effect set
continues to describe only that function's execution; nesting does not merge
all callback effects into it automatically.

Call-bound effect row variables use the existing binding scopes. After polarity
composition, collect set-inclusion constraints for all occurrences together.
Infer the least concrete sets satisfying their lower bounds, and check every
upper bound before accepting. An unconstrained call-bound row receives the
empty set. For example, constraints `stdio ⊆ E` and `E ⊆ {stdio, network}`
infer `E = {stdio}`; constraints `stdio ⊆ E` and `E ⊆ {network}` reject.
Concrete effects already written in a row remain present. Multiple occurrences
of one row share one solution, independent of traversal order. Rows bound by
an enclosing generic declaration are rigid during its body check: an inclusion
must hold for every permitted instantiation, not just an empty substitution.
This adds neither a new row binder nor general first-class row polymorphism.

The [Rustonomicon's subtyping and variance chapter](https://doc.rust-lang.org/nomicon/subtyping.html)
explains the function input/output directions and distinguishes owning
collection covariance from mutation through an alias. Veln adopts these
polarity directions, with immutable source values replacing Rust's ownership
conditions. Rust's lifetime relation supplies comparative reasoning, not the
ADT-refinement relation or effect rules specified here. Treating every payload
as covariant is rejected because a stored callback can consume its parameter.
Treating every constructor as invariant is safe but needlessly rejects the
immutable producer cases above.

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
expression must not change previously admitted type facts, effect-row
substitutions, or alias provenance. Successful inference over the same
contributions has the same resolved type and presentation in every order.
Diagnostic recovery may report errors at their original source locations; it
must not use a failed contribution to justify another accepted assignment.

## Metadata, Presentation, And Failure

| Surface | Selected behavior |
| --- | --- |
| Dependency type metadata | Source-package analysis computes one classification per declared parameter, keyed by resolved declaration identity and parameter order. Importers use these checked semantic descriptors, including for hidden payloads. No metadata-only package input or serialized interface format is introduced. |
| Missing or invalid variance metadata | Use invariance for unavailable classifications; malformed dependency metadata still follows the package loader's ordinary rejection policy. Never infer covariance from a displayed signature. |
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

The following evidence is planned, not implemented or passing. The tables above
are the independent acceptance oracle; current compiler behavior cannot supply
the expected results for the change.

| Contract | Planned evidence and required observation |
| --- | --- |
| Constructor and polarity tables | Table-driven semantic tests cover each composition cell, both assignment directions, same-spelled distinct ADTs, and refinement sets with varying arguments. |
| Recursive inference | Source-ADT fixtures cover positive, negative, mixed, irrelevant, opaque, and mutually recursive cases; declaration permutations must yield the same classifications. |
| Callables and effects | Accepted and rejected source fixtures cover every shape row, nested callbacks, record width, effect subsets, shared row lower/upper bounds, and unsatisfiable rows. |
| Bounded inference | Permuted branch, collection, and payload cases cover successful joins, annotation-required meets, irrelevant slots, exact constraints, and transactional exclusion of failed contributions. |
| Immutable dictionary safety | Companion tests in `crates/veln-stdlib/veln/` widen refined keys and values, query existing and absent keys, insert/remove on the widened value, and prove that the original keys, values, and iteration order remain unchanged. |
| Host and backend behavior | Backend cases independently check representation-preserving widened callback calls and container transport; they do not substitute for stdlib API tests. |
| Metadata and identity | Package fixtures compare local and imported behavior, hidden payloads, absent metadata, and invalidation after a variance-changing payload edit. |
| Presentation and diagnostics | Human and JSON fixtures cover nested paths, reversed input comparison, direct exclusions, aliases, modifier-free declarations, and stable package/LSP/MCP presentations. |
| Failure preservation | Analysis and language-service regression cases prove no partial signature or row substitution escapes rejection, retained successful snapshots survive applicable failures, and rejected programs do not run. |

Compiler evidence belongs in the existing semantic test suites and
`examples/specification/check/` with diagnostic companions. Standard-library
behavior belongs in companion tests under the
[test placement policy](../reference/toolchain-test-harness.md). Run the
existing executable specification harness and focused package, LSP, MCP, and
backend suites for their respective contracts. Recursive generated-input
coverage must follow the repository's bounded-run and performance-audit
skills; this proposal sets no unmeasured timing target.

Once those checks pass, update the smallest assignment-compatibility and ADT
refinement sections in the current type specification, together with executable
nested-refinement examples. Then retire this proposal and its catalog entry.

## Scope And Implementation Basis

This proposal is independently ready. It does not depend on the remaining
refinement-flow, repair, or handler proposals. Current direct-boundary
refinement behavior remains authoritative until this change is implemented.
It excludes mutable references, declaration-site variance syntax, general
unions/intersections, higher-kinded types, predicate-based subtyping, and
runtime representation changes solely for variance.

The inspected local implementation anchors are
`crates/veln-sema/src/type_relations.rs` for current record width, same-direction
function component checks, and nested refinement restrictions;
`crates/veln-sema/src/effect_rows.rs` for existing row binding/substitution;
`crates/veln-sema/src/aggregate_type_join.rs` for current refinement joins; and
`crates/veln-backend-jvm/src/runtime/collections.java.inc` for persistent
map operations. These identify the affected implementation boundaries; the
proposal tables define the intended replacement behavior.
