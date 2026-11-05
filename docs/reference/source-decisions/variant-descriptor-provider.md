---
role: reference
authority: supporting
update-when: A production compiler or package input supplies ADT identity independently of its public finite variant descriptor.
---

# Variant Descriptor Provider Boundary

Do not add a package or compiler provider merely to make
`variant_descriptor_unavailable` reachable. There is no selected production
input that needs an independently known ADT identity while omitting its finite
variant descriptor. That hypothetical diagnostic is not active proposal work.

The current [diagnostic contract](../../specification/diagnostics-json.md)
distinguishes non-ADT bases from known opaque bases. A local source `type`
declaration without variants establishes the former fact. It does not
establish a missing external descriptor.

## Evidence And Decision

[Veln PR #1792](https://github.com/oakcask/veln/pull/1792) introduced a
provider-eligibility distinction. Its
[command fixture at revision `47307e0`](https://github.com/oakcask/veln/blob/47307e023ca7f2d25c8d9a1adc79342d35a50fb2/examples/specification/check/adt-variant-refinement-base-diagnostics-human/main.veln)
uses an empty local `type Empty` for `missing_descriptor`. This is evidence of
the earlier diagnostic choice, not evidence of a user-selectable external
provider with incomplete metadata.

Current analysis loads source modules and the standard environment through
`crates/veln-analysis/src/analysis.rs`.
The descriptor model in `crates/veln-sema/src/adt/descriptors.rs`
retains finite, opaque, and unavailable states, while refinement base analysis
in `crates/veln-sema/src/types/environment/refinements.rs`
classifies an unavailable source-backed declaration as `not_adt`. An internal
descriptor state or unit-test injection alone does not create a production
package contract.

Adding metadata-only package loading, foreign ABI types, or partial interface
artifacts would introduce separate user-visible features and failure modes.
None is justified by the need to populate a diagnostic reason. Rejecting that
expansion keeps diagnostics tied to facts that selected production inputs can
establish.

## Revisit Boundary

Revisit this decision if an independently justified provider feature supplies
the missing state. Its proposal must define input selection, nominal identity,
generic arity, opacity, descriptor availability, and provenance before choosing
the resulting diagnostic. Command evidence must use that real input path.
An empty source type or a unit-only injected descriptor remains insufficient.
