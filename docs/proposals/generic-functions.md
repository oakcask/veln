---
role: proposal
update-when: Generic function binder syntax, rigid body checking, call instantiation, function alias polymorphism, type and effect inference, public interface retention, lowering feasibility, or generic function acceptance decisions change.
---

# Generic Functions

Support explicitly declared generic functions whose type arguments are inferred
independently at each use. Preserve that polymorphism through transparent
function aliases, and check function bodies against their declared abstraction.
This proposal is a Ready implementation target. Q1–Q9 have first-delivery
decisions or explicit exclusions, supported by the bounded
[feasibility experiment](#feasibility-evidence-and-limits). The examples and
acceptance cases below specify planned production behavior; they are not
passing examples for the current Veln toolchain.

## Purpose and current boundary

Generic source ADTs can describe containers and recursive structures, but users
also need reusable functions that transport and transform their payloads.
Writing an `Int` and a `String` version of the same structural operation makes
that abstraction depend on concrete element types.

The current [type specification](../specification/types.md) defines generic
constructor inference, variance, variant refinements, and monomorphic local
inference. Private function inference requires concrete types and can use
same-module call-site facts. That inference is not universal quantification.
The current [source grammar](../specification/source-surface.md#executable-grammar)
allows an effect binder on a function, but no ordinary type-parameter binder.
The [effect specification](../specification/effects.md#effect-labels) already
supports one declared effect-row parameter and call-specific substitution.

At investigation base `fb433dd78`, `FunctionSignature` in
`crates/veln-sema/src/types/signatures.rs` contains parameter, result, and
effect information but no ordinary quantified type-parameter list. `Type` in
`crates/veln-sema/src/semantic_model.rs` distinguishes unknown, named,
refinement, record, and function types, without an explicit ordinary type
variable or universal scheme variant. These observations identify feasibility
work; they do not prescribe the replacement representation.

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
The [first-delivery decisions](#first-delivery-decisions) define syntax,
inference, runtime representation, and explicit extension boundaries.

### Declaration and body checking

Declaration syntax reuses the ADT type-parameter spelling:

```veln
pub fn identity<T>(value: T) -> T
	value
end

pub fn wrap<T>(value: T) -> Option<T>
	Some(value)
end
```

`T` denotes one rigid type throughout its declaration's signature, body, and
contract annotations. Its identity belongs to the declaration, not its printed
name. Renaming a binder consistently preserves meaning. Equal binder spelling
in another declaration does not share a type. A type parameter denotes an
ordinary type, not a constructor that can itself receive arguments.

The body must be valid for every admitted substitution. Returning the input,
placing it in `Option<T>`, and passing it to another compatible generic
function are valid. Returning `0` as an arbitrary `T`, treating `T` as `Int`,
or applying integer addition to arbitrary `T` is invalid, even if all observed
callers use `Int`. Checking cannot specialize an invalid source definition into
valid instances. The same condition applies to operations in contract clauses.
No operation acquires a new implicit type-class constraint from this proposal.

The proposed binder is visible in local type annotations. It cannot escape
through an unbound public type. Duplicate type-parameter names are invalid.
A binder cannot shadow a visible type name, including a built-in type or an
imported type alias. Type and effect binders in one list must have distinct
names. Value and field names remain in their existing namespaces. Public
generic functions retain explicit parameter, result, and effect boundaries; client calls cannot infer their exported abstraction.

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
a generic function. Generic type and effect-row parameters compose in one
binder list, such as `<T, U, effect E>`.
[Q5](#q5-type-and-effect-substitution) defines their joint instantiation and
failure boundary.

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
expected outcomes. The feasibility experiment checks a bounded subset;
production acceptance evidence remains implementation work.

| Input or boundary | Required observation | Planned verification |
| --- | --- | --- |
| `identity<T>(value: T) -> T` used with `Int` and `String`, in either order | Both uses succeed independently and return the matching values. | Semantic type tests and a compiler-instantiation/backend case. |
| `wrap<T>` constructs `Some(value)` | Rigid `T` flows through the constructor and inferred call result. | Semantic tests for generic ADT substitution. |
| Generic definition returns `0` as arbitrary `T`, or uses integer `+` on `T` | Definition fails even when its only caller supplies `Int`, and when it has no callers. | Definition-check and human/JSON diagnostic cases. |
| Two generic definitions both spell a parameter `T`; consistently rename one | Separate identities; equivalent checking after renaming. | Binder and capture-avoiding substitution tests. |
| Duplicate binder or binder outside its declaration | Reject at the failed binder or type use. | Accepted/rejected grammar and semantic fixtures. |
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
| Formatter, AST, generated docs, hover, and typed hole inside a generic body | Preserve/render binders; show rigid types without exposing internal inference identifiers. | Surface round trips and focused tool output cases. |
| Direct and indirect instances with primitive, record, nested ADT, and callback arguments | Agree on checked result and runtime representation; no cross-instance cast or linkage failure. | JVM lowering and execution boundary cases. |

Use accepted/rejected fixtures for the executable source grammar and formatter
round trips once syntax is decided. Put inference, rigidity, scheme scope,
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
These decisions specify future behavior, not current compiler support.

### Q1: Source surface and name resolution

Use one nonempty binder list after the function name. Ordinary type parameters
come first. At most one `effect E` entry comes last. Permit one trailing comma.
Type parameters have ordinary kind: `T<Int>` is invalid. The binder
scope includes the signature, body, and executable contracts, and excludes
other declarations. The collision rules in Declaration and body checking
apply equally to public and private functions.

Explicit instantiation uses `name<Type, ...>(arguments)` on an unqualified or
qualified name resolving to a scheme, including a transparent alias. It fixes
all ordinary parameters in declaration order and still infers the effect row.
Do not add standalone `identity<Int>` function values or type application to
arbitrary expressions. Use `let f: fn(Int) -> Int = identity` to select a
function value. A monomorphic function value cannot receive type arguments.

Retain the existing name-path and balanced-angle suffix followed by `(` rule
in `crates/veln-syntax/src/parser/expression_core.rs`; do not consult symbol
types to parse the suffix. Consequently `f<T>(x)` parses as a typed call even
when `f` later fails scheme lookup. Use parentheses to express the competing
comparison interpretation. `x < y` remains a comparison. Qualified calls,
nested `>>` closers, trailing commas, and formatted output must round-trip.
The formatter writes type binders before the effect binder with comma-space
separators and no trailing comma in a single-line list. This adapts the current
ADT spelling and call suffix instead of introducing a second application form.
The following are production source-surface acceptance targets. The prototype
checks binder lists and typed-call lookahead separately; it does not establish
production parser or formatter conformance.

| Source shape | Acceptance and formatter target |
| --- | --- |
| `fn f<T>(x: T) -> T` | Accept; retain `<T>`. |
| `fn f<T, U, effect E,>(x: T, f: fn(T) -> U effects [...E]) -> U effects [...E]` | Accept; print `<T, U, effect E>`. |
| `fn f<effect E>(...)` | Retain existing row-only syntax and behavior; `...` here abbreviates the signature. |
| `fn f<>(...)`, `fn f<effect E, T>(...)`, `fn f<T, T>(...)`, or `fn f<T, effect T>(...)` | Reject the empty, misordered, or duplicate binder. |
| `fn f<T: Trait>(...)` or `f<_, Int>(x)` | Reject bounds or placeholders. |
| `module::identity<Vec<Int>>([1])` | Accept nested closers and qualified application. |
| `f<Int,>(1)` after `let f = identity` | Accept the alias call; print `f<Int>(1)`. |
| `let f = identity<Int>` or `factory()<Int>(1)` | Reject unsupported standalone or expression type application. |
| `x < y` | Preserve comparison parsing. |
| `f<T>(x)` | Parse as typed call; subsequent name/scheme checking decides validity. |
| `(f < t) > x` | Parse both operators as comparisons. |

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

Support combined `<T, U, effect E>` declarations in the first delivery. Type
and row variables have distinct roles and substitutions. Transparent aliases
retain both binder lists. Expected callback monotypes constrain ordinary types
and effect inclusion together. Recheck callback compatibility against the
complete substitution; resolving one kind must not discard the other's facts.

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

The feasibility experiment executes generated identity, wrapping, callback,
and same-type-recursive bodies from a separately compiled consumer. It checks
aliases, the representation matrix, and one body per declaration. The type/row
model separately checks effect retention; the Java consumer checks that a
callback runs once. Production module resolution, effect dispatch, and backend
integration still need the acceptance coverage above. The current Object ABI
alone cannot establish those outcomes.

### Q7: Artifacts, tooling, and diagnostics

AST output retains ordered type binders, the optional row binder, and source
spans. Semantic schemes identify each binder by owning declaration identity,
kind (ordinary type or effect row), and ordinal. Printed names are presentation,
not identity. Instantiations and recovery unknowns are distinct from rigid
binders. Core and typed IR retain schemes on declarations and each accepted
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

| Failed fact | Primary diagnostic location | Related context |
| --- | --- | --- |
| Invalid body result or operation on rigid `T` | Failed expression or operand | Binder declaration and required type. |
| Duplicate or shadowing binder | Conflicting binder token | Earlier binder or visible type declaration. |
| Wrong explicit type arity | Type argument list | Declared binder list and required count. |
| Incompatible argument or fixed substitution | Failed argument or explicit type argument | Relevant parameter/binder and conflicting bound. |
| Unresolved immediate instance | Call expression | Unresolved binder and annotation/explicit-argument hint. |
| Unresolved pending local | Omitted local binding | Initializing call and unresolved binder. |
| Unsupported generic callable form | Variadic or `callsite` marker | Generic binder and fixed-arity ordinary-function boundary. |

Choose diagnostic identifiers during implementation using the repository's
existing registry. Human and JSON cases must establish these failure facts
and spans; human output must include the related binder context. Fixing names
or serialized field layouts without their implementation would not establish
that contract.

### Q8: Callable forms and executable contracts

The first delivery accepts ordinary fixed-arity generic functions only. Reject
an ordinary type binder on a variadic or `callsite` declaration at its unsupported
marker; do not silently drop the quantifier or expose a partially generic
signature. Existing nongeneric and row-only forms retain their current rules.
A fixed-arity generic function may still accept a compatible monomorphic
callback, subject to existing callback compatibility.

Check every executable precondition and postcondition under the same rigid
binders as the body. Calls in contracts instantiate independent helpers;
contracts cannot specialize `T` from callers. A contract passing a `T` to an
`Int`-only operation is invalid even when all callers use `Int`. A contract
using an operation already valid for that rigid type follows the existing
contract execution and effect rules. No new equality, ordering, schema, or
proof capability is granted merely by writing a binder. Generic static proof
abstraction and contract-bearing callable types remain separate proposals.

### Q9: Larger polymorphism

Exclude type classes/traits, inferred constrained signatures, higher-kinded
parameters, higher-rank callable types, and annotated polymorphic recursion.
Reject bounds in binder lists and quantifiers in callback, field, or element
types rather than erasing them. Unsupported operations on unconstrained `T`
fail at that operation; no hidden constraint is inferred. A recursive cycle
changing a declared rigid instance fails at the changing call, even with
explicit type arguments. These are observable first-delivery boundaries.
Each extension needs its own proposal, safety argument, and acceptance model;
none blocks the unconstrained rank-one baseline.

## Readiness and lifecycle

The baseline is ready for implementation: Q1–Q9 select its behavior, and the
experiment below supplies source-surface, constraint/alias, and JVM feasibility
evidence. Readiness accepts the design and its bounded feasibility; it does
not claim production conformance. Excluded extensions do not block this
baseline. The following cases remain production acceptance requirements,
including boundaries not represented in the experiment.

### Feasibility evidence and limits

Run the retained experiment from the repository root with Python 3.10 or newer
and a JDK providing `javac` and `java` (17 or newer):

```sh
bash scripts/agent-run python3 -B tools/prototypes/generic-functions/run.py
```

The runner checks the independent model, checks and lowers the bounded source
fixture, compiles the generated provider, compiles its consumer against the
class files separately, and executes it with JVM verification enabled. Build
output lives in a temporary directory and is removed when the run ends.
The experiment is a local feasibility gate; it is not wired into CI and is not
part of the installed compiler or executable language specification.

| Design concern | Retained evidence and observation | Boundary of that evidence |
| --- | --- | --- |
| Q1 binder and application syntax | `surface.py` and `test_model.py` in `tools/prototypes/generic-functions/` check ordinary/combined/row-only lists, trailing-comma normalization, invalid lists, qualified nested type arguments, and comparison lookahead. | A separate subset parser, not the production grammar or formatter. It rejects expression type application within its subset; production may parse a comparison and reject during checking. |
| Q2 independent schemes and Q3 local completion | The immutable scheme model in `model.py` preserves alias owner identity in both call orders, selects annotated monotypes, rejects nontransparent scheme uses, and publishes pending-local solutions only after validation. Tests reject an incompatible completion and then accept a valid one with retained bounds. | Import/re-export lookup is modeled by name-to-scheme bindings. Source binding classification, pending-instance creation/escape checks, and evaluation-once enforcement require production integration. |
| Q4 bounded constraints | The model checks contextual widening, fixed arguments, lower/upper/equality constraints, nested polarity, invariant nesting, nominal shape rejection, record width, irrelevant positions, and permutations of complete contributions. A function-bound case has an unresolved two-contribution prefix but resolves with the third. | The model uses a finite type vocabulary and explicit variance metadata; it does not compute ADT variance or replace the current bounded solver. |
| Q5 joint type/effect failure boundary | Tests retain pure, concrete, and rigid-tail callback effects, default an unconstrained call row to empty, and reject input or row-upper-bound conflicts without changing the scheme or pending binding. | One call-bound row and set inclusion model the proposed solver boundary. These checks do not implement host effects, effect handlers, or nested contextual scheduling. |
| Definition rigidity and recursion | `functions.veln` is checked before emission. Negative cases reject a concrete result or integer operation on rigid `T`, including a definition whose only caller supplies `Int`. Same-type recursion passes; explicit and inferred type-growing self-recursion fail. | The subset checker has no caller-fact input. Mutual recursive-group detection, contracts, visibility, and diagnostics are production acceptance work. |
| Q6 runtime representation | The separately compiled `Consumer.java` exercises five generated shared bodies with boxed `Int`, `Float`, `Bool`, `String`, records, nested ADTs, and callable payloads in both orders. Reflection checks exactly one Object-only method per declaration and no runtime type parameters. A persistent alias alternates representations; an effectful callback runs once. | Java is the prototype emission format and consumer language. This checks JVM erasure feasibility, not the Veln backend's bytecode writer, package loader, runtime dispatch, or production closure adapters. |

The local run passes 19 model/surface tests and 112 JVM assertions. Those
assertions use the proposal's decision tables as their oracle; they are not
independent proof of the whole type system. Review of the retained model checks
that invariant nesting requires equality, complete contributions survive an
unresolved prefix, fixed and contextual substitutions validate every bound,
and rejected solutions cannot mutate schemes or pending state. The separate
source-to-JVM path checks that valid rigid bodies have a shared erased
implementation without runtime type witnesses. Together these observations
remove the identified feasibility blockers for the baseline.

Artifact round trips and version rejection, real import/re-export resolution,
full parser/formatter round trips, source alias binding, recursive groups,
contracts, tooling, and human/JSON diagnostics remain planned implementation
acceptance. Re-run the experiment when its inputs or modeled rules change;
revisit Ready status if it cannot reproduce the selected boundary, or if
production integration reveals that the baseline needs a new design choice.
Retire each prototype component once production tests cover its concern,
and remove the experiment when no feasibility-only scope remains.

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
| Generic variadic or `callsite` declaration, or a contract using an `Int` operation on `T` | Reject at the unsupported marker or failed contract operation, with binder provenance. | Negative declaration and contract diagnostic cases. |

During implementation, move supported behavior and checked acceptance examples
into the current source-surface, type, effect, and affected tooling
specifications. Keep only incomplete scope here. Remove this proposal and its
catalog entry when no active scope remains. Current specifications and root
feature summaries must not advertise generic functions merely because this
proposal exists.
