---
role: proposal
update-when: ADT variant-refinement typing, diagnostics, schema or runtime behavior, language-service support, or planned verification changes.
---

# ADT Variant Refinement Types For State Transitions

## Summary

Complete the remaining semantic and tooling support for implemented ADT variant
refinement types. The current [type specification](../specification/types.md)
owns singleton and finite-set resolution, constructor singleton inference,
direct assignability and widening, refined calls and results, stable mismatch
diagnostics, and runtime erasure.

This proposal retains only the unfinished work: alias presentation and
visibility, aggregate retention and joins, postfix result propagation,
pattern-based control-flow refinement, schema boundaries, package
documentation, command-wide enforcement, LSP, MCP, and language-reference
publication.

## Outcomes And Boundaries

The remaining proposal has three intended outcomes:

- Pattern matching can convert an ordinary ADT value into the required
  variant refinement without a cast or runtime assertion.
- Compiler, package-documentation, LSP, and MCP views agree on the
  spelling and identity of a refined variant.
- `run`, `test`, and `doc` enforce refinements at their existing analysis and
  recovery boundaries in human and machine-readable modes.

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

## Alias Presentation And Visibility

The remaining alias work makes a type alias whose target resolves to an ADT
qualify a variant refinement while retaining the target constructor identity.
Aliases will be transparent for refinement identity and assignability. If
`AliasOne` and `AliasTwo` both
resolve to `Target`, then `AliasOne::V`, `AliasTwo::V`, and `Target::V` have
the same refinement identity and are mutually assignable.

Alias spelling is presentation provenance, not part of the type identity. The
following table defines which spelling an observable surface uses:

| Surface | Refinement spelling |
| --- | --- |
| Type propagated from one explicit annotation | Preserve that annotation's spelling while the propagated type retains one unambiguous preferred spelling. |
| Type inferred without an explicit annotation | Use the existing canonical display name of the resolved target ADT. |
| Join or other inference with different preferred alias spellings | Discard the conflicting preferences and use the target ADT's canonical display name. |
| Mismatch diagnostic | Format the actual and expected types independently. Preserve an unambiguous spelling propagated from the corresponding explicit annotation; otherwise use the target ADT's canonical display name. |
| Package declaration signature | Preserve a written public annotation. Use the target ADT's canonical display name for an inferred refinement. |

An expected type does not relabel the actual type for a mismatch diagnostic.
Alias presentation provenance does not affect equality, assignability, union
duplicate removal, declaration-order sorting, or runtime representation.

Navigation also separates the written alias from the constructor identity.
Definition on an alias-qualified base segment selects the written alias
declaration. Definition, references, prepare-rename, and rename on the final
variant segment select the constructor owned by the resolved target ADT. A
constructor reference set therefore combines target-qualified,
alias-qualified, expression, and pattern occurrences without treating the
refinement as a synthetic declaration.

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
function-value invariance, constructor singleton inference, call and result
checking, and runtime erasure. The sections below define only the remaining
semantic extensions.

### Aggregate Retention, Contextual Widening, And Joins

An aggregate without an expected aggregate type retains every refinement that
its component expressions contribute. Record fields retain their initializer
types. Collection element, dictionary key and value, and generic ADT payload
inference retain refinements in their corresponding inferred type arguments.
An outer constructor expression also retains its own singleton refinement.

When several expressions contribute to one inferred aggregate position,
refinements of the same instantiated ADT use the symmetric least upper bound.
The result is the union of their variant sets, independent of source order. A
complete union becomes the base ADT. A refinement combined with its base ADT
also becomes the base ADT. Contributions that are not refinements of the same
instantiated ADT continue to use the existing aggregate inference and mismatch
rules.

For example, these unannotated values retain nested refinements:

```veln
let record = { state: Connected(1) }
let states = [Connected(1), Closed("normal")]
let boxed = Boxed(Connected(1))
```

The record field has type `Connection::Connected`. The vector has type
`Vec<Connection::Connected | Connection::Closed>`. If `Boxed` is the sole
variant of `Box<A>`, the third expression has type
`Box<Connection::Connected>::Boxed`.

An explicit aggregate annotation supplies expected component types before the
aggregate type is formed. Each component can widen at that direct boundary:

```veln
let states: Vec<Connection> = [Connected(1), Closed("normal")]
```

This contextual construction produces `Vec<Connection>` directly. It is not
an assignment from `Vec<Connection::Connected | Connection::Closed>`. After
an unannotated aggregate has been constructed, assigning its nested refinement
to a wider aggregate remains invalid until
[Generic And Function Variance](generic-and-function-variance.md) defines and
implements a covariant path for that aggregate.

Field access observes the retained record-field type. Constructor payload
patterns observe the retained payload type after generic substitution. A
collection operation whose declared result or callback parameter uses the
element type observes the collection's retained type argument. None of these
projections widens a refinement merely because the surrounding aggregate is
unannotated.

`if` and `match` use a symmetric least upper bound that does not depend on
branch order. The join of refinements with the same ADT identity and generic
arguments is the union of their variant sets. A join of `A::V` and `A::W` is
therefore `A::V | A::W`. A join of a refinement and its base ADT is the base
ADT. A union that gains every declared variant also becomes the base ADT.
These joins do not require an enclosing expected type. Other joins use the
existing type compatibility rules.

This least-upper-bound rule operates on already resolved branch types. It does
not by itself resolve an ambiguous constructor or infer missing generic
arguments. Only the current
[constructor-context rules](../specification/types.md#inference-rules) allow an
expected type at the constructor expression to supply that context.

### Refined Result Propagation

Postfix `?` preserves the existing `Result` propagation contract when its
operand has a singleton refinement. The refinement determines which runtime
branch the operator takes. It does not relax the operator's typing rules.

| Operand static type | Observable result |
| --- | --- |
| `Result<T, E>::Ok` | Apply the ordinary `?` context and error-compatibility checks, produce `T`, and continue evaluation. |
| `Result<T, E>::Err` | Apply the same ordinary checks and return the error through the existing propagation path. Normal evaluation after the operator does not occur. |
| `Result<T, E>::Ok \| Result<T, E>::Err` or `Result<T, E>` | Use the existing runtime branch and propagation behavior. |

A statically known `Ok` operand must still occur in a context where ordinary
postfix `?` is permitted. Its error type must satisfy the same compatibility
rules as an unrefined `Result<T, E>`. The checker does not discard those
requirements merely because the error branch is impossible. The produced type
is exactly `T`; if `T` is itself a refinement, the operator preserves it.

A statically known `Err` operand is a guaranteed early return. This fact does
not produce an error or warning as part of variant refinement. The operator's
success type remains `T` for checking its surrounding expression, and source
after the guaranteed return still receives ordinary name, type, effect, and
declared-result checking. Existing unreachable-code diagnostics, if any, remain
independent of refinement propagation.

### Pattern Refinement

A stable value is an immutable binding or parameter, or a record-field path
rooted at a stable value. Parentheses do not change the stable value. A
qualified name is stable only when name resolution identifies an immutable
value binding. Calls, indexing, operators, and other computed expressions are
not stable values, even when the same source text occurs more than once.

A local initialized directly from a stable value is a transparent alias of
that value. A field path through a transparent alias denotes the same stable
value as the corresponding path through the alias source. A pattern binding
for the complete matched value is also a transparent alias. These rules are
transitive. Separate construction, equality, a contract predicate, or a
user-defined Boolean helper does not establish a transparent alias.

When a `match` scrutinee is a stable value, a constructor arm refines that
value and every transparent alias to the matched singleton variant for the arm
expression. Payload bindings retain their existing payload types. The
refinement ends with the arm and does not change any binding's declared or
inferred type outside the arm.

```veln
pub fn advance(state: Connection) -> Connection
	match state
		Disconnected => connect(state)
		Connected(_) => close(state)
		Closed(_) => state
	end
end
```

A `match` whose scrutinee is already `A::V` has the one-case finite domain
`V`. A scrutinee of type `A::V | A::W` has the two-case finite domain `V` and
`W`. The checker accepts arms that cover the complete finite domain, or a
catch-all, as exhaustive. An arm for a variant outside the scrutinee's set is
rejected as impossible.

For each arm, the remaining variant set is the scrutinee's finite domain minus
the variants selected by preceding valid constructor arms. A constructor arm
uses its singleton intersection with that remaining set. A catch-all arm uses
the complete remaining set. A binding catch-all becomes a transparent alias
with that refinement. A catch-all that binds no name still refines every
existing transparent alias for its arm expression.

Arm classification follows this precedence:

1. Validate constructor casing, resolution, visibility, owning ADT, generic
   arguments, and payload shape.
2. If a valid constructor belongs to the scrutinee ADT but its variant is not
   in the scrutinee's original finite domain, report
   `type.match_impossible_variant`.
3. If a valid constructor or catch-all intersects the original domain but no
   variants remain for it after preceding valid arms, report
   `type.match_redundant_arm`.
4. Otherwise, type the arm with its intersection and remove that intersection
   from the remaining set.

An invalid-cased, unresolved, hidden, private, wrong-ADT, wrong-generic, or
malformed constructor arm reports its intrinsic name, visibility, type, arity,
or pattern diagnostic. It does not also report an impossible or redundant-arm
diagnostic, and it does not remove a variant from the remaining set. An
unambiguous recovery identity can still type its payload bindings and body to
avoid derivative unknown-type errors, but recovery does not make the arm valid
or contribute exhaustiveness coverage.

An impossible or redundant arm still receives binding and body checking. A
valid constructor pattern gives its payload bindings the constructor's
substituted payload types. A redundant constructor arm refines stable aliases
to that constructor when it belongs to the original scrutinee domain. An
impossible constructor cannot refine the scrutinee or its aliases because its
intersection with the original domain is empty. A redundant binding catch-all
with no remaining variants uses the original scrutinee type for recovery
because the language has no empty variant union. Inherited expected-type checks
and independent body diagnostics apply even though the arm cannot execute. The
arm inherits the declared result as its expected type when the complete `match`
is the final expression of a function. Veln does not add an explicit `return`
statement to an arm.

The first valid catch-all consumes the complete remaining set. A later valid
constructor or catch-all is redundant. A repeated valid constructor is
redundant after its first covering arm. Impossible and redundant arms do not
change the remaining set.

The redundant-arm reason is deterministic. Use `preceding_catch_all` when a
valid catch-all precedes the arm. Without a preceding catch-all, use
`duplicate_variant` for a constructor whose variant was already covered. Use
`complete_prior_coverage` for a catch-all reached after constructor arms have
covered the complete original domain. Related context selects the preceding
catch-all, the first arm that covered the duplicate variant, or the arms that
completed the domain, respectively.

The residual refinement and transparent aliases make a catch-all usable as a
state transition without repeating every remaining constructor:

```veln
pub fn recover(state: Connection) -> Connection::Disconnected
	let current = state
	match current
		Disconnected => current
		remaining => reset(state)
	end
end
```

In the second arm, `remaining`, `current`, and `state` all have type
`Connection::Connected | Connection::Closed`. Matching a record-field path
likewise refines repeated uses of that path and paths reached through its
transparent aliases.

Matching a computed expression does not refine a later reevaluation of that
expression. Code must first bind the result when an arm needs the complete
refined value. No narrowing follows from equality, a contract predicate, a
user-defined Boolean helper, or payload contents. Refinement state cannot be
mutated because ordinary Veln bindings and their field paths are immutable.

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
| `type.match_impossible_variant` | A match arm names a variant excluded by the refined scrutinee. | `scrutinee_type`, `arm_variant` | The refinement source and selected ADT declaration. |
| `type.match_redundant_arm` | A valid arm has no variant left after preceding valid coverage. | `scrutinee_type`, `arm_pattern`, nullable `arm_variant`, `reason` as `duplicate_variant`, `preceding_catch_all`, or `complete_prior_coverage` | The preceding arm or arms that consumed the applicable variants. |
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

Published diagnostics are ordered by primary source span. Diagnostics with the
same primary span use the check order above, followed by union-base and
assignability diagnostics. A uniquely recovered casing identity lets later
independent checks run, but recovery never makes the annotation valid.

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

## Command Behavior

`run` and `test` will use the shared semantic analysis and will not execute a
selected program whose analysis contains a refinement error. `doc` will use the
same analysis and preserve written public refinements in canonical declaration
signatures. Their machine-readable modes will use the same diagnostic codes,
details, spans, and related notes as their human modes.

These commands will not treat a recovered or unresolved refinement as a
successful static transition.

## Language-Service Contract

The semantic model, shared language service, LSP adapter, MCP adapter, and
package-documentation catalog must carry the resolved base-type and constructor
identities without reconstructing them from display text.

### LSP

The existing LSP surface gains the following behavior:

- Semantic tokens classify each base path as `type`, each final variant as
  `enumMember`, and `|` as the existing operator token class. Declaration and
  existing modifiers follow the underlying type and constructor identities.
- Published diagnostics project the diagnostic table with the existing UTF-16
  range conversion and related information.
- Definition on the base type goes to the type or selected type alias.
  Definition on the final segment goes to the constructor declaration.
- References selected at a constructor declaration, constructor expression,
  constructor pattern, or refinement segment share one constructor identity
  and include refinement occurrences.
- Prepare-rename and rename on the final segment use the existing constructor
  casing and conflict rules. Rename edits constructor expressions, patterns,
  and refinement occurrences atomically. Rename on an alias-qualified base
  retains the alias identity and does not rename the target type.

Invalid or recovered refinements contribute only identities that the shared
recovery rules can establish unambiguously. An invalid request or analysis
failure does not replace the retained document snapshot or change the result
of a later valid request.

This proposal does not add hover, completion, signature-help, or inlay-hint
capabilities. Those protocol surfaces require independent whole-language
contracts rather than variant-only implementations.

### MCP

The existing MCP surface gains matching saved-snapshot behavior:

- `check_project` returns the new structured diagnostics and counts them in
  the existing summary.
- `definition`, `references`, and `rename` select the same base-type, alias,
  and constructor identities as LSP. Locations use the existing one-based
  Unicode-scalar coordinates, ordering, pagination, and cursor rules.
- A constructor reference page includes refinement occurrences. A rename
  result includes singleton and union occurrences and preserves the existing
  workspace-only edit boundary.
- Package-documentation declaration signatures preserve public singleton and
  union refinement annotations. Constructor documentation identity remains
  the owning ADT and constructor identity rather than a synthetic declaration.
- The language-reference catalog expands its parser-only refinement material
  to cover the current semantic contract, either in the existing types topic
  or in a focused topic if the catalog's subject boundaries require one. The
  existing `search_docs` and `read_doc` tools and language-reference resources
  expose that semantic material after the feature becomes current behavior.

Protocol-invalid input, failed stable capture, failed analysis, pagination
failure, and rename refusal create no partial result, consume no unrelated
cursor, admit no dependency resource, and preserve the previous saved state.
LSP and MCP must obtain navigation from the same transport-independent result;
coordinate and JSON adapters must not implement separate refinement lookup.

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
| Aliases and visibility | Alias-qualified refinements preserve target identity while following the stated presentation and navigation rules. Imported, private, opaque, ambiguous, and exact-companion exposure paths follow the visibility contract. Public-signature checking traverses record fields, generic arguments, function positions, public source ADT payloads, refinement unions, and alias chains without leaking a private base or variant or looping on recursion. Direct leaks select the private written segment; alias-hidden leaks select the outermost written alias and report the structural exposure path. Aliases of one target are mutually assignable, written annotations retain their spelling, unannotated and conflicting-provenance inference uses the canonical target spelling, mismatch sides select their spelling independently, and base and variant navigation select the alias and target constructor respectively. Failed visibility retains unambiguous source navigation identities under existing recovery rules but publishes no declaration or package signature. | Table-driven semantic, display, package-signature, and shared navigation cases covering every structural position, direct and multi-alias leaks, multiple paths, recursive cycles, exact companions, deterministic diagnostic order, exact primary and related spans, retained source identities, absent public identities, and rendered types. |
| Aggregate retention, contextual widening, and joins | Unannotated aggregate positions retain exact refinements; multiple contributions to one aggregate position use a source-order-independent variant union; field access, payload patterns, and collection element positions observe the retained type; and expected component types widen during aggregate construction without a later nested assignment. | Table-driven type-checker cases for aggregate joins, unannotated record, vector, dictionary, and generic ADT retention, explicit aggregate widening, projection, and rejected post-construction nested widening, plus executable `check` examples. |
| Result propagation | Postfix `?` on a known `Ok` produces its exact success type but still requires the ordinary propagation context and error compatibility. Postfix `?` on a known `Err` follows the ordinary error path without a refinement-specific diagnostic, and later source remains checked. | Table-driven type-checker cases for compatible and incompatible known-`Ok` propagation, refined success payloads, known-`Err` early return, and independent failures after that return, plus executable `check` examples. |
| Control-flow refinement | Constructor arms refine stable values and transparent aliases, catch-all arms receive the remaining variant set, union scrutinees restrict the finite match domain, and complete union arms are exhaustive. A valid variant outside the original domain is impossible; a valid constructor or catch-all with no remaining variants is redundant. Invalid arm heads take diagnostic precedence, contribute no coverage, and can use only unambiguous recovery for binding and body checking. Impossible and redundant arms still receive independent body checks and any expected-type check inherited from the enclosing expression, and reevaluated computed expressions gain no refinement. | Match and exhaustiveness cases covering bindings, parentheses, record-field paths, transitive aliases, binding and non-binding catch-alls, duplicate variants, complete prior coverage, invalid casing, hidden and private constructors, wrong-ADT constructors, qualified immutable values, recovered binding and body types, declared-result mismatches in final `match` expressions, and computed-expression boundaries, plus state-machine `check` examples. |
| Schema encode and decode | Refinement annotations preserve the base ADT wire representation. Encode and typed pass-through helpers require statically assignable refined inputs. External decode validates singleton, union, and nested refined positions only after the complete base value decodes successfully. A valid base value with an excluded variant returns `schema.variant_refinement_mismatch` through the existing decode failure channel without publishing a partial result. A decoder that cannot construct or validate the required variant is rejected statically. | Schema eligibility and type-checker cases for refined and base inputs; binary, format-neutral, incremental, singleton, union, nested record, payload, option, result, collection, and dictionary cases; runtime cases for admitted variants, excluded variants, malformed tags, malformed payloads, truncation, deterministic paths, offsets, reasons, and unchanged wire bytes. |
| Diagnostics | Each remaining semantic failure has the exact code, primary span, closed JSON details, related notes, and deterministic overlap ordering. Base-refinement reasons use only the closed values in the diagnostic contract. Remaining resolution, base-eligibility, variant, visibility, and union-base failures compose with current casing, arity, and assignability diagnostics; derivative failures are suppressed; and each new failure retains exactly the specified navigation identities. Impossible and redundant-arm cases use separate codes, while intrinsic casing, resolution, visibility, ADT, generic, arity, and pattern failures suppress derivative arm-classification diagnostics. | Human and JSON command fixtures covering the remaining diagnostic rows, base-reason values, their overlaps with current diagnostics, identity-retention outcomes, and arm-precedence overlaps. |
| Commands | `run` and `test` share semantic analysis and preserve their no-execution boundary on refinement errors; `doc` shares semantic analysis, preserves written public refinements in canonical declaration signatures, and preserves its recovery boundary. Their machine-readable modes use the same diagnostic contract as their human modes. | Command harness cases for `run`, `test`, and `doc` with accepted, rejected, and recovered sources. |
| LSP | Tokens, diagnostics, definition, references, prepare-rename, rename, recovery, UTF-16 conversion, and unchanged-snapshot failures follow the LSP contract. | Editor-neutral cases and stdio LSP request/response fixtures. |
| MCP | Check, navigation, pagination, rename, package signatures, reference publication, and failure-state preservation follow the MCP contract. | Schema validation and multi-request stdio MCP fixtures. |
| Cross-transport identity | LSP and MCP select the same declaration and reference set from the same saved source before coordinate projection. | Shared language-service cases consumed by both adapter suites. |

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

This proposal is complete only when every remaining acceptance row passes, the
language-reference artifact includes the type form, and the smallest current
specification pages for types, execution, diagnostics, editor support, package
documentation, and MCP describe the implemented contract. Completion also
requires the public examples to explain both the state-machine benefit and the
testing boundary.

This page remains the authority for the unimplemented rows, and no stage may
claim end-to-end variant-refinement support until those rows are current and
checked. After all remaining rows are complete, remove this proposal and its
catalog entry.
