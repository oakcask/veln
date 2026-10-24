---
role: proposal
update-when: ADT variant-refinement typing, diagnostics, schema or runtime behavior, or planned verification changes.
---

# ADT Variant Refinement Types For State Transitions

## Summary

Complete the remaining semantic and tooling support for implemented ADT variant
refinement types. The current [type specification](../specification/types.md)
owns singleton and finite-set resolution, constructor singleton inference,
direct assignability and widening, aggregate retention and joins, control-flow
result joins, contextual aggregate construction, refined calls and results,
stable mismatch diagnostics, runtime erasure, and recursive public structural
visibility. The current
[package-documentation specification](../specification/package-documentation.md)
owns publication of accepted written signatures and package-atomic rejection
of visibility-invalid declarations. That current gate resolves imports and
transparent aliases through retained non-exported modules. A diagnostic rooted
only in a retained source remains non-blocking while every exported declaration
is publishable. If dependency closure makes an exported declaration
unavailable, only retained privacy diagnostics in that declaration's causal
dependency closure participate in the package-atomic failure.

This proposal retains only the unfinished work: qualified-value match
refinement, external or representation schema decode validation, the
provider-dependent unavailable-descriptor diagnostic, and semantic identity
recovery for a lowercase final segment. That compiler recovery is distinct
from the implemented protocol-visible
workspace-base navigation recovery for an invalid final segment. Qualified
immutable-value refinement depends
on a separately specified source form for module-addressable immutable data
values; current qualified constructor and function expressions do not satisfy
that dependency.

## Outcomes And Boundaries

The remaining proposal has these intended outcomes:

- Qualified immutable values gain the implemented stable-value match
  refinement only after a separate source-surface and name-resolution contract
  provides module-addressable immutable data values.
- External or representation schema decode preserves the base ADT wire format
  while validating the annotated variant set.
- Production inputs with a known ADT identity but no public finite variant
  descriptor report the defined provider-dependent diagnostic after that
  provider contract exists.
- A lowercase final segment participates in deterministic parser and semantic
  diagnostic overlap. A unique casing recovery retains its constructor
  identity without adding a lookup or unknown-variant failure caused only by
  the casing. A final segment without a unique recovery retains only identities
  selected independently of that segment and suppresses dependent failures.

The feature is useful for protocol phases, compiler passes, security-sensitive
state gates, and other finite state machines. It does not prove that every
state is reachable, that an event eventually occurs, that payload predicates
hold, or that an external system follows the same transition graph.

The implemented call admission model does not add a runtime state store,
consume a value linearly, or prevent another function from constructing an
otherwise visible variant.

## Scope Boundary

The implemented structural and call-typing foundation is outside this
proposal. The current
[source](../specification/source-surface.md#variant-refinement-shaped-type-text),
[formatting](../specification/command-fmt.md#formatting-rules), and
[type](../specification/types.md#compatibility-and-limits) and
[diagnostic](../specification/diagnostics-json.md#diagnostic-families)
specifications own those contracts and their limits. This proposal owns only
the unimplemented semantic and tooling behavior below.

## Static Semantics

The current [type specification](../specification/types.md#compatibility-and-limits)
owns refinement identity, finite-set subset assignment, direct widening,
function-value invariance, constructor singleton inference, aggregate retention
and joins, control-flow result joins, contextual aggregate construction, call
and result checking, and runtime erasure. The sections below define only the
remaining semantic extensions.

The examples below use this illustrative API. Its direct call and result
typing, record-literal retention, and aggregate refinement retention are
current behavior. Bare and redundantly parenthesized binding match refinement,
transparent local aliases with shared feasible domains across widening,
record-field paths rooted at immutable parameters and local bindings, finite
refined domains, residual catch-alls, and impossible and redundant arm
classification are also current behavior. Only the qualified immutable-value
extension described after it remains proposed.

```veln
pub type Connection
	pub Disconnected
	pub Connected(socket: Int)
	pub Closed(reason: String)
end

pub type Box<A>
	pub Boxed(value: A)
end

pub fn connect(state: Connection::Disconnected) -> Connection::Connected
	Connected(1)
end

pub fn close(state: Connection::Connected) -> Connection::Closed
	Closed("normal")
end

pub fn reset(
	state: Connection::Connected | Connection::Closed,
) -> Connection::Disconnected
	Disconnected
end
```

### Remaining Pattern Refinement

The current [type specification](../specification/types.md#inference-rules)
owns constructor and residual catch-all refinement, transparent local aliases,
record-field paths rooted at immutable parameters and local bindings,
complete-value pattern aliases, refined finite match domains, exhaustiveness,
impossible-arm classification, and redundant-arm classification. The
remaining work extends that behavior to qualified immutable values.

For the remaining extension, a qualified name is stable only when name
resolution identifies an immutable data-value binding. Calls, indexing,
operators, constructors, and other computed expressions are not stable values,
even when the same source text occurs more than once.

### Source-Surface Dependency

The current [source grammar](../specification/source-surface.md#executable-grammar) admits
`let` bindings only in a function, test, or `begin` body. Its module items do
not include an immutable data-value declaration. Current
[name resolution](../specification/name-resolution.md#namespaces-and-use-roles)
therefore resolves a qualified expression value as a constructor or function,
not as a module-addressable immutable data value.

Qualified immutable-value refinement remains planned. Before selecting that
slice, a separate
Ready proposal must define its declaration syntax, initialization and
visibility rules, package identity, and qualified name-resolution behavior.
That source-surface work must land with accepted and rejected grammar evidence
before match refinement can use the new value form.

Executable boundary evidence must classify an expression by its resolved
declaration kind. A qualified constructor tests the constructor boundary, and
a qualified function reference or call tests the function or computed-value
boundary. Neither can stand in for a qualified immutable data value.

## Diagnostics

Each primary human diagnostic identifies the failed source fact at its own
span. Resolution provenance, the base ADT, the required variant set, and
repair guidance belong in `related` notes rather than the primary message.

| Code | Primary span and failed fact | Stable structured details | Planned related context |
| --- | --- | --- | --- |
| `type.variant_refinement_base` | The ADT identity is known, but its selected production provider has no public finite variant descriptor. | `written_type`, `reason` | The selected provider and its provenance. |
| `schema.variant_refinement_decode_unsupported` | A decode target contains a refinement that the selected decoder can neither construct directly nor validate from the decoded ADT tag. | `target_type`, `base_type`, `expected_variants`, `decoder` | The decoder or codec declaration whose result cannot establish the refinement. |

Parser failures that cannot form a base type, `::`, and an uppercase final
constructor name remain syntax diagnostics. A source with any such parse
failure does not proceed to semantic analysis. Retaining identities from an
invalid lowercase final for the semantic diagnostic overlap below remains
unimplemented.
Existing generic arity, name ambiguity, duplicate, and invalid-casing
diagnostics remain independently reportable. Diagnostic order must be
deterministic, and failure must not publish a partially typed declaration.

The casing overlap below applies only to carriers that the current
[name-resolution contract](../specification/name-resolution.md#resolution-rules)
classifies for casing analysis. Explicit expression type arguments remain
navigation-only casing carriers and do not emit `name.invalid_case`. The
current diagnostic specification owns the independent casing failure for a
qualified type nested in a refinement base, the enclosing base-eligibility
failure, focused union-base comparison, and their per-alternative order. The
planned diagnostics do not expand that current casing boundary.

### Diagnostic Overlap And Recovery

The current
[diagnostic specification](../specification/diagnostics-json.md#diagnostic-families)
owns the completed provider-independent `not_adt` and `opaque` reasons. The
current specification also owns recovery that skips a provider-independent
base failure while preserving comparison of later successfully instantiated
alternatives through `type.variant_union_base`, including that comparison in a
later generic argument after an earlier sibling argument fails. The remaining
provider-dependent work may extend
`type.variant_refinement_base.details.reason` with exactly this value:

| Reason | Failed fact |
| --- | --- |
| `variant_descriptor_unavailable` | The ADT identity is known, but its selected compiler or package provider supplies no public finite variant descriptor. |

The `variant_descriptor_unavailable` reason has a provider prerequisite. Before
selecting that reason for implementation, a separate Ready proposal must
define a production compiler or package input where the ADT identity is known
independently of its public variant descriptor. That proposal must also define
how the selected provider and its provenance reach semantic analysis. A local
source `type` declaration with no lowered variants does not satisfy this
prerequisite and must not be used as evidence for this reason.

The provider prerequisite must land with command evidence that uses the
production provider path. Unit-only provider injection cannot replace that
evidence because it does not show that a user-selectable compiler or package
input reaches the diagnostic.

Known opacity is current behavior and takes precedence over an unavailable
descriptor. Unresolved,
ambiguous, private, and wrong-kind bases use their existing diagnostics instead
of adding `type.variant_refinement_base`. Exact or uniquely casing-recovered
source-backed and source-less bases with wrong generic arity retain the existing
invalid-annotation diagnostic. A uniquely recovered invalid-cased base keeps
`name.invalid_case` on a carrier where casing analysis applies and remains
eligible for an independently provable base failure.

The checker reports every failure that it can prove from identities available
without assuming that an earlier failed step succeeded. It suppresses a
diagnostic when that diagnostic follows only from a missing or invalid result
of an earlier step. The following table defines the observable overlap and
identity contract. Retained semantic identity supports suppression and later
analysis; it does not by itself publish protocol-visible navigation.

| Condition | Emitted diagnostics | Suppressed diagnostics | Retained identity |
| --- | --- | --- | --- |
| The refinement syntax is incomplete and no planned recovery applies. | The responsible syntax diagnostic. | All semantic refinement diagnostics for that type occurrence. | None from the incomplete refinement. |
| The base is unresolved. | The existing unresolved-name diagnostic. | Base eligibility, variant, visibility, union comparison involving that alternative, and assignability diagnostics. | None. |
| The base is ambiguous. | The existing ambiguity diagnostic with its candidates. | Base eligibility, variant, visibility, union comparison involving that alternative, and assignability diagnostics. | No selected base or variant identity; candidates remain related context. |
| A provider-backed ADT identity has no public finite variant descriptor. | `type.variant_refinement_base` with `reason: "variant_descriptor_unavailable"`. | Variant, visibility, union comparison involving that alternative, and assignability diagnostics. | The resolved base identity only. |
| Under the planned invalid-refinement recovery, a lowercase final segment has one recovery identity. | `parse.variant_refinement_type`, followed by `name.invalid_case` and independent semantic failures discovered through that identity. | A lookup or unknown-variant diagnostic whose only cause is the recovered casing. | The uniquely recovered constructor declaration. |
| Under the planned invalid-refinement recovery, a lowercase final segment has no unique recovery. | `parse.variant_refinement_type`, followed by the independently provable casing or lookup diagnostic. | Downstream failures that require a selected constructor identity. | Only identities selected independently of the failed final segment. |

Within one annotation, the checker visits union alternatives in written order.
It checks each alternative in this order: casing, base resolution, generic
arity, base eligibility, final-segment resolution, and visibility. The current
[diagnostic specification](../specification/diagnostics-json.md#diagnostic-families)
owns comparison of successfully instantiated union bases, including retained
identities, failed-alternative suppression, and focused diagnostic details.

Diagnostics for one annotation follow the written alternative order and check
order above. Diagnostics with the same primary span use that check order,
followed by union-base and assignability diagnostics. The remaining
diagnostics extend the current cross-annotation and cross-family ordering
contract in the [diagnostic specification](../specification/diagnostics-json.md#diagnostic-families).
A uniquely recovered casing identity lets later independent checks run, but
recovery never makes the annotation valid.

Machine-readable diagnostics use the existing diagnostic envelope and
half-open spans. New detail objects are closed schemas. Human and JSON cases
must cover every row, including overlap with one independently provable name or
arity failure. A schema refinement diagnostic selects the refinement annotation
as its primary span. If the refinement is nested, it selects the innermost
refinement that the decoder cannot establish.

## Schema Decode Soundness

A refinement in a schema-visible shape constrains the decoded value.
It does not define a new wire shape, omit an existing constructor tag, or change
the underlying ADT representation.

The current [execution specification](../specification/execution.md#schema-execution)
owns format-neutral encode helpers and typed pass-through decode helpers. Those
helpers retain eligible source-ADT refinements, reject base or excluded values
statically, preserve their result types, and do not change runtime
representation. This section retains only external or representation decode
behavior.

| Boundary | Refinement behavior |
| --- | --- |
| External or representation decode | Decode the complete base value first. After successful base decoding, validate that every refinement-bearing position contains an admitted variant. Return the refined result only after every validation succeeds. |
| Base-typed external decode result | Preserve the declared base type unless the decoder performs the specified validation or directly constructs an admitted variant. Observing a runtime tag alone does not silently strengthen the declared result. |

An external decoder can produce a refinement only when it directly constructs
an admitted variant or can inspect the existing ADT tag after decoding. If it
can do neither, the schema or decode operation is rejected with
`schema.variant_refinement_decode_unsupported`. A handwritten Veln function can
return a refinement through ordinary constructor, match, and result-type
checking; its signature alone does not grant an opaque or host decoder this
capability.

Singleton targets accept only their selected variant. Union targets accept any
variant in their set. Validation is recursive through record fields, ADT
payloads, option and result payloads, collection elements, and dictionary keys
and values wherever the schema vocabulary admits those shapes. Record and ADT
payloads use declaration order, indexed collections use increasing index, and
dictionaries use their existing canonical traversal order. The first failed
position selects the reported field path. A failed validation publishes no
partial decoded value.

The decoder completes ordinary base decoding before refinement validation.
Malformed tags, malformed payloads, truncation, and other existing decode
failures therefore take precedence. If base decoding succeeds but a value has
an excluded variant, the decoder uses its existing failure channel. An
incremental decoder returns `Invalid(DecodeErrorWithReason(...))`; a
result-returning decoder returns `Err`. The error id is
`schema.variant_refinement_mismatch`, the offset identifies the decoded value's
constructor tag or the narrowest available containing position, the field path
identifies the failed refined position, and the reason renders the actual
variant and expected variant set. This is a recoverable decode failure, not a
trap or process failure.

## Acceptance Model

The current [source](../specification/source-surface.md#variant-refinement-shaped-type-text),
[formatting](../specification/command-fmt.md#formatting-rules),
[type](../specification/types.md#compatibility-and-limits), and
[diagnostic](../specification/diagnostics-json.md#diagnostic-families)
specifications own the completed structural and call-typing foundation, which
is outside the remaining acceptance targets. The following evidence is
required before the remaining semantic or tooling support is described as
current behavior:

| Concern | Observable acceptance | Planned evidence |
| --- | --- | --- |
| Qualified immutable values | After a separate Ready proposal adds module-addressable immutable data values, matching a qualified reference to such a declaration has the same stable-value refinement as the corresponding direct binding. A same-shaped qualified constructor or function expression remains outside this rule. | Accepted and rejected source-grammar fixtures and name-resolution cases for the prerequisite declaration, followed by match cases that resolve an actual qualified immutable data-value declaration and distinguish it from constructors and functions. Public state-machine examples must include match-based recovery of a refined value. |
| External schema decode | Refinement annotations preserve the base ADT wire representation. External decode validates singleton, union, and nested refined positions only after the complete base value decodes successfully. A valid base value with an excluded variant returns `schema.variant_refinement_mismatch` through the existing decode failure channel without publishing a partial result. A decoder that cannot construct or validate the required variant reports `schema.variant_refinement_decode_unsupported` at the innermost unsupported refinement. Its closed details contain only `target_type`, `base_type`, `expected_variants`, and `decoder`; related context identifies the selected decoder or codec. | Binary and incremental decoder cases for singleton, union, nested record, payload, option, result, collection, and dictionary positions; runtime cases for excluded variants, malformed tags, malformed payloads, truncation, deterministic paths, offsets, reasons, unchanged wire bytes, and unsupported decoder capabilities; and human and JSON command fixtures for the static diagnostic, closed details, primary span, and related context. |
| Provider descriptor unavailable | After the separate provider prerequisite lands, a production compiler or package input with a known ADT identity and no public finite variant descriptor reports `type.variant_refinement_base` with reason `variant_descriptor_unavailable`. Its closed details contain only `written_type` and `reason`; related context identifies the selected provider and its provenance. A local source declaration with no lowered variants does not establish this reason. | Human and JSON command fixtures that select the production provider and check the diagnostic details and related provenance, plus semantic cases that distinguish compiler or package provenance from local source declarations and unit-only provider injection. |
| Lowercase-final semantic recovery | A lowercase final always keeps `parse.variant_refinement_type` first in the deterministic overlap order. If initial-only casing repair selects one constructor, `name.invalid_case` and independently provable semantic failures follow, the constructor identity is retained, and no lookup or unknown-variant failure is added solely because of the recovered casing. If repair has no unique result, only the independently provable casing or lookup diagnostic follows, downstream failures that require a constructor are suppressed, and only identities selected independently of the final segment are retained. Recovery never makes the annotation valid. | Planned accepted and rejected parser cases, focused semantic cases for both unique and no-unique recovery, and human and JSON command overlap fixtures that check order, suppression, and the retained identity for each case. |

## Non-Goals

- General value-dependent types, type-level computation, proof terms, or a
  theorem prover.
- Payload predicates such as integer ranges, collection lengths, or equality
  between two fields.
- Typestate for mutable external resources or automatic proof of protocol
  liveness, fairness, reachability, or security policy completeness.
- Inferring refinements from contracts, arbitrary Boolean conditions, or
  property-based test results.
- Replacing runtime validation, unit testing, integration testing, fuzzing, or
  property-based testing.
- Adding casts that assert a variant without a constructor expression or
  pattern match.
- General union types between unrelated ADTs or other named types.

## Completion

This proposal is complete only when every remaining acceptance row passes,
including both lowercase-final semantic recovery outcomes, and
the smallest current specification pages for types, execution, diagnostics,
and the package-documentation catalog describe the
implemented contract. Completion also requires the public examples to explain both the
state-machine benefit and the testing boundary.

This page remains the authority for the unimplemented rows, and no stage may
claim end-to-end variant-refinement support until those rows are current and
checked. After all remaining rows are complete, remove this proposal and its
catalog entry.

## Related Work

[Veln PR #1792](https://github.com/oakcask/veln/pull/1792) used local source
declarations with no variants as a proxy for a missing provider descriptor.
That input did not expose a production compiler or package provider whose ADT
identity was available independently of its public finite variant descriptor.
The provider-dependent reason therefore remains unselected until a production
input and provenance contract establish that state.
