---
role: proposal
update-when: Generic named-type assignability, aggregate covariance, function subtyping, effect-subset composition, or variance verification changes.
---

# Generic And Function Variance

## Summary

Define how assignment compatibility composes through generic named types,
immutable aggregates, and function types. The design must distinguish
covariant, contravariant, and invariant positions instead of assuming that
every nested type relationship has the same direction. The compiler infers
variance from type-parameter occurrences. Veln does not add declaration-site
variance modifiers such as `in`, `out`, `+`, or `-`.

This proposal is separate from
[ADT Variant Refinement Types](adt-variant-refinement-types.md). That proposal
permits refinements in nested annotations but widens a refinement to its base
ADT only at a direct value boundary. This proposal owns any future rule that
allows `Container<A::V>` to widen to `Container<A>` or relates function types
whose parameter or result refinements differ.

## Motivation And Safety Boundary

Veln values and aggregates are immutable from source code, which permits some
covariant relationships without exposing mutation through an alias. Immutability
alone is not sufficient for every generic source ADT. A type parameter can
occur in a function parameter carried by an ADT payload. Treating that source
ADT as covariant can then allow a caller to supply a wider value to a callback
that accepts only a narrower type.

The current type specification describes named types with pairwise-assignable
arguments and function types whose components are recursively assignable. This
proposal must replace that current behavior only after it defines and verifies
a sound variance contract. Until then, refinement widening does not use those
recursive relationships.

## Selected Variance Model

Source ADT parameters use compiler-inferred variance. A type declaration keeps
the existing `type Name<T...>` syntax. The formatter, package declaration
display, LSP, and MCP do not publish an inferred variance modifier as source
text. Dependency type metadata carries the inferred classification needed for
downstream checking.

Inference classifies each parameter from all of its occurrences in the ADT
payload types:

| Occurrences of one parameter | Inferred classification |
| --- | --- |
| Only positive occurrences | Covariant. |
| Only negative occurrences | Contravariant. |
| Both positive and negative occurrences, or any invariant occurrence | Invariant. |
| No occurrence | Irrelevant to the represented value; changing only that argument does not prevent assignment. |
| Occurrence behind a type whose variance is unavailable | Invariant. |

An ADT payload starts in a positive position. An immutable record field and a
function result preserve the enclosing position. A function parameter reverses
it. A nested named type composes the enclosing position with the selected
variance of its corresponding parameter. An invariant nested parameter makes
the containing occurrence invariant. Reversing twice restores the original
position.

The compiler computes this classification from resolved type identity rather
than written alias spelling. An alias neither declares nor overrides variance.
An opaque or dependency type whose definition does not expose a checked
variance contract is invariant at the boundary.

This model keeps variance out of Veln source syntax and makes the safety of a
source ADT follow from the payload types that give the parameter meaning. A
declaration change that moves a parameter between positive and negative
positions can therefore change assignment compatibility. Dependency metadata
and downstream checking must reflect that change without adding a modifier to
the source-level declaration display.

## Owned Decisions

This proposal must decide:

- which parameters of compiler-owned types such as `Option`, `Result`, `List`,
  `Vec`, and `Dict` are covariant, contravariant, or invariant;
- how immutable record fields participate in recursive assignability;
- how fixed, variadic, and nested function parameters compose
  contravariantly, results compose covariantly, and both compose with the
  existing effect-subset rule;
- how aliases, generic inference, joins, diagnostics, package signatures, LSP,
  and MCP expose a variance result without inventing a second type identity;
  and
- whether variance applies only to ADT refinement widening or uniformly to
  every implemented subtype relationship.

The selected rules must make assignment independent of branch order and
display spelling. A rejected assignment must not publish a partially typed
declaration or replace a retained language-service snapshot.

## Decision Table

The completed design must preserve the selected source-ADT results and define
the remaining observable results:

| Type relationship | Selected or required result |
| --- | --- |
| `Container<Narrow>` to `Container<Wide>` | Classify the constructor parameter and accept only when its variance permits the direction. |
| Source ADT parameter used only in produced payload data | Infer covariance. |
| Source ADT parameter used as a carried callback parameter | Infer contravariance unless another occurrence makes the parameter invariant. |
| Source ADT parameter used in both produced and consumed positions | Infer invariance. |
| Source ADT parameter absent from every payload | Treat the parameter as irrelevant to assignment. |
| Source ADT parameter used through an opaque or unclassified type | Infer invariance at that boundary. |
| `fn(Wide) -> Narrow` to `fn(Narrow) -> Wide` | Define parameter contravariance, result covariance, and effect compatibility as one rule. |
| Fixed and variadic function types | Define whether different arity shapes remain incompatible and how the variadic element position composes. |
| Nested functions and aggregates | Apply the selected rules recursively without reversing a position twice incorrectly. |
| Alias-qualified types | Preserve alias presentation while using the target declaration's variance and identity. |

## Acceptance Model

The proposal becomes implementation-ready only after every owned decision has
one unambiguous result in the decision tables. Implementation requires:

- table-driven semantic cases for every accepted and rejected variance
  direction, including parameters used in positive, negative, and mixed
  positions;
- source cases for compiler-owned types, source ADTs, records, aliases, fixed
  functions, variadic functions, nested functions, irrelevant parameters,
  opaque boundaries, and effects;
- source and package-signature cases proving that inferred variance adds no
  declaration-site modifier;
- human and JSON diagnostics that identify the incompatible nested position
  and preserve independently established type identities;
- package-signature, LSP, and MCP cases proving that presentation does not
  change assignability; and
- regression cases proving that rejected assignments do not change saved
  analysis or execute a program.

After those cases pass, update the assignment-compatibility section of the
current type specification. If ADT variant refinements are current by then,
add nested refinement cases to their smallest current specification and
executable evidence as well.

## Non-Goals

- Mutable references, mutable aggregate aliases, or declaration-site mutation
  capabilities.
- General union types, intersection types, higher-kinded types, or type-level
  functions.
- Inferring subtyping from runtime predicates, contracts, or payload values.
- Changing runtime representation solely to support variance.

## Dependency And Readiness

The source-ADT inference model is selected. This proposal remains blocked while
the compiler-owned type classifications and complete function composition
table are undecided. ADT variant refinement can proceed with direct-boundary
widening and does not depend on this proposal. Once this proposal is complete
and implemented, the refinement proposal or current refinement specification
can adopt recursive widening through the positions that the variance contract
proves safe.
