---
role: proposal
update-when: Generic function binder syntax, rigid body checking, call instantiation, function alias polymorphism, type and effect inference, public interface retention, lowering feasibility, or generic function acceptance decisions change.
---

# Generic Functions

Support explicitly declared generic functions whose type arguments are inferred
independently at each use. Preserve that polymorphism through transparent
function aliases, and check function bodies against their declared abstraction.
This is a Design In Progress proposal, not implemented behavior or a Ready
implementation target. The examples and acceptance cases below are proposed
contracts; they are not checked, passing Veln examples.

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
Their possible extensions and other unresolved choices are collected under
[Open Questions](#open-questions).

### Declaration and body checking

Candidate syntax reuses the ADT type-parameter spelling:

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
The spelling and name-resolution policy for other binder collisions remain
open. Public generic functions retain explicit parameter, result, and effect
boundaries; client calls cannot infer their exported abstraction.

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
rejected. Partial explicit lists remain open.

Value arguments and an available expected result type constrain the same
instance. For `empty<T>() -> Vec<T>`, whose body is `[]`, a `Vec<Int>` expected
result or `empty<Int>()` supplies `T = Int`. An instance with no unique
supported solution is ambiguous; the checker cannot choose an arbitrary type
or leave an unknown type in executable output. Whether later local uses can
complete a result-only instance is an open decision.

The inference relation must respect the existing nominal identities,
structural function variance, ADT variance, and bounded refinement joins and
meets. This proposal does not add a universal top type or an arbitrary
cross-ADT union to make arguments fit. For a context-free `identity(Ready)`,
the result retains `State::Ready`; a compatible expected `State` may widen
that result. Exact repeated-parameter constraint selection is an open question
because refinement compatibility is richer than equality unification.

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
a separately reviewed generalization rule admits them. Selecting a function
through an `if`, aggregate, or field access is not a transparent alias under
the baseline. Such use needs a compatible monotype context; absence of one
must not silently freeze a scheme to its first later use.

This deliberately limited alias rule follows the use-by-use benefit of
[Haskell generalization](https://www.haskell.org/onlinereport/haskell2010/haskellch4.html#sect4.5.2).
It defers generalization of arbitrary evaluated expressions because of the
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
a generic function. Generic type and effect-row parameters should compose,
but their joint binder syntax and inference contract require the decision
described in Open Questions before that combination is Ready.

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
the current toolchain accepts these programs. Rows depending on open questions
must acquire explicit expected outcomes before Ready promotion.

| Input or boundary | Required observation | Planned verification |
| --- | --- | --- |
| `identity<T>(value: T) -> T` used with `Int` and `String`, in either order | Both uses succeed independently and return the matching values. | Semantic type tests and a compiler-instantiation/backend case. |
| `wrap<T>` constructs `Some(value)` | Rigid `T` flows through the constructor and inferred call result. | Semantic tests for generic ADT substitution. |
| Generic definition returns `0` as arbitrary `T`, or uses integer `+` on `T` | Definition fails even when its only caller supplies `Int`, and when it has no callers. | Definition-check and human/JSON diagnostic cases. |
| Two generic definitions both spell a parameter `T`; consistently rename one | Separate identities; equivalent checking after renaming. | Binder and capture-avoiding substitution tests. |
| Duplicate binder or binder outside its declaration | Reject at the failed binder or type use. | Accepted/rejected grammar and semantic fixtures. |
| `identity<Int>(42)`, wrong explicit arity, or `identity<Int>("hello")` | Accept the first; reject the other two without changing the reusable scheme. | Source and call-check fixtures. |
| `empty<T>() -> Vec<T>` with expected `Vec<Int>` or explicit `<Int>` | Resolve `T` and produce an empty vector of that type. | Contextual call and lowering tests. |
| Result-only instance with no resolving context | Report ambiguity or incompleteness according to the selected local-completion rule; never publish an arbitrary instance. | Diagnostic cases finalized under Q3. |
| Direct and transitive unannotated aliases of `identity` | Preserve use-by-use polymorphism in both call orders. | Scheme/alias tests and indirect-call backend case. |
| Annotated `fn(Int) -> Int` alias, then a `String` call | Reject the second call; preserve the binding's explicit type. | Semantic and diagnostic fixtures. |
| Pass `identity` to a monomorphic callback parameter | Instantiate once for that parameter; no higher-rank promise. | Higher-order call tests. |
| Function parameter aliased locally, then used at incompatible types | Reject; aliasing does not generalize the parameter's type. | Negative alias/scope tests. |
| `identity(Ready)` without widening context; compatible explicit base context | Preserve the singleton in the first case and permit widening in the second. | Refinement type assertions. |
| Repeated `T` in input and callback positions, including incompatible invariant arguments | Respect polarity, report conflicts, and infer the same supported result when independent contributions are reordered. | Constraint decision cases finalized under Q4. |
| Failed instance followed by a valid different instance | Failure leaves no tentative type or effect facts in the later call or enclosing local. | Recovery/state-preservation tests. |
| Same-type recursion versus a cycle growing `T` into `Vec<T>` | Accept ordinary recursion; reject polymorphic recursion. | Recursive-group semantic tests. |
| Imported generic function and public re-export | Retain the scheme, nominal identities, visibility, and independent instances across modules/packages. | Import and public-interface cases. |
| Callback with concrete effects; generic function with a row binder | Retain concrete effects; settle joint type/row substitution and rejection behavior under Q5. | Effect-signature tests and boundary diagnostics. |
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
| OCaml manual, version 5.3, [Polymorphism and its limitations, sections 1.1–1.3](https://ocaml.org/manual/5.3/polymorphism.html) | Shared mutable storage and state hidden by a returned function distinguish weak unknowns from universal variables. The value restriction conservatively limits generalization; its relaxed form uses covariance. Veln needs its own safety argument for its effects and resources. | Preserve known schemes through transparent aliases, but defer generalization of evaluated initializers. Q2 compares syntactic and effect-based eligibility. |

Direct-call-only generics would leave `let f = identity` unexpectedly weaker
than its target. Fixing `f` from its first use would make unrelated call order
change acceptance. The baseline rejects those alternatives in favor of
preserving the known scheme. Full Hindley–Milner inference would also generalize
new definitions and locals; that broader change is deferred because Veln's
current inference and explicit public interfaces have different boundaries.
The sources above support these design comparisons, not implementation
correctness or a claim of complete Haskell compatibility.

## Open Questions

These decisions are unresolved. Suggested directions below are candidates,
not additional accepted grammar or hidden implementation requirements.

| ID | Decision needed | Alternatives and evidence required |
| --- | --- | --- |
| Q1 | Binder and explicit-instantiation syntax | Settle how `<T, U>` combines with existing `<effect E>`, binder ordering, trailing commas, name collisions/shadowing, and explicit type arguments on aliases and function values. A combined `<T, U, effect E>` list is one candidate. Decide partial lists and placeholders. Require parser/formatter accepted/rejected examples, including qualified calls and comparison ambiguities. |
| Q2 | Generalization beyond transparent aliases | Keep the narrow baseline, admit syntactic values, or use type-and-effect eligibility. Decide `if` results, records, field selection, and factory-produced functions explicitly. A safety argument must cover shared state, resources, and effects; purity of one initializer alone is not assumed sufficient. Do not import Haskell's monomorphism restriction as an unexplained special case. |
| Q3 | Ambiguity timing and expected result types | Decide whether `let xs = empty()` can be completed by a later `Vec<Int>` use, consistently with existing monomorphic local completion. Decide how much result context directs arguments and whether nested bidirectional cases need annotations. None of these choices may generalize a call result into a new scheme. |
| Q4 | Type constraint solutions with refinements and variance | Decide how repeated occurrences of `T` combine lower, upper, and invariant bounds, and how explicit arguments interact with widening. Compare bounded join/meet reuse with a stricter annotated first slice. Require examples for mixed refinement inputs, record width, nested callbacks, and invariant ADTs; define ambiguity when no unique supported solution exists. |
| Q5 | Combined type and effect polymorphism | Decide joint substitutions, alias retention of row binders, expected callback context, unresolved rows, and failed-call rollback. Preserve the existing row-only behavior unless a separately justified change is specified. Require both pure and effectful callback cases, including rigid enclosing rows. |
| Q6 | Backend representation and specialization | Compare erased shared implementations, monomorphization, and hybrids. Demonstrate primitive/record/ADT/callback instances, imported and indirect calls, recursion, and value representation. If specializing, define treatment of unbounded instance growth through generic helper references as well as recursive calls; do not mistake a generation cutoff for a typing rule. |
| Q7 | Public artifacts and tooling contracts | Decide AST/core/typed-IR and package/interface serialization changes, generated docs and hover spelling, typed-hole expectations/candidates, and diagnostics. Choose stable identifiers and primary spans only after the semantic failure facts are fixed. Human causes and binder provenance belong in related notes. |
| Q8 | Remaining callable forms and contracts | Decide whether variadic and `callsite` functions participate in the first delivery or get explicit unsupported diagnostics. Audit executable contract expressions under rigid types. Generic static proof abstraction remains outside this proposal and must coordinate with the callable-contract proposal. |
| Q9 | Future constraints and larger polymorphism | Decide separately whether to add type classes/traits, inferred constrained signatures, higher-kinded parameters, explicit higher-rank types, or annotated polymorphic recursion. Each requires its own scope and evidence; their absence must not block the unconstrained rank-one baseline once its gates are met. |

## Readiness and lifecycle

Keep this proposal in Design In Progress until Q1–Q8 have either a concrete
first-delivery decision or an explicit deferred boundary with an observable
rejection. Provide a reviewed constraint/alias model, source-surface prototype,
and JVM feasibility evidence before Ready promotion. In particular, demonstrate
that definition checking does not use concrete callers to repair rigid types,
and that indirect instances preserve runtime representations and effects.
Q9 extensions do not need to be implemented for baseline readiness.

During implementation, move supported behavior and checked acceptance examples
into the current source-surface, type, effect, and affected tooling
specifications. Keep only incomplete scope here. Remove this proposal and its
catalog entry when no active scope remains. Current specifications and root
feature summaries must not advertise generic functions merely because this
proposal exists.
