---
role: proposal
update-when: Generic function binder syntax, rigid body checking, call instantiation, function alias polymorphism, type and effect inference, public interface retention, lowering feasibility, or generic function acceptance decisions change.
---

# Generic Functions

Support explicitly declared generic functions whose type arguments are inferred
independently at each use. Preserve that polymorphism through transparent
function aliases, and check function bodies against their declared abstraction.
This proposal is a Ready implementation target. Q1–Q9 have first-delivery
decisions or explicit exclusions and an identified
[implementation approach](#implementation-approach). The semantic, tooling,
artifact, and backend outcomes below remain planned except for the implemented
declaration-owned binder scope and rigid definition checking described by the
current type rules. The current
[source-surface specification](../specification/source-surface.md#function-declaration-binders)
owns declaration parsing and formatting behavior.

## Purpose and current boundary

Generic source ADTs can describe containers and recursive structures, but users
also need reusable functions that transport and transform their payloads.
Writing an `Int` and a `String` version of the same structural operation makes
that abstraction depend on concrete element types.

The current [type specification](../specification/types.md) defines generic
constructor inference, variance, variant refinements, and monomorphic local
inference. Private function inference for functions without ordinary binders
requires concrete types and can use same-module call-site facts. It does not
publish a rigid parameter owned by a caller into a helper's monomorphic
signature. Functions with ordinary binders require complete parameter and
return annotations and do not receive private signature inference. This
inference is not universal quantification.
The current
[source grammar](../specification/source-surface.md#function-declaration-binders)
accepts and retains ordinary function type binders followed by an optional
effect binder. It formats those declaration binders canonically and rejects
unsupported declaration forms. The linked source-surface specification owns
malformed-list parsing and syntax-tree recovery. The current
[diagnostic contract](../specification/diagnostics-json.md#diagnostic-families)
owns the observable diagnostic identifiers, ordering, and primary spans.
The current [type specification](../specification/types.md#usage-and-annotations)
owns declaration-local rigid binder identity, scope through signatures, body
annotations, and executable contracts, unresolved-name isolation from
inaccessible declarations, schema and ADT scope boundaries, qualifier-first
named-type lookup, and owner-scoped visible-type shadow rejection. The current
[diagnostic contract](../specification/diagnostics-json.md#diagnostic-families)
owns the resulting unresolved-name and shadow-classification records.
Explicit binders also take precedence over the embedded standard package's
temporary compatibility names. These outcomes do not yet establish an
independently instantiable type scheme.
The
[effect specification](../specification/effects.md#effect-labels) already
supports one declared effect-row parameter and call-specific substitution.

## Proposed baseline

The baseline supports rank-one parametric polymorphism. A *type scheme* is a
function signature with universally quantified type parameters. A *monotype*
has no universal quantifier, though it can mention rigid parameters belonging
to an enclosing generic declaration. Rank one means that quantifiers belong to
the outer declaration or supported alias binding, not inside ordinary callback
parameters, record fields, or container element types.

The initial scope is ordinary fixed-arity functions with explicit type
parameters and complete parameter and result annotations. It includes direct,
qualified, imported, and indirect calls after instantiation. Functions without
type binders retain the existing inference rules. Adding generic functions does
not infer universal parameters for previously monomorphic private functions.

Type classes, inferred operator constraints, higher-kinded parameters,
higher-rank callback types, and polymorphic recursion are outside the baseline.
The [first-delivery decisions](#first-delivery-decisions) cite the current
source syntax and define planned inference, runtime representation, and
explicit extension boundaries.

### Independent instantiation and contextual inference

```veln
fn use_identity() -> ()
	let number: Int = identity(42)
	let text: String = identity("hello")
	let explicit: Int = identity<Int>(42)
	()
end
```

Every non-recursive use instantiates the scheme independently. Calling one
instance with `Int` cannot constrain another instance to `Int`. Explicit type
arguments fix the declared parameters in declaration order. A complete list
with the wrong arity, or an argument incompatible with a fixed type, is
rejected. Partial lists, placeholders, and explicit effect-row arguments are
not supported. See [Q1](#q1-source-surface-and-name-resolution).

Value arguments and an available expected result type constrain the same
instance. For `empty<T>() -> Vec<T>`, whose body is `[]`, a `Vec<Int>` expected
result or `empty<Int>()` supplies `T = Int`. An instance with no unique
supported solution is ambiguous; the checker cannot choose an arbitrary type
or leave an unknown type in executable output. A direct call initializing an
unannotated local may retain one pending monomorphic instance for later
same-function completion under [Q3](#q3-context-and-local-completion).

The inference relation must respect the existing nominal identities,
structural function variance, ADT variance, and bounded refinement joins and
meets. This proposal does not add a universal top type or an arbitrary
cross-ADT union to make arguments fit. For a context-free `identity(Ready)`,
the result retains `State::Ready`; a compatible expected `State` may widen
that result. Repeated-parameter selection follows the bounded constraint rules
in [Q4](#q4-bounded-type-constraints), rather than equality unification alone.

A rejected call must not publish tentative substitutions into a reusable
scheme, a sibling call, or an enclosing monomorphic local's inferred facts.
Argument diagnostics remain independently observable. A subsequent valid use
of the same generic function must still infer its own instance.

### Function values and aliases

```veln
fn use_aliases() -> ()
	let f = identity
	let g = f
	let number: Int = f(42)
	let text: String = g("hello")
	let int_only: fn(Int) -> Int = identity
	()
end
```

An unannotated transparent local alias of a generic declaration preserves its
scheme. Transitive aliases preserve it too. A call instantiates the alias
without fixing later uses. These bindings contain known quantifiers; they are
not unresolved monomorphic locals waiting for their first concrete use.

An explicit monotype annotation instead selects one compatible instance. The
`int_only` binding above cannot later accept a `String`. A public function
re-export preserves the target scheme and its declaration identity. Imports
and explicit import aliases preserve the same abstraction.

Ordinary callback parameters still receive monotypes. Passing `identity` to
a parameter of type `fn(Int) -> Int` selects an `Int` instance. A callback of
type `fn(T) -> T` in a generic declaration works at that declaration's one
rigid `T`; it does not promise a callback usable at arbitrary unrelated types.
Using one received callback at both `Int` and `String` would require a
higher-rank type and is outside this baseline.

Transparent aliases do not evaluate a function body. Calling an initializer
once does not create a universally quantified value: `let result = empty()`
and a function returned by a factory follow the monomorphic local rules unless
a future, separately reviewed generalization rule admits them. Selecting a
function through an `if`, aggregate, or field access is not a transparent alias
under the baseline. Such use needs a compatible monotype context; absence of one
must not silently freeze a scheme to its first later use.

This deliberately limited alias rule follows the use-by-use benefit of
[Haskell generalization](https://www.haskell.org/onlinereport/haskell2010/haskellch4.html#sect4.5.2).
It excludes generalization of arbitrary evaluated expressions because of the
shared-state concern illustrated by
[OCaml's value restriction](https://ocaml.org/manual/5.3/polymorphism.html#ss%3Avalrestriction).

### Recursion, effects, and public interfaces

Within a recursive declaration group, recursive references use the group's
declared type parameters rigidly. Ordinary recursion at the same type is
supported. A recursive cycle that changes an instance from `T` to `Vec<T>`
is rejected in the baseline. Independent completed generic helpers can still
be instantiated with expressions involving an enclosing rigid `T`.

Type instantiation must preserve the declared function's effects and the
effects of evaluating arguments. Type parameters cannot stand for effect rows.
A typed callback's concrete effect set must not disappear when passed through
a generic function. The current declaration syntax accepts ordinary type
binders followed by a final effect-row binder, such as `<T, U, effect E>`.
[Q5](#q5-type-and-effect-substitution) defines the remaining joint
instantiation and failure boundary.

Aliases, imports, generated documentation, and tooling must preserve binder
scope and scheme structure. Public visibility checks continue through
instantiated type arguments. Exported interfaces cannot contain an unbound
type variable, a recovery unknown, or an inaccessible nominal type.
The existing proposal for
[contract-bearing callable types](contract-bearing-callable-types.md) remains
a separate design; this proposal does not establish generic proof abstraction
or verified callable contracts.

## Planned acceptance and verification

The table is an acceptance model for future implementation, not a claim that
the current toolchain accepts these programs. The decisions below fix the
expected outcomes and verification destinations. Executable acceptance
evidence belongs to implementation and is not a prerequisite for Ready.

| Input or boundary | Required observation | Planned verification |
| --- | --- | --- |
| `identity<T>(value: T) -> T` used with `Int` and `String`, in either order | Both uses succeed independently and return the matching values. | Semantic type tests and a compiler-instantiation/backend case. |
| `wrap<T>` constructs `Some(value)` | Rigid `T` flows through the constructor and inferred call result. | Semantic tests for generic ADT substitution. |
| `identity<Int>(42)`, wrong explicit arity, or `identity<Int>("hello")` | Accept the first; reject the other two without changing the reusable scheme. | Source and call-check fixtures. |
| `empty<T>() -> Vec<T>` with expected `Vec<Int>` or explicit `<Int>` | Resolve `T` and produce an empty vector of that type. | Contextual call and lowering tests. |
| Result-only instance with no resolving context | A pending direct local initializer reports incompleteness at the binding if unused or unresolved; other unresolved calls report ambiguity at the call. | Diagnostic and later-completion cases under Q3. |
| Direct and transitive unannotated aliases of `identity` | Preserve use-by-use polymorphism in both call orders. | Scheme/alias tests and indirect-call backend case. |
| Annotated `fn(Int) -> Int` alias, then a `String` call | Reject the second call; preserve the binding's explicit type. | Semantic and diagnostic fixtures. |
| Pass `identity` to a monomorphic callback parameter | Instantiate once for that parameter; no higher-rank promise. | Higher-order call tests. |
| Function parameter aliased locally, then used at incompatible types | Reject; aliasing does not generalize the parameter's type. | Negative alias/scope tests. |
| `identity(Ready)` without widening context; compatible explicit base context | Preserve the singleton in the first case and permit widening in the second. | Refinement type assertions. |
| Repeated `T` in input and callback positions, including incompatible invariant arguments | Respect polarity, report conflicts, and infer the same supported result when independent contributions are reordered. | Constraint cases specified under Q4, including argument-order permutations. |
| Failed instance followed by a valid different instance | Failure leaves no tentative type or effect facts in the later call or enclosing local. | Recovery/state-preservation tests. |
| Same-type recursion versus a cycle growing `T` into `Vec<T>` | Accept ordinary recursion; reject polymorphic recursion. | Recursive-group semantic tests. |
| Imported generic function and public re-export | Retain the scheme, nominal identities, visibility, and independent instances across modules/packages. | Import and public-interface cases. |
| Callback with concrete effects; generic function with a row binder | Retain concrete effects and rigid enclosing tails; commit or discard both substitutions together under Q5. | Effect-signature tests and boundary diagnostics. |
| Generated docs, hover, and typed hole inside a generic body | Render the retained declaration binders and show rigid types without exposing internal inference identifiers. | Focused tool output cases. |
| Direct and indirect instances with primitive, record, nested ADT, and callback arguments | Agree on checked result and runtime representation; no cross-instance cast or linkage failure. | JVM lowering and execution boundary cases. |

The current source-surface specification owns accepted and rejected declaration
fixtures plus formatter round trips. Put inference, rigidity, scheme scope,
substitution, and recovery assertions in focused compiler tests. Add checked
examples under `examples/specification/` for observable compiler, diagnostics,
public-interface, and backend contracts. Each backend case must identify the
instantiation or representation failure it detects. Standard-library API
behavior remains in companion tests under `crates/veln-stdlib/veln/`, following
the [test placement policy](../reference/toolchain-test-harness.md#test-placement-policy).
This proposal does not require migrating standard-library APIs in the same
implementation change.

## Related work and alternatives

| Inspected primary source | Finding and applicability limits | Design consequence |
| --- | --- | --- |
| Simon Marlow, editor, [Haskell 2010 Language Report, sections 4.1 and 4.5](https://www.haskell.org/onlinereport/haskell2010/haskellch4.html) | Generalization quantifies variables not fixed by the environment; type classes express overloaded operations. Haskell's equality-based inference and evaluation model do not define Veln's refinement, subtyping, or effect rules. | Adapt independent instantiation and preserve environment-bound variables. Use explicit Veln binders initially; defer constraints and broad inference. |
| Haskell project, [A Gentle Introduction to Haskell, section 12.1](https://www.haskell.org/tutorial/pitfalls.html#sect12.1) | Ordinary received function parameters cannot be instantiated at two unrelated types within the receiver. | Keep callback parameters monomorphic; treat higher-rank callbacks as a separate extension. |
| OCaml manual, version 5.3, [Polymorphism and its limitations, sections 1.1–1.3](https://ocaml.org/manual/5.3/polymorphism.html) | Shared mutable storage and state hidden by a returned function distinguish weak unknowns from universal variables. The value restriction conservatively limits generalization; its relaxed form uses covariance. Veln needs its own safety argument for its effects and resources. | Preserve known schemes through transparent aliases, but defer generalization of evaluated initializers. Q2 excludes broader syntactic and effect-based eligibility from the first delivery. |

Direct-call-only generics would leave `let f = identity` unexpectedly weaker
than its target. Fixing `f` from its first use would make unrelated call order
change acceptance. The baseline rejects those alternatives in favor of
preserving the known scheme. Full Hindley–Milner inference would also generalize
new definitions and locals; that broader change is deferred because Veln's
current inference and explicit public interfaces have different boundaries.
The sources above support these design comparisons, not implementation
correctness or a claim of complete Haskell compatibility.

## First-delivery decisions

Q1–Q9 are resolved for this proposal's first delivery. Excluded extensions need
separate proposals; their absence is not an implementation gap in this scope.
The linked source-surface rules identify the completed declaration syntax; the
remaining decisions specify future semantic and toolchain behavior.

### Q1: Source surface and name resolution

The current
[function declaration binder syntax](../specification/source-surface.md#function-declaration-binders)
owns the accepted list forms, syntax-tree spans, declaration-form exclusions,
parser recovery, and canonical formatting. The
[diagnostic contract](../specification/diagnostics-json.md#diagnostic-families)
owns the corresponding observable diagnostic identifiers, ordering, and
primary spans. The existing expression grammar also retains the name-path and
balanced-angle suffix followed by `(` rule, so a qualified typed call remains
syntactically distinct from comparisons without consulting symbol types.

The current [type rules](../specification/types.md#usage-and-annotations) own
ordinary binder declaration identity, declaration-local scope, and visible-type
shadow rejection for public and private functions. Shadow lookup is scoped by
the declaration's owning package and module. They also require complete
signatures on binder-bearing declarations. The remaining Q1 work is typed-call
resolution and scheme selection.

When an unqualified or qualified `name<Type, ...>(arguments)` resolves to a
scheme, explicit instantiation fixes every ordinary parameter in declaration
order and still infers the effect row. The same rule applies through a
transparent alias. A monomorphic function value cannot receive type arguments.
Standalone `identity<Int>` function values and type application to arbitrary
expressions remain unsupported; an annotated function binding selects a
monomorphic function value instead.

| Remaining typed-call boundary | Required observation |
| --- | --- |
| A typed call whose callee resolves to a scheme | Check the complete explicit list against that scheme in declaration order. |
| A typed call whose callee is monomorphic or whose explicit arity is wrong | Reject without changing any reusable scheme. |

### Q2: Scheme preservation without new generalization

A transparent alias initializer is a name path resolving to a generic
function, an imported/re-exported generic function, or another transparent
alias. Parentheses around that name are transparent. Only an unannotated
single local name binding preserves the complete scheme. No new quantifier
is inferred. Aliasing a monomorphic parameter or local preserves its monotype.

An `if`, record construction, field selection, collection insertion, and
factory call never produce a scheme. A scheme used in those positions needs a
complete monotype expected at that expression, such as an annotated record
field or an annotated `if` result. Without it, report that a monotype context
is required at the scheme use. Later use of the enclosing local cannot repair
that failure or freeze the scheme to its first call. A factory may return an
ordinary monomorphic function whose unknown result parameters use Q3, but
cannot return a quantified function type.

The [OCaml shared-state example](https://ocaml.org/manual/5.3/polymorphism.html#ss:valrestriction)
shows why a returned function need not support unrelated instantiations.
This proposal conservatively preserves existing declared schemes only. Neither
syntactic-value generalization nor an empty inferred effect set is adopted as
a sufficient safety condition for new quantifiers.

### Q3: Context and local completion

An explicit type list fixes the instance. Otherwise use the immediately
available expected result or function-value monotype and the argument facts
under Q4. Context can flow into nested calls and callbacks through those
resolved parameter types. An argument whose type depends on a not-yet-resolved
parameter waits for independent argument or result facts within the same call.
Do not search other function bodies or use a caller to specialize a generic
definition. If the complete call still lacks a supported solution, require a
type annotation or explicit type arguments.

There is one deferred-call boundary: a direct generic call initializing an
unannotated local may retain its fresh instance as part of that binding's
monotype. Every unresolved ordinary parameter must occur in that result type;
an unused parameter in `unused<T>() -> ()` needs an explicit type argument
even in a local initializer. Later concrete same-function contexts listed in
the current [local inference rules](../specification/types.md#inference-rules) may complete
it. This extends existing local completion to generic calls; it does not
create a scheme. For `let xs = empty()`, a later `Vec<Int>` context fixes that
one instance; a subsequent `Vec<String>` use fails. If no use completes it,
report incomplete local inference at `xs`. A standalone unresolved `empty()`
reports ambiguity at the call. Nested unresolved calls must resolve within
their enclosing expression; they do not acquire cross-statement metavariables
through this exception. No pending instance may escape a checked function.

A pending call retains all its type and effect constraints. A completing
context must satisfy them all before any solution is published. A rejected
completion preserves the prior pending binding so a later valid context can
still complete it. Argument errors reject the initializer immediately and
cannot become deferred inference. Argument expressions are evaluated once;
completion is static checking and never repeats runtime evaluation.

### Q4: Bounded type constraints

Adapt the current
[constructor constraint model](../specification/types.md#inference-rules)
and [bounded joins and meets](../specification/types.md#bounded-joins-and-meets).
For an actual argument assignable to a parameter template, start positive in
the template. Record fields and function results preserve polarity; function
inputs reverse it; named arguments compose with checked ADT variance.
Invariant positions require equality. Irrelevant positions contribute no bound.
Rigid enclosing variables compare by declaration identity, never as unknowns.

| Available facts for one parameter | Selected substitution and validation |
| --- | --- |
| Explicit type argument | Fix that type; validate all lower, upper, equality, and result bounds. No inferred widening overrides it. |
| Complete expected result projecting one consistent type for the parameter through matching result structure | Try that contextual type, including a permitted widening, and validate every bound. A conflict rejects the call; do not ignore the context. Repeated incompatible projections require explicit arguments. |
| Equality bounds | Require resolved equality among them; select that type and validate every lower and upper bound. |
| Lower bounds, without a fixed contextual or equality type | Select the complete set's bounded join and validate all upper bounds. |
| Upper bounds only | Select the complete set's bounded meet. |
| No effective bounds, or no unique supported bounded result | Leave pending only under Q3; otherwise report ambiguity. |

Expected results constrain `instantiated result <: expected result`; they do
not reverse argument polarity. The contextual projection row applies to a
bare result `T` or occurrences reached through matching record fields or named
type arguments with known variance. It does not guess a type by inverting
arbitrary subtype relations. Other expected-result shapes contribute their
directional bounds; add an annotation or explicit list if those bounds cannot
select a supported solution. An expected function-value monotype constrains
the whole instantiated function by ordinary function compatibility.

Here `A <: B` means assignment compatibility. The following decision cases
fix the intended outcomes. `State` has exactly `Ready` and `Closed` variants;
`R1 = { x: Int }` and `R2 = { x: Int, y: String }`.

| Constraints or call context | Expected outcome |
| --- | --- |
| `identity(Ready)` without context | `T = State::Ready`. |
| `identity(Ready)` expected as `State` | `T = State` by contextual projection. |
| Two input occurrences receive `Ready` and `Closed`, in either order | `T = State` by the bounded join. |
| Input lower bound `Ready`, callback-input upper bound `State` | `T = State::Ready` without a result context. |
| Input lower bound `State`, callback-input upper bound `State::Ready` | Reject the incompatible bounds. |
| Lower bounds `R1` and `R2`, in either order | `T = R1`, the unique admitted contributed record accepting both. Do not invent other record fields. |
| Only unrelated lower bounds `Int` and `String` | Reject; no supported common type exists. |
| Two invariant ADT occurrences require `State::Ready` and `State::Closed` | Reject even though their covariant join would be `State`. |
| `fn(fn(T) -> ()) -> ()` inside a parameter template | Two input reversals give a positive occurrence of `T`. |
| Explicit `<State::Ready>` with a `Closed` argument or incompatible result context | Reject without widening the fixed argument. |
| A parameter occurs only in irrelevant positions | Require explicit or result context; no arbitrary default type. |

Use the complete set of independently available contributions, not a pairwise
fold that discards an unresolved prefix. No universal top, cross-ADT union,
synthetic intersection, or type-class constraint is introduced. Validate the
complete argument shapes too; an unrelated outer ADT cannot contribute a
usable nested bound. Rejection leaves reusable schemes, sibling instances,
and enclosing monomorphic inference unchanged. Diagnostics for independent
arguments remain available even when another argument fails.

### Q5: Type and effect substitution

The current declaration syntax accepts combined `<T, U, effect E>` lists. The
remaining Q5 work gives type and row variables distinct semantic roles and
substitutions. Transparent aliases retain both binder lists. Expected callback
monotypes constrain ordinary types and effect inclusion together. Recheck
callback compatibility against the complete substitution; resolving one kind
must not discard the other's facts.

Use the existing
[row solver contract](../specification/types.md#compatibility-and-limits):
choose the least row satisfying all lower bounds, check all upper bounds, and
retain rigid enclosing tails. A call-bound row without a lower bound becomes
empty, even when it occurs only in a result; an unresolved ordinary type does
not default to any type. Concrete declared effects and argument-evaluation
effects remain observable. An enclosing rigid row is never replaced by empty.

For `apply<T, U, effect E>(f: fn(T) -> U effects [...E], x: T) -> U effects [...E]`,
where the body is `f(x)`, a pure `fn(Int) -> String` callback and `Int` input
infer `T = Int`, `U = String`, and an empty call row. A callback with `stdio`
adds `stdio`; a callback using an enclosing rigid row retains that tail.
An incompatible `x` or an effect upper-bound conflict rejects the call and
publishes neither its tentative type result nor its row solution. This does
not suppress independent diagnostics or legitimate effects from evaluating
argument expressions. In Q3's pending-local case, keep the joint constraints
local until completion. Row-only declarations retain their current behavior.

### Q6: Shared JVM implementation

Choose erased, shared implementations for the first delivery. Do not generate
a method per concrete type tuple. One checked declaration has one runtime
body; ordinary type and row parameters have no runtime type argument. Keep
semantic type arguments through checking and typed lowering for visibility,
compatibility, and diagnostics, then erase their quantifiers for emission.
Schema reflection or operations requiring a concrete runtime type descriptor
cannot accept arbitrary `T` without a separately designed witness mechanism.

At proposal revision `c868eb540`, the design direction is supported by the
existing all-Object method
boundary in `object_method_descriptor` in
`crates/veln-backend-jvm/src/classfile/contracts_and_tail_recursion.rs` and the
direct-call emission in
`crates/veln-backend-jvm/src/classfile/call_emission.rs`. Primitive literal
emission already boxes values in `crates/veln-backend-jvm/src/classfile.rs`.
These inspected mechanisms reduce representation risk; they are not evidence
that a generic source body currently lowers successfully.

Direct, imported, and indirect calls must use compatible boxed primitive,
record, ADT, and callback representations. A transparent alias refers to the
same body, not to a first-use specialization. Same-type recursion calls that
body. The rigid recursive-group rule still rejects type-changing recursion;
erasure does not relax typing. Acyclic generic helpers may receive `Vec<T>`
without generating an expanding family of methods. No generation cutoff acts
as a typing rule. Defer monomorphization and hybrid specialization until a
separate performance case justifies their complexity.

The implementation approach is to check generic bodies once with rigid
parameters, retain scheme and call-substitution information through typed
lowering, and emit the existing Object call boundary without runtime type
arguments. Aliases and imported references select the owning declaration's
shared body; callbacks use the existing function-value call boundary. This
explains how the baseline can be implemented. Ready does not require a separate
JVM prototype. The representation matrix, shared-body checks, and effect
preservation cases above remain acceptance requirements for implementation;
the existing ABI does not establish that they already pass.

### Q7: Artifacts, tooling, and diagnostics

The syntax tree and lowered AST already retain ordered type binders, the
optional row binder, and source spans. Semantic function declarations identify
each ordinary binder by owning declaration identity and ordinal. Printed names
are presentation, not identity. The remaining scheme representation must also
distinguish ordinary binders, effect-row binders, instantiations, and recovery
unknowns. Core and typed IR retain schemes on declarations and each accepted
call's substitution until the erasure boundary; JVM emission follows Q6.

Package/interface artifacts retain ordered binders, signature structure,
effects, and referenced nominal declaration identities. Re-exporting a function
retains its owning scheme identity. Round-trip loading and separate consumer
checking must preserve independent instantiation. Version any affected
artifact schema and reject an unsupported version explicitly instead of
silently loading a generic signature as monomorphic. Do not require stable
internal inference-variable numbers across runs.

Generated docs and declaration hover render `fn identity<T>(value: T) -> T`
and the combined binder spelling from Q1. Call hover shows its instantiated
signature. A typed hole checked as rigid `T` reports `T` in its binder scope;
candidate matching cannot offer `Int` as arbitrary `T`. A local of that same
rigid `T` is an eligible candidate. JSON identifies binder provenance without
using machine addresses or exposing transient solver identifiers as identities.

| Remaining diagnostic work | Primary diagnostic location | Related context to add |
| --- | --- | --- |
| Missing binder provenance for an invalid body result or operation on rigid `T` | Existing failed expression or operand | Binder declaration and required type. |
| Wrong explicit type arity | Type argument list | Declared binder list and required count. |
| Incompatible argument or fixed substitution | Failed argument or explicit type argument | Relevant parameter/binder and conflicting bound. |
| Unresolved immediate instance | Call expression | Unresolved binder and annotation/explicit-argument hint. |
| Unresolved pending local | Omitted local binding | Initializing call and unresolved binder. |

The current
[diagnostic contract](../specification/diagnostics-json.md#diagnostic-families)
owns the implemented binder-shadow identifier, primary span, details, and
related visible declaration. Choose identifiers for the remaining rows during
their implementation using the repository's existing registry. Human and JSON
cases must establish those failure facts and spans; human output must include
the related binder context. Fixing names or serialized field layouts without
their implementation would not establish that contract.

### Q8: Callable forms and executable contracts

The source parser restricts ordinary type binders to ordinary fixed-arity
functions and rejects generic `test`, variadic, and `callsite` declarations.
The current [type](../specification/types.md#usage-and-annotations) and
[contract](../specification/contracts.md#predicate-usage-and-validation)
specifications own rigid binder scope through function signatures, bodies, and
executable contracts. Existing nongeneric and row-only forms retain their
current rules.

The remaining scheme implementation must not expose a partially generic
signature. Calls in contracts instantiate independent helpers. A fixed-arity
generic function may accept a compatible monomorphic callback, subject to
existing callback compatibility. Generic static proof abstraction and
contract-bearing callable types remain separate proposals.

### Q9: Larger polymorphism

Exclude type classes/traits, inferred constrained signatures, higher-kinded
parameters, higher-rank callable types, and annotated polymorphic recursion.
The current
[function declaration binder syntax](../specification/source-surface.md#function-declaration-binders)
already rejects declaration-list bounds. The remaining work rejects nested
quantifiers in callback, field, or element types rather than erasing them.
A recursive cycle changing a declared rigid instance fails at the changing
call, even with explicit type arguments. This is an observable first-delivery
boundary.
Each extension needs its own proposal, safety argument, and acceptance model;
none blocks the unconstrained rank-one baseline.

## Readiness and lifecycle

The baseline is ready for implementation because Q1–Q9 resolve the design
choices and the implementation approach below identifies how they connect to
the existing compiler. Readiness requires a sufficiently clear way to implement
the selected behavior, not a separate prototype or passing implementation
tests. The acceptance tables specify what implementation must verify.
Excluded extensions do not block the baseline.

### Implementation approach

This is implementation guidance, not an additional observable language
contract. The existing mechanisms provide starting points; generic-function
support still needs the changes and acceptance coverage described here.

| Concern | Existing starting point | Planned change |
| --- | --- | --- |
| Typed generic calls | Function syntax retains ordinary and effect binders with spans, declaration checking resolves ordinary binders by declaration identity, and named-call suffixes distinguish balanced type arguments from comparisons. | Connect typed calls to schemes under Q1 without changing the source grammar. |
| Rigid definitions and independent instances | Semantic function signatures carry declaration-owned ordinary binders through complete annotations and rigid body checking; current private inference applies only to declarations without ordinary binders and rejects signature contributions containing another declaration's rigid parameter. | Represent independently instantiable schemes and create fresh substitutions at non-recursive uses. Keep recursive-group references rigid under Q9. |
| Type and effect inference | Constructor inference already collects polarity-sensitive bounds, and effect calls already substitute one row. | Apply Q3–Q5 to generic call instances, retaining complete constraints and publishing type and row solutions together. Pending locals retain one monomorphic instance. |
| Aliases and public interfaces | Name resolution identifies declarations and imports; function values already have monomorphic signatures. | Preserve the owning scheme for transparent aliases and exports, instantiate at monotype boundaries, and retain binder identity in versioned interfaces under Q7. |
| JVM emission | Object method descriptors, primitive boxing, direct calls, and function-value calls already provide erased runtime boundaries. | Retain semantic substitutions through typed lowering, then emit one shared body per declaration as described in Q6. No runtime type witness or specialization mechanism is needed for the baseline. |

Q1 identifies the current declaration syntax. Q4 and Q5 identify the current
inference and effect rules being extended, and Q6 identifies the relevant
backend code. This provides an implementation route without introducing a
second parser, solver, or backend as a readiness gate.
Production integration may still reveal a design conflict. Revisit the affected
decision if implementing an acceptance case requires changing the selected
behavior or its scope; ordinary implementation work and missing tests do not
by themselves make the design unready.

### Production acceptance still required

| Planned case | Required observation | Verification destination |
| --- | --- | --- |
| Parenthesized and imported alias chains, with calls in both type orders | Preserve type and row schemes without first-use freezing. | Scheme tests and indirect JVM execution. |
| `if`, record field, or factory-selected callable with and without a complete monotype context | Accept only the supported monotype use; reject attempts to retain quantifiers. | Context and negative generalization tests. |
| `let xs = empty()` followed by a `Vec<Int>` context, then a `Vec<String>` context | Complete once as `Vec<Int>`; reject the later incompatible use. | Local inference and human/JSON cases. |
| Pending local completion conflicts with retained bounds, followed by valid completion | Failed completion leaves the pending constraints intact; valid completion succeeds. | Recovery tests with type and row assertions. |
| Unused pending local, standalone `empty()`, or `unused<T>() -> ()` without explicit arguments | Diagnose the unresolved local or call at the Q3 boundary. | Ambiguity and incompleteness diagnostics. |
| Combined `apply` with pure, `stdio`, and rigid enclosing-row callbacks; incompatible input or row upper bound | Produce the Q5 substitutions; publish neither tentative type nor row on failure. | Joint-solver tests and effect-boundary cases. |
| Generic signature exported, serialized, loaded, and re-exported | Preserve owner identity, arity, visibility, and independent use; reject an unsupported schema version. | Artifact round-trip and separate consumer tests. |
| Rigid-`T` hole with a local of `T` and an unrelated `Int` local | Render `T`; include the former candidate and exclude the latter. | Hover/hole JSON and human output cases. |
| A rejected concrete-only contract operation on rigid `T` | Add binder provenance to the existing operation-local type mismatch. | Human and JSON contract diagnostic cases with related binder context. |

During implementation, move remaining supported behavior and checked acceptance
examples into the current type, effect, and affected tooling
specifications. Keep only incomplete scope here. Remove this proposal and its
catalog entry when no active scope remains. Current specifications and root
feature summaries must not advertise generic functions merely because this
proposal exists.
