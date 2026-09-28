---
role: proposal
update-when: ADT variant refinement syntax, typing, diagnostics, language-service behavior, or planned verification changes.
---

# ADT Variant Refinement Types For State Transitions

## Summary

Add a refinement type for one variant or a finite union of variants of an
algebraic data type (ADT). A function can require or promise a value that is
known to have one of the admitted variants:

```veln
pub type Connection
	pub Disconnected
	pub Connected(socket: Int)
	pub Closed(reason: String)
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

`Connection::Connected` is the type of the complete `Connected` ADT value. It
is not the type of the variant payload. The type
`Connection::Connected | Connection::Closed` admits exactly those two
variants. Each admitted variant is assignable to the union, and the union is
assignable to `Connection`. An arbitrary `Connection` is not assignable to
either refinement.

This feature is deliberately narrower than general dependent types. It makes
finite state transitions visible to the type checker without adding
value-indexed types, theorem proving, or proof terms. Ordinary types, unit
tests, and property-based tests remain responsible for data-dependent behavior
and properties that cannot be established from a finite variant identity.

## Outcomes And Boundaries

The proposal has three intended outcomes:

- A public API can state which ADT state or finite set of states a transition
  accepts and produces.
- Pattern matching can convert an ordinary ADT value into the required
  variant refinement without a cast or runtime assertion.
- Compiler, formatter, package-documentation, LSP, and MCP views agree on the
  spelling and identity of a refined variant.

The feature is useful for protocol phases, compiler passes, security-sensitive
state gates, and other finite state machines. It does not prove that every
state is reachable, that an event eventually occurs, that payload predicates
hold, or that an external system follows the same transition graph.

The introductory API has this statically observable transition model:

| Current static type | Event | Guard | Next static type | Output | Failure |
| --- | --- | --- | --- | --- | --- |
| `Connection::Disconnected` | Call `connect`. | The argument retains the `Disconnected` refinement. | `Connection::Connected` | One ordinary `Connection` ADT value. | None. |
| `Connection::Connected` | Call `close`. | The argument retains the `Connected` refinement. | `Connection::Closed` | One ordinary `Connection` ADT value. | None. |
| `Connection::Connected` or `Connection::Closed` | Call `reset`. | The argument's variant set is a subset of the parameter's union. | `Connection::Disconnected` | One ordinary `Connection` ADT value. | None. |
| `Connection::Disconnected` | Call `reset`. | The argument's variant is excluded from the parameter's union. | No transition. | No call result. | Compile-time `type.variant_mismatch`. |
| `Connection::Connected` or `Connection::Closed` | Call `connect`. | The required `Disconnected` refinement is absent. | No transition. | No call result. | Compile-time `type.variant_mismatch`. |
| `Connection` | Call `connect`, `close`, or `reset` directly. | A match has not established the required refinement. | No transition. | No call result. | Compile-time `type.variant_mismatch`. |

This table describes static call admission. It does not add a runtime state
store, consume a value linearly, or prevent another function from constructing
an otherwise visible variant.

## Source Syntax

A singleton variant refinement appends `::` and a constructor name to a
complete ADT type. A variant union joins two or more singleton refinements with
`|`:

```text
VariantRefinementType ::= VariantAlternative ("|" VariantAlternative)*
VariantAlternative    ::= NamedAdtType "::" UpperName
NamedAdtType           ::= TypePath TypeArguments?
```

Representative forms are:

```veln
Connection::Connected
protocol::Connection::Connected
Result<Int, DecodeError>::Ok
List<String>::Cons
Connection::Connected | Connection::Closed
Result<Int, DecodeError>::Ok | Result<Int, DecodeError>::Err
```

Each prefix must resolve as one named ADT type before the final segment is
resolved as one of its variants. Every alternative in one union must resolve
to the same ADT identity with the same generic arguments. A union of unrelated
ADTs or differently instantiated forms of one generic ADT is invalid. Module
qualification belongs to the prefix. Type arguments occur before the final
`::Variant` segment. This rule keeps
`protocol::Connection::Connected` distinct from a module path that names a
type and gives generic built-in and source ADTs one spelling. It does not add
unions such as `Int | String` or unions between unrelated named types.

Singleton refinements and variant unions are accepted everywhere an ordinary
type annotation is accepted, including:

- function parameters and results;
- result bindings, local annotations, record fields, and ADT payload fields;
- generic type arguments; and
- function type parameters and results.

The formatter preserves the written alternatives and applies the ordinary
formatting rules for type paths and type arguments. It places one space on
each side of `|`. It does not reorder or remove written alternatives, expand
an alias, or remove module qualification.

## Resolution And Visibility

The base prefix uses the existing type namespace, import, alias, generic-arity,
and ambiguity rules. The final segment uses the constructor identity owned by
the resolved ADT. A type alias whose target resolves to an ADT can qualify a
variant refinement. The refinement then has the target constructor identity,
while formatting preserves the written alias. Aliases are transparent for
refinement identity and assignability. If `AliasOne` and `AliasTwo` both
resolve to `Target`, then `AliasOne::V`, `AliasTwo::V`, and `Target::V` have
the same refinement identity and are mutually assignable.

Alias spelling is presentation provenance, not part of the type identity. The
following table defines which spelling an observable surface uses:

| Surface | Refinement spelling |
| --- | --- |
| Formatted source | Preserve the alias or target spelling written in each annotation alternative. |
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

Every alternative of a variant union must independently satisfy these
resolution and visibility rules. Union order and duplicate alternatives do
not affect type identity. The semantic variant set removes duplicates. When
an inferred type needs a stable display order, it uses the owning ADT's
declaration order. A written public signature retains its source spelling.

Built-in finite ADTs participate through their compiler-owned descriptors.
This includes `Option<T>`, `Result<T, E>`, and `List<T>`. Source ADTs and
descriptor-backed standard-library ADTs participate when their variants are
source-visible. An opaque type without a public finite variant descriptor
cannot be refined.

## Static Semantics

For an ADT `A<T...>` and its variant `V`, the type `A<T...>::V` is the
singleton refinement whose variant set is `{V}`. A variant union denotes the
union of its alternatives' variant sets. A refinement is assignable to
another refinement exactly when both have the same ADT identity and generic
arguments and the actual variant set is a subset of the expected variant set.
Every refinement is a subtype of `A<T...>` and retains the same generic
arguments and runtime value. Different singleton variants of the same ADT are
not subtypes of each other.

Refinement widening applies only when the actual and expected types being
compared are themselves the refinement and its base ADT. It does not recurse
through a named type argument, record field, ADT payload, or function parameter
or result. A nested refinement position must therefore be identical on both
sides of assignment. For example, `A::V` is assignable to `A`, but
`List<A::V>` is not assignable to `List<A>`, and neither
`fn(A) -> R` nor `fn(A::V) -> R` is assignable to the other solely because of
that refinement relationship.

This direct-boundary rule keeps generic and function variance outside this
feature. [Generic And Function Variance](generic-and-function-variance.md)
owns any future recursive widening through aggregates or callable positions.

A union that contains every declared variant of the instantiated ADT is
semantically equivalent to the base ADT. An inferred complete union is
displayed as the base ADT. A written complete union remains written as a union
in formatted source and package signatures. There is no empty variant union.

### Construction, Context, And Joins

An expected base ADT, singleton refinement, or variant union selects the
owning ADT for an unqualified constructor expression. Constructor lookup uses
the written constructor name within that ADT. Same-spelled visible
constructors from other ADTs do not make the expression ambiguous. If the
expected type supplies generic arguments, constructor inference uses them,
including when a nullary constructor has no payload from which to infer those
arguments:

```veln
let absent: Option<Int>::None = None
let terminal: Connection::Connected | Connection::Closed = Closed("normal")
```

An expected singleton does not reinterpret a different written constructor.
If `W` is a constructor of `A`, an expression that writes `W` resolves to
`A::W` even when its expected type is `A::V`. Assignability then rejects the
expression with `type.variant_mismatch`. If the expected ADT has no constructor
with the written name, a same-spelled constructor from another ADT is not
selected as a fallback.

These contextual rules apply in every expression position that supplies an
expected type, including annotated bindings, arguments, returns, record
fields, and ADT payloads. Without an expected ADT, constructor resolution uses
the existing visibility and ambiguity rules.

After resolution, a constructor expression has its singleton variant
refinement. An expected base ADT accepts that expression by widening it. A
local binding without an annotation retains the refinement. An explicit
base-ADT annotation widens the initializer:

```veln
let exact = Connected(socket)
let widened: Connection = exact
```

Expected payload types and generic inference continue to flow through the
constructor as they do for ordinary ADTs. A generic variant annotation must
supply enough type arguments for the existing type-annotation rules. It does
not introduce a new inference hole in public signatures.

### Aggregate Retention And Contextual Widening

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
arguments. Only an expected type at the constructor expression supplies the
context described above.

### Calls, Returns, And Function Values

The following assignability table is normative for the planned feature:

| Actual value or callable | Expected type | Result |
| --- | --- | --- |
| `A<T>::V` | `A<T>::V` | Accept. |
| `A<T>::V` | `A<T>::V \| A<T>::W` | Accept because `{V}` is a subset of `{V, W}`. |
| `A<T>::V \| A<T>::W` | `A<T>::V \| A<T>::W \| A<T>::X` | Accept because the actual variant set is a subset of the expected set. |
| `A<T>::V \| A<T>::W` | `A<T>::V` | Reject because the actual value can have variant `W`. |
| `A<T>::V` | `A<T>` | Accept by widening without a runtime conversion. |
| `A<T>::V \| A<T>::W` | `A<T>` | Accept by widening without a runtime conversion. |
| `A<T>` | `A<T>::V` | Reject because the variant is not known. |
| `A<T>` | `A<T>::V \| A<T>::W` | Reject because the variant is not restricted to the expected set. |
| `A<T>::W` | `A<T>::V` where `W` and `V` differ | Reject. |
| `fn(A<T>::V) -> R` | `fn(A<T>::V) -> R` | Accept when the remaining function shape and effects satisfy the existing rules. |
| `fn(A<T>) -> R` | `fn(A<T>::V) -> R` | Reject because refinement widening does not recurse into a function parameter. |
| `fn(P) -> A<T>::V` | `fn(P) -> A<T>` | Reject because refinement widening does not recurse into a function result. |

A call to a refined parameter requires the argument's static variant set to be
a subset of the parameter's set. A refined result checks every return-producing
expression that semantic analysis successfully types against the declared
variant set. Control-flow reachability, including a constant condition or an
arm excluded by a refined match domain, does not waive this check. An excluded
arm remains invalid under the pattern-refinement rules. If an earlier error
prevents a return-producing expression from receiving a type, the checker does
not emit a derivative `type.variant_mismatch` for that expression. Contracts
and runtime assertions do not convert a base ADT into a refinement. Effects
remain orthogonal to variant assignability.

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
return checking. Existing unreachable-code diagnostics, if any, remain
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
because the language has no empty variant union. Return checks and independent
body diagnostics apply even though the arm cannot execute.

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
| `type.variant_refinement_base` | The base prefix does not resolve to one refinable ADT. | `written_type`, `reason` | Ambiguous candidates or the opaque/non-ADT declaration. |
| `type.variant_refinement_unknown` | The final segment is not a visible variant of the resolved ADT. | `base_type`, `variant` | The ADT declaration and visible variant names. |
| `type.variant_refinement_private` | A refinement exposes or selects an inaccessible variant. | `base_type`, `variant`, `boundary` | The private declaration and public-signature boundary when applicable. |
| `type.variant_union_base` | A union alternative resolves to a different ADT identity or generic arguments. | `expected_base_type`, `actual_base_type` | The first alternative that established the required union base. |
| `type.variant_mismatch` | A value's possible variant set is not a subset of the required set. | `actual_type`, `expected_type`, `expected_variants` in ADT declaration order | The refined parameter, return, field, or local annotation. |
| `type.match_impossible_variant` | A match arm names a variant excluded by the refined scrutinee. | `scrutinee_type`, `arm_variant` | The refinement source and selected ADT declaration. |
| `type.match_redundant_arm` | A valid arm has no variant left after preceding valid coverage. | `scrutinee_type`, `arm_pattern`, nullable `arm_variant`, `reason` as `duplicate_variant`, `preceding_catch_all`, or `complete_prior_coverage` | The preceding arm or arms that consumed the applicable variants. |

Parser failures that cannot form a base type, `::`, and final constructor name
remain syntax diagnostics. Once that structure exists, semantic failures use
the table above. Existing generic arity, name ambiguity, duplicate, and
invalid-casing diagnostics remain independently reportable. Diagnostic order
must be deterministic, and failure must not publish a partially typed
declaration.

Machine-readable diagnostics use the existing diagnostic envelope and
half-open spans. New detail objects are closed schemas. Human and JSON cases
must cover every row, including overlap with one independently provable name or
arity failure.

## Runtime And Compatibility

Singleton refinements and variant unions are erased after static checking.
They do not add tags, checks, casts, allocation, or a distinct JVM
representation. A refined value uses the existing ADT representation and
pattern-match behavior.

Existing source remains valid because the proposal adds a type form and a
subtype-to-base widening rule. Existing unannotated constructor bindings can
gain a more precise internal type, but observable acceptance must remain
compatible unless code attempts an operation that the existing base ADT also
rejects. Canonical package signatures and diagnostics may expose the more
precise type only where source or inference retains it under the rules above.

Serialization, schema encode/decode, equality, and exhaustiveness use the
underlying ADT representation. Decode helpers that return a base ADT do not
claim a refined result unless their declared signature does so.

## Command Behavior

Every command that invokes shared semantic analysis observes the same variant
resolution, refinement, assignability, and diagnostics. `check` reports the
result without execution. `run` and `test` do not execute a selected program
whose analysis contains a refinement error. `doc` preserves written public
refinements in canonical declaration signatures. JSON modes use the same
diagnostic codes, details, spans, and related notes as their human modes.

`fmt` formats a structurally complete refinement or variant union without
requiring semantic resolution. Parser recovery must keep surrounding
declarations available, but no command may treat a recovered or unresolved
refinement as a successful static transition.

## Language-Service Contract

The parser, AST wire form, formatter, semantic model, shared language service,
LSP adapter, MCP adapter, and package-documentation catalog must carry the
base-type and constructor identities without reconstructing them from display
text.

### LSP

The existing LSP surface gains the following behavior:

- Semantic tokens classify each base path as `type`, each final variant as
  `enumMember`, and `|` as the existing operator token class. Declaration and
  existing modifiers follow the underlying type and constructor identities.
- Full-document formatting preserves and canonically spaces singleton and
  union refinements, including generic alternatives.
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
- The language-reference catalog receives a variant-refinement topic. The
  existing `search_docs` and `read_doc` tools and language-reference resources
  expose it after the feature becomes current behavior.

Protocol-invalid input, failed stable capture, failed analysis, pagination
failure, and rename refusal create no partial result, consume no unrelated
cursor, admit no dependency resource, and preserve the previous saved state.
LSP and MCP must obtain navigation from the same transport-independent result;
coordinate and JSON adapters must not implement separate refinement lookup.

## Open Questions

The following decisions remain open. Each one changes observable typing,
diagnostics, serialization, or language-service results and must be resolved
before its affected acceptance row can pass:

- **Decode and schema soundness:** If refinements are allowed in record fields
  and ADT payloads, can schema or decode operations produce those refined
  types? Either decoding must validate the selected variant, the schema must
  encode a single-variant shape, or refinement-bearing decode targets must be
  rejected; erasure alone does not establish that decoded data has variant
  `V`.
- **Transitive visibility:** Does the public-signature visibility check reject
  a private type or variant hidden under a record field, function type, source
  ADT payload, collection argument, or alias chain? The required traversal and
  the diagnostic span must be defined for every type position in which a
  refinement is accepted.
- **Diagnostic overlap contract:** The closed values of
  `type.variant_refinement_base.details.reason` and the deterministic order for
  simultaneous base-resolution, arity, casing, visibility, and variant errors
  are not yet specified. The proposal must decide which independent errors are
  emitted, which derivative errors are suppressed, and which identity remains
  available to LSP and MCP after each failure.

## Acceptance Model

The following evidence is required before any part of this proposal is
described as current behavior:

| Concern | Observable acceptance | Planned evidence |
| --- | --- | --- |
| Grammar and formatting | Singleton and union refinement forms parse in every type position, only variants of one instantiated ADT compose a union, malformed separators and final segments fail at the responsible token, and format is idempotent. | Executable source grammar, accepted and rejected fixtures, parser cases, AST wire round trips, and formatter cases. |
| Resolution, aliases, and visibility | Source, built-in, generic, qualified, imported, private, opaque, ambiguous, and exact-companion bases follow the stated identity and visibility rules. Aliases of one target are mutually assignable, written annotations retain their spelling, unannotated and conflicting-provenance inference uses the canonical target spelling, mismatch sides select their spelling independently, and base and variant navigation select the alias and target constructor respectively. | Table-driven semantic, display, package-signature, and shared navigation cases with exact diagnostics, spans, identities, and rendered types. |
| Construction, joins, aggregate retention, and widening | Expected base, singleton, union, and aggregate types select the owning ADT and supply generic arguments for unqualified constructors; nullary generic constructors use that context; a written different variant remains different; same-spelled constructors from other ADTs do not create ambiguity or provide a fallback; constructors and unannotated aggregate positions retain exact refinements; multiple contributions to one aggregate position use a source-order-independent variant union; field access, payload patterns, and collection element positions observe the retained type; and expected component types widen during aggregate construction without a later nested assignment. | Table-driven type-checker cases for each contextual constructor outcome, direct and aggregate join, unannotated record, vector, dictionary, and generic ADT retention, explicit aggregate widening, projection, and rejected post-construction nested widening, plus executable `check` examples and backend execution and representation cases. |
| Calls, returns, function values, and result propagation | Every singleton, union, and base assignability-table row succeeds or fails as specified. Nested refinement differences in named types, records, ADT payloads, and fixed, variadic, or nested function types remain incompatible. Existing function shapes and effects remain compatible only when their refinement-bearing positions are identical. Every successfully typed return-producing expression satisfies the declared result refinement even under a constant condition or other statically dead control flow. An expression that has no type because of an earlier error produces no derivative variant mismatch. Postfix `?` on a known `Ok` produces its exact success type but still requires the ordinary propagation context and error compatibility. Postfix `?` on a known `Err` follows the ordinary error path without a refinement-specific diagnostic, and later source remains checked. | Table-driven type-checker cases, including nested aggregate and callable boundaries, constant-condition cases, prior-error cases, compatible and incompatible known-`Ok` propagation, refined success payloads, known-`Err` early return, and independent failures after that return, plus executable `check` examples. |
| Control-flow refinement | Constructor arms refine stable values and transparent aliases, catch-all arms receive the remaining variant set, union scrutinees restrict the finite match domain, and complete union arms are exhaustive. A valid variant outside the original domain is impossible; a valid constructor or catch-all with no remaining variants is redundant. Invalid arm heads take diagnostic precedence, contribute no coverage, and can use only unambiguous recovery for binding and body checking. Impossible and redundant arms still receive independent body and return checks, and reevaluated computed expressions gain no refinement. | Match and exhaustiveness cases covering bindings, parentheses, record-field paths, transitive aliases, binding and non-binding catch-alls, duplicate variants, complete prior coverage, invalid casing, hidden and private constructors, wrong-ADT constructors, qualified immutable values, recovered binding and body types, return mismatches, and computed-expression boundaries, plus state-machine `check` examples. |
| Diagnostics | Every semantic failure has the exact code, primary span, closed JSON details, related notes, and deterministic overlap ordering. Impossible and redundant-arm cases use separate codes, while intrinsic casing, resolution, visibility, ADT, generic, arity, and pattern failures suppress derivative arm-classification diagnostics. | Human and JSON command fixtures covering every diagnostic row and each arm-precedence overlap. |
| Commands | Check, run, test, doc, format, and their machine-readable modes share analysis and preserve their execution or recovery boundaries. | Command harness cases with accepted, rejected, and recovered sources. |
| Runtime erasure | Singleton-refined, union-refined, and widened values preserve constructor tag, payload, matching, equality, schema, and backend behavior without a refinement check. | JVM execution, encode/decode, and regression cases. |
| LSP | Tokens, formatting, diagnostics, definition, references, prepare-rename, rename, recovery, UTF-16 conversion, and unchanged-snapshot failures follow the LSP contract. | Editor-neutral cases and stdio LSP request/response fixtures. |
| MCP | Check, navigation, pagination, rename, package signatures, reference publication, and failure-state preservation follow the MCP contract. | Schema validation and multi-request stdio MCP fixtures. |
| Cross-transport identity | LSP and MCP select the same declaration and reference set from the same saved source before coordinate projection. | Shared language-service cases consumed by both adapter suites. |

The state-machine examples must include a permitted transition, a rejected
transition from the wrong variant, widening at an API boundary, match-based
recovery of a refined value, a generic state ADT, a private-variant boundary,
and an unchanged saved result after a failed language-service request.

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

This proposal is complete only when every acceptance row passes, the source
grammar and language-reference artifact include the new type form, and the
smallest current specification pages for source syntax, types, execution,
diagnostics, editor support, package documentation, and MCP describe the
implemented contract. Completion also requires the public examples to explain
both the state-machine benefit and the testing boundary.

Do not promote only the parser spelling as the feature. If implementation is
staged, this page remains the authority for all unimplemented rows and no
partial stage may claim end-to-end variant refinement support. After all rows
are current and checked, remove this proposal and its catalog entry.
