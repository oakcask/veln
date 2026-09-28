---
role: proposal
update-when: Generic named-type assignability, aggregate covariance, function subtyping, effect-subset composition, or variance verification changes.
---

# Generic And Function Variance

## Summary

Define how assignment compatibility composes through generic named types,
immutable aggregates, and function types. The design must distinguish
covariant, contravariant, and invariant positions instead of assuming that
every nested type relationship has the same direction.

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

## Owned Decisions

This proposal must decide:

- which parameters of compiler-owned types such as `Option`, `Result`, `List`,
  `Vec`, and `Dict` are covariant, contravariant, or invariant;
- whether source ADT variance is inferred from payload use, declared in source,
  or kept invariant;
- how immutable record fields participate in recursive assignability;
- how fixed, variadic, and nested function parameters compose
  contravariantly, results compose covariantly, and both compose with the
  existing effect-subset rule;
- how a parameter used in both positive and negative positions becomes
  invariant;
- how aliases, generic inference, joins, diagnostics, package signatures, LSP,
  and MCP expose a variance result without inventing a second type identity;
  and
- whether variance applies only to ADT refinement widening or uniformly to
  every implemented subtype relationship.

The selected rules must make assignment independent of branch order and
display spelling. A rejected assignment must not publish a partially typed
declaration or replace a retained language-service snapshot.

## Required Decision Tables

The completed design must define observable results for these relationships:

| Type relationship | Required decision |
| --- | --- |
| `Container<Narrow>` to `Container<Wide>` | Classify the constructor parameter and accept only when its variance permits the direction. |
| Source ADT parameter used only in produced payload data | Decide whether that parameter is covariant. |
| Source ADT parameter used as a carried callback parameter | Decide whether that negative occurrence makes the parameter contravariant or invariant. |
| Source ADT parameter used in both produced and consumed positions | Define the invariant result and its diagnostic. |
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
  functions, variadic functions, nested functions, and effects;
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

This proposal remains blocked while its variance classifications and complete
function composition table are undecided. ADT variant refinement can proceed
with direct-boundary widening and does not depend on this proposal. Once this
proposal is complete and implemented, the refinement proposal or current
refinement specification can adopt recursive widening through the positions
that the variance contract proves safe.
