---
role: proposal
update-when: Module immutable-value declarations, initializer restrictions, import lookup, public visibility, or their planned verification change.
---

# Module Immutable Data Values

Provide named immutable data that a module can reuse and export without a
function call or an initialization effect. This proposal defines the source
declaration and its value identity. Qualified match refinement is a separate
dependent feature.

Current [source syntax](../specification/source-surface.md#module-immutable-value-declarations)
and [formatting](../specification/command-fmt.md#formatting-rules) accept typed
module `let` declarations. The compiler deliberately reports that these
declarations are not executable. This proposal retains the finite-data
semantics, name resolution, visibility, execution, and documentation work that
remains to make them usable values.

## Initialization Semantics

```veln
pub type State
	pub Idle
	pub Active(count: Int)
end

pub let initial: State = Idle
let example: State = Active(3)
pub let defaults: { state: State, attempts: Int } = {
	state: initial,
	attempts: 3,
}
```

The written annotation must be closed, although it may instantiate generic
types with closed arguments. The initializer must satisfy the rules below.

The initializer language is deliberately finite data construction:

| Initializer form | Admission |
| --- | --- |
| Integer, Boolean, string, and unit literals | Accepted; a negative integer literal is accepted with its ordinary range checks. |
| Parenthesized admitted initializer | Accepted. |
| Record literal | Accepted when every field initializer is admitted. |
| Visible nullary ADT constructor | Accepted as data. |
| Visible ADT constructor application | Accepted when every payload initializer is admitted and ordinary constructor type checking succeeds. The resolved declaration must be a constructor. |
| Bare or qualified module data reference | Accepted when it resolves to one eligible module value. |
| Function reference, function call, lambda, operator computation other than the negative-literal form, field access, indexing, control flow, effect operation, handler, or hole | Rejected, including calls to pure functions. |

The resulting value consists only of those literals, records, and finite ADT
constructor trees. Builtin `Option`, `Result`, and `List` constructors may
participate under the same rule. Opaque host values and function values are
not data initializers, including when nested in a record or payload. A type
with such a payload can still supply a different constructor whose actual
payload is admitted data. The initializer must be assignable to the written
annotation. The declaration exposes that annotation, rather than an inferred
singleton type that would reveal a private implementation choice.

Module data declarations are visible throughout their declaring module,
including before their source position. Their references form an acyclic
dependency graph across the selected package sources. A direct or indirect
cycle is rejected even when every declaration is unused. Every initializer
is checked even when unused. A failure does not publish that value or any
value whose initializer depends on it. Independent valid declarations retain
their semantic facts for diagnostics.

Every accepted reference yields the same immutable data. Source order, import
order, and first-use order do not change its value. There is no effectful
startup phase, user-code initializer execution, lazy failure, or mutable
storage associated with these declarations. Physical sharing is unspecified;
the stable identity used by language analysis is the declaration identity.

## Names, Identity, And Visibility

A module value belongs to the ordinary value namespace. Functions, public
function aliases, and module data declarations with the same name in one
module are duplicates. Type and schema namespaces remain separate. A public
function alias cannot target data; it retains its existing wrong-kind error.
This proposal adds no value-alias declaration syntax.

| Lookup situation | Result |
| --- | --- |
| Bare name with a visible local or parameter | Select the lexical binding. |
| Bare name without a lexical binding, with a same-module value declaration | Select that declaration before imported candidates, including when its initializer is invalid; an invalid declaration does not expose an imported fallback. |
| Bare name supplied by one public imported value declaration | Select that declaration under the existing written-import and implicit-prelude rules. |
| Bare name supplied by distinct imported value identities | Report ambiguity, including a function/data collision. |
| Qualified reference | Resolve the module using an exact written import path first, otherwise one unique implicit leaf qualifier; then select the value in that module. A local binding does not replace this qualified selection. |
| Colliding module qualifiers, missing member, or private member | Retain the existing name or visibility failure; do not fall back to a same-spelled value in another module. |
| Data value used as a call target | Report that the selected value is not callable; do not fall back to an imported function. |

Only imports whose contract admits bare member exposure contribute bare
imported candidates. The separate
[explicit import-alias proposal](explicit-import-aliases.md)
makes explicitly aliased imports qualified-only; this proposal does not depend
on that syntax. Every admitted alias route retains the target declaration's
identity.

Qualification selects the package and module before the member. Two packages
with the same module and member spellings have distinct identities. Different
valid written paths to the same declaration retain one identity. A value
initialized from another module value is a distinct declaration with equal
data; equality of initializers alone does not merge identities.

`let` is private and `pub let` is public. Other modules require an import and
an eligible public export. An exact test companion may access its private
target values through the same explicit, target-specific import permission as
private functions. That permission does not transfer to another module.

Public annotations must satisfy the existing recursive public type and
variant-refinement visibility rules. A public base-typed value may be
initialized privately with a hidden variant: the annotation exposes the base,
not the constructor. A public annotation naming that hidden refinement is
rejected. Public data documentation publishes the name and written annotation,
not the initializer. A rejected public value follows the existing package-
atomic documentation failure boundary; no partial package documentation is
published.

## Diagnostics

Existing syntax, casing, duplicate, unresolved-name, ambiguity, visibility,
constructor arity, and type-mismatch diagnostics continue to own those failed
facts. These two new failures concern only module initialization:

| Code | Primary failed fact | Related context |
| --- | --- | --- |
| `value.module_initializer_unsupported` | The smallest offending initializer expression is not admitted immutable data construction. | The declaration and permitted initializer categories. |
| `value.module_initializer_cycle` | A module-value reference closes a cyclic initializer dependency. | The participating declaration and reference locations. |

Cycle reporting is deterministic under source enumeration and import order
changes. At least one cycle diagnostic identifies every cyclic component;
the same component does not produce one diagnostic per importing consumer.
Invalid casing uses the current value-binding classification and quarantine
rules. Failures block execution and artifact emission through the existing
checked-core gate. Human primary messages describe the failed fact; provenance
and repair guidance are related notes.

## Acceptance Model

| Concern | Observable acceptance | Planned evidence |
| --- | --- | --- |
| Data construction | Literals, records, nullary/payload constructors, nested standard ADTs, and forward references produce their declared values. Computed/function/host-value initializers fail even when unused. | Semantic cases plus checked `veln check` and `veln run` examples; runtime results compared with independently written expected data. |
| Dependencies | Acyclic forward references are order independent. Self, cross-module, and longer cycles fail without publishing dependents. Unrelated valid declarations retain useful diagnostics. | Semantic dependency cases and command cases that reorder sources and imports. |
| Lookup | Local shadowing, function/data duplicates, imported ambiguity, exact-path precedence, colliding leaf qualifiers, same spellings in different packages, and non-callable data obey the table. | Name-resolution cases and multi-package command fixtures. |
| Visibility | Private data is hidden except to its exact companion. Public base annotations may hide initializer variants; public annotations may not expose private types/refinements. | Human/JSON visibility cases and package-documentation success/failure cases. |
| Stable declaration identity | Qualified data references are distinguished from constructors and functions; same declaration reached by different imports retains identity, equal data in distinct declarations does not merge. | Semantic identity tests and language-service definition cases for the new declaration kind. |

Use focused semantic tests and the checked-examples harness for the remaining
evidence. Update the name-resolution, type, execution, and package-documentation
specifications as those contracts become implemented. Remove this proposal
only after all remaining rows are implemented and checked.

## Related Work And Alternatives

The [Rust Reference, Constant items](https://doc.rust-lang.org/reference/items/constant-items.html)
specifies an explicitly typed module value, compile-time evaluation, and no
guaranteed common storage address across uses. This proposal adopts explicit
types and separates declaration identity from storage. It uses Veln's existing
`let` and lowercase value naming. It deliberately admits only data construction,
not Rust's broader constant evaluation, references, or destructors. That limit
provides reusable defaults without adding a compile-time interpreter or a
runtime initialization lifecycle.

Current Veln [local binding scope](../specification/name-resolution.md#value-calls-and-shadowing)
begins after the initializer. The proposed module rule differs intentionally:
declaration-wide lookup plus cycle rejection makes exported data independent
of source ordering. This rule does not change local lexical shadowing.

A zero-argument function remains the alternative for computed defaults. It
executes ordinary code and produces a fresh expression result; it does not
provide a stable module data binding for qualified match refinement. General
effectful module initialization and mutable global variables are outside this
proposal.
