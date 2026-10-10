---
role: proposal
update-when: ADT variant-refinement typing, diagnostics, schema or runtime behavior, language-service support, or planned verification changes.
---

# ADT Variant Refinement Types For State Transitions

## Summary

Complete the remaining semantic and tooling support for implemented ADT variant
refinement types. The current [type specification](../specification/types.md)
owns singleton and finite-set resolution, constructor singleton inference,
direct assignability and widening, aggregate retention and joins, control-flow
result joins, contextual aggregate construction, refined calls and results,
stable mismatch diagnostics, and runtime erasure.

This proposal retains only the unfinished work: alias visibility,
qualified-value match refinement, schema boundaries, package documentation
catalog signatures, remaining LSP diagnostics and recovery, and remaining MCP
diagnostics, package-signature, and saved-state behavior. Qualified
immutable-value refinement depends
on a separately specified source form for module-addressable immutable data
values; current qualified constructor and function expressions do not satisfy
that dependency.

## Outcomes And Boundaries

The remaining proposal has two intended outcomes:

- Qualified immutable values gain the implemented stable-value match
  refinement only after a separate source-surface and name-resolution contract
  provides module-addressable immutable data values.
- Package-documentation catalog, LSP, and MCP views agree on the identity and
  visibility of a refined variant.

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

## Alias Visibility

The current [type specification](../specification/types.md#compatibility-and-limits)
owns alias-qualified refinement identity, assignability, and presentation. The
remaining alias work covers public package signatures and visibility failures.
Current source navigation and rename behavior is specified by
[editor support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and [MCP](../specification/mcp.md#saved-workspace-navigation).

A refinement is valid only when the selected constructor is visible at the
annotation. A public declaration cannot expose a private type or private
variant through a refinement. Exact test-companion access follows the same
boundary as constructor expressions and patterns. A value cannot use a same-
spelled variant from another ADT to satisfy the refinement.

Public-signature visibility is transitive through every structural type
position. The check traverses record fields, named type arguments, function
parameters and results, refinement-union alternatives, and transparent alias
targets. A public source ADT declaration also traverses every payload position
of each public variant. A named public ADT used by another signature relies on
that declaration's own completed visibility check; it is not expanded again at
each use. Traversal is cycle-safe, but a recursive alias or ADT cycle does not
hide an inaccessible refinement reached before the repeated declaration.

Visibility does not depend on variance or runtime erasure. A refinement nested
in a contravariant function parameter, an invariant generic argument, or an
otherwise erased position still exposes its base type and selected variants in
the public type contract. Exact test-companion permission can use a private
constructor in test source, but it cannot make that constructor valid in a
public signature or generated public schema boundary.

When source writes the inaccessible refinement directly, the primary span is
the private base-type segment if the ADT is private, or the final variant
segment if only the variant is private. When the inaccessible refinement is
hidden entirely behind one or more aliases, the primary span is the outermost
alias occurrence written in the public signature. Related notes identify the
alias declarations along the exposure path and the final private type or
variant declaration. Structured `exposure_path` segments distinguish record
fields, generic arguments, function parameters and results, ADT payloads,
union alternatives, and aliases.

Each written public type occurrence reports one
`type.variant_refinement_private` diagnostic for each distinct inaccessible
exposure path. Diagnostics follow source order and structural child order;
cycle detection and repeated traversal of the same path do not duplicate a
diagnostic. A visibility failure prevents publication of the declaration and
its package signature. Resolved alias, type, and constructor identities remain
available to source navigation when the existing recovery rules retain an
unambiguous analysis result. The failure does not create a public declaration
identity, and a failed stable capture or analysis preserves the prior saved
snapshot under the existing language-service rules.

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
| `type.variant_refinement_base` | A uniquely resolved base type cannot provide a refinable finite ADT. | `written_type`, `reason` | The non-ADT or opaque declaration, or the provider that lacks a public variant descriptor. |
| `type.variant_refinement_unknown` | The final segment does not name a variant owned by the resolved ADT. | `base_type`, `variant` | The ADT declaration and visible variant names. |
| `type.variant_refinement_private` | A refinement exposes a private base type or selects an inaccessible variant. | `written_type`, `base_type`, `variant`, `boundary`, `exposure_path` | The alias declarations on the exposure path, the final private declaration, and the public-signature boundary. |
| `type.variant_union_base` | A union alternative resolves to a different ADT identity or generic arguments. | `expected_base_type`, `actual_base_type` | The first alternative that established the required union base. |
| `schema.variant_refinement_decode_unsupported` | A decode target contains a refinement that the selected decoder can neither construct directly nor validate from the decoded ADT tag. | `target_type`, `base_type`, `expected_variants`, `decoder` | The decoder or codec declaration whose result cannot establish the refinement. |

Parser failures that cannot form a base type, `::`, and final constructor name
remain syntax diagnostics. Once that structure exists, semantic failures use
the table above. Existing generic arity, name ambiguity, duplicate, and
invalid-casing diagnostics remain independently reportable. Diagnostic order
must be deterministic, and failure must not publish a partially typed
declaration.

### Diagnostic Overlap And Recovery

`type.variant_refinement_base.details.reason` has exactly these values:

| Reason | Failed fact |
| --- | --- |
| `not_adt` | The uniquely resolved base denotes a named type that has no finite ADT variants. |
| `opaque` | The uniquely resolved declaration intentionally hides its variant identities at the annotation site. |
| `variant_descriptor_unavailable` | The ADT identity is known, but its selected compiler or package provider supplies no public finite variant descriptor. |

The `not_adt` and `opaque` reasons can be selected independently. The
`variant_descriptor_unavailable` reason has a provider prerequisite. Before
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

Known opacity takes precedence over an unavailable descriptor. Unresolved,
ambiguous, private, wrong-kind, invalid-cased, and wrong-arity bases use their
existing diagnostics instead of adding `type.variant_refinement_base`.

The checker reports every failure that it can prove from identities available
without assuming that an earlier failed step succeeded. It suppresses a
diagnostic when that diagnostic follows only from a missing or invalid result
of an earlier step. The following table defines the observable overlap and
identity contract. A retained identity is available to shared source
navigation even though the annotation remains invalid.

| Condition | Emitted diagnostics | Suppressed diagnostics | Retained identity |
| --- | --- | --- | --- |
| The refinement syntax is incomplete. | The responsible syntax diagnostic. | All semantic refinement diagnostics for that type occurrence. | None from the incomplete refinement. |
| The base is unresolved. | The existing unresolved-name diagnostic. | Base eligibility, variant, visibility, union comparison involving that alternative, and assignability diagnostics. | None. |
| The base is ambiguous. | The existing ambiguity diagnostic with its candidates. | Base eligibility, variant, visibility, union comparison involving that alternative, and assignability diagnostics. | No selected base or variant identity; candidates remain related context. |
| The base resolves uniquely but is not refinable. | `type.variant_refinement_base` with one closed reason above. | Variant, visibility, union comparison involving that alternative, and assignability diagnostics. | The resolved base declaration only. |
| Generic arity is invalid after the base ADT resolves. | The existing generic-arity diagnostic, plus any independently provable final-segment casing, unknown-variant, or visibility diagnostic. | Union comparison involving that alternative and assignability diagnostics that require an instantiated type. | The base declaration and, when resolved, the constructor declaration; no instantiated refinement type. |
| The final segment is not a variant of the resolved ADT. | `type.variant_refinement_unknown`, plus an independently applicable casing diagnostic. | Visibility and assignability diagnostics that require a selected constructor. | The base declaration only. |
| A base or final segment has invalid casing and one existing recovery identity. | `name.invalid_case`, plus independent arity, base-eligibility, or visibility failures discovered through that identity. | A lookup or unknown-variant diagnostic whose only cause is the recovered casing. | Every uniquely recovered base and constructor declaration. |
| An invalid-cased segment has no unique recovery and is independently missing, ambiguous, private, or wrong-kind. | `name.invalid_case` when the segment role is known, plus the applicable existing lookup or visibility diagnostic. | Downstream failures that require a selected identity. | Only identities selected independently of the failed segment. |
| The base or selected constructor is inaccessible. | `type.variant_refinement_private` for each exposure path defined above, plus independent casing or arity failures. | Publication and assignability diagnostics that require a valid public type. | Every uniquely resolved alias, base, and constructor declaration for source navigation. |
| Two successfully instantiated alternatives have different base identities or generic arguments. | `type.variant_union_base`. | Assignability diagnostics that require the rejected union type. | All successfully resolved alternative identities; no union type identity. |

Within one annotation, the checker visits union alternatives in written order.
It checks each alternative in this order: casing, base resolution, generic
arity, base eligibility, final-segment resolution, and visibility. The earliest
successfully instantiated alternative establishes the expected union base;
each later successfully instantiated alternative is compared with it. An
incomplete alternative does not prevent an independently complete later pair
from reporting `type.variant_union_base`.

Diagnostics for one annotation follow the written alternative order and check
order above. Diagnostics with the same primary span use that check order,
followed by union-base and assignability diagnostics. This proposal does not
reorder diagnostics from separate annotations or unrelated diagnostic
families. A uniquely recovered casing identity lets later independent checks
run, but recovery never makes the annotation valid.

LSP and MCP use the same retained identities. Definition, references,
prepare-rename, and rename can select a uniquely resolved or uniquely recovered
source identity. An invalid public declaration creates no published package
identity or replacement saved snapshot. A failed stable capture or analysis
preserves the previous saved snapshot under the existing language-service
rules.

Machine-readable diagnostics use the existing diagnostic envelope and
half-open spans. New detail objects are closed schemas. Human and JSON cases
must cover every row, including overlap with one independently provable name or
arity failure. A schema refinement diagnostic selects the refinement annotation
as its primary span. If the refinement is nested, it selects the innermost
refinement that the decoder cannot establish.

## Schema Encode And Decode Soundness

A refinement in a schema-visible shape constrains the decoded or encoded value.
It does not define a new wire shape, omit an existing constructor tag, or change
the underlying ADT representation.

| Boundary | Refinement behavior |
| --- | --- |
| Encode | Require the supplied value to be assignable to the refined schema-visible shape, then use the existing base-ADT encoding. A base ADT or excluded variant fails static checking with `type.variant_mismatch`. |
| Typed pass-through decode | Require the supplied value to be assignable to the refined input shape and preserve that type in the result. No runtime refinement check is needed because the helper does not strengthen a base type. |
| External or representation decode | Decode the complete base value first. After successful base decoding, validate that every refinement-bearing position contains an admitted variant. Return the refined result only after every validation succeeds. |
| Base-typed decode result | Preserve the declared base type even when one execution produces a particular variant. Observing a runtime tag does not silently strengthen the declared result. |

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

## Language-Service Contract

For invalid and recovered refinements, the semantic model and shared language
service must carry every unambiguous base-type or constructor identity without
reconstructing it from display text. Package-documentation signatures must
preserve those identities when the visibility and publication work below makes
them eligible. Navigation and rename for valid refinements are current behavior
specified by [editor support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and [MCP](../specification/mcp.md#saved-workspace-navigation).

### LSP

The existing LSP surface still requires the following behavior:

- Published diagnostics project the diagnostic table with the existing UTF-16
  range conversion and related information.

Invalid or recovered refinements contribute only identities that the shared
recovery rules can establish unambiguously. An invalid request or analysis
failure does not replace the retained document snapshot or change the result
of a later valid request.

This proposal does not add hover, completion, signature-help, or inlay-hint
capabilities. Those protocol surfaces require independent whole-language
contracts rather than variant-only implementations.

### MCP

The remaining MCP surface gains matching saved-snapshot behavior:

- `check_project` returns the new structured diagnostics and counts them in
  the existing summary.
- Package-documentation declaration signatures preserve public singleton and
  union refinement annotations. Constructor documentation identity remains
  the owning ADT and constructor identity rather than a synthetic declaration.

Protocol-invalid input, failed stable capture, failed analysis, pagination
failure, and rename refusal create no partial result, consume no unrelated
cursor, admit no dependency resource, and preserve the previous saved state.

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
| Alias visibility | Imported, private, opaque, ambiguous, and exact-companion exposure paths follow the visibility contract. Public-signature checking traverses record fields, generic arguments, function positions, public source ADT payloads, refinement unions, and alias chains without leaking a private base or variant or looping on recursion. Direct leaks select the private written segment; alias-hidden leaks select the outermost written alias and report the structural exposure path. Public package signatures preserve written annotations. Failed visibility retains unambiguous source navigation identities under existing recovery rules but publishes no declaration or package signature. | Table-driven package-signature cases covering every structural position, direct and multi-alias leaks, multiple paths, recursive cycles, exact companions, deterministic diagnostic order, exact primary and related spans, retained source identities, and absent public identities. |
| Qualified immutable values | After a separate Ready proposal adds module-addressable immutable data values, matching a qualified reference to such a declaration has the same stable-value refinement as the corresponding direct binding. A same-shaped qualified constructor or function expression remains outside this rule. | Accepted and rejected source-grammar fixtures and name-resolution cases for the prerequisite declaration, followed by match cases that resolve an actual qualified immutable data-value declaration and distinguish it from constructors and functions. |
| Schema encode and decode | Refinement annotations preserve the base ADT wire representation. Encode and typed pass-through helpers require statically assignable refined inputs. External decode validates singleton, union, and nested refined positions only after the complete base value decodes successfully. A valid base value with an excluded variant returns `schema.variant_refinement_mismatch` through the existing decode failure channel without publishing a partial result. A decoder that cannot construct or validate the required variant is rejected statically. | Schema eligibility and type-checker cases for refined and base inputs; binary, format-neutral, incremental, singleton, union, nested record, payload, option, result, collection, and dictionary cases; runtime cases for admitted variants, excluded variants, malformed tags, malformed payloads, truncation, deterministic paths, offsets, reasons, and unchanged wire bytes. |
| Diagnostics available without the provider prerequisite | Each selected semantic failure has the exact code, primary span, closed JSON details, related notes, and annotation-scoped overlap ordering. The `not_adt` and `opaque` base reasons compose with current casing, arity, and independently provable failures; derivative failures are suppressed; unrelated diagnostic-family ordering is unchanged; and each new failure retains exactly the specified navigation identities. | Human and JSON command fixtures for both selectable base reasons and the other selected diagnostic rows. Base-reason fixtures include an earlier independent final-segment or arity failure beside a later base failure, and an earlier base failure beside a later recovered-casing occurrence. Semantic cases verify identity retention and unchanged ordering outside the annotation. |
| Provider descriptor unavailable | After the separate provider prerequisite lands, a production compiler or package input with a known ADT identity and no public finite variant descriptor reports `type.variant_refinement_base` with reason `variant_descriptor_unavailable`. A local source declaration with no lowered variants does not establish this reason. | Human and JSON command fixtures that select the production provider, plus semantic cases that distinguish compiler or package provenance from local source declarations and unit-only provider injection. |
| LSP | Diagnostics, invalid-refinement recovery, UTF-16 conversion, and unchanged-snapshot failures follow the remaining LSP contract. | Shared language-service cases and stdio LSP request/response fixtures. |
| MCP | Check, invalid-refinement recovery, package signatures, and failure-state preservation follow the MCP contract. | Schema validation and multi-request stdio MCP fixtures. |

The remaining state-machine examples must include match-based recovery of a
refined value, a private-variant boundary, and an unchanged saved result after
a failed language-service request.

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

This proposal is complete only when every remaining acceptance row passes and
the smallest current specification pages for types, execution, diagnostics,
editor support, the package-documentation catalog, and MCP describe the
implemented contract. Completion also requires the public examples to explain both the
state-machine benefit and the testing boundary.

This page remains the authority for the unimplemented rows, and no stage may
claim end-to-end variant-refinement support until those rows are current and
checked. After all remaining rows are complete, remove this proposal and its
catalog entry.

## Related Work

[Veln PR #1792](https://github.com/oakcask/veln/pull/1792) attempted the full
base-diagnostic slice. Its checked output showed that a global source-span sort
changed unrelated checker order, while removing that sort grouped diagnostics
by checker phase instead of by written union alternative. The same change used
local source declarations with no variants as a proxy for a missing provider
descriptor. The pull request is implementation evidence rather than current
behavior, and it did not expose a production compiler or package provider with
the required state. This proposal therefore limits ordering to one annotation
and leaves the provider-dependent reason unselected until its production input
and provenance contract exist.
