---
role: proposal
update-when: Module immutable-value imported lookup, cross-module dependencies, visibility, execution, documentation publication, language-service navigation, or their planned verification change.
---

# Module Immutable Data Value Completion

Complete the remaining compiler and tooling support for typed module `let`
declarations. The current specifications own the implemented syntax,
formatting, finite-data initializer checking, annotation assignability,
same-module lookup and dependency validation, and initializer diagnostics.

## Scope

The current [type specification](../specification/types.md#module-immutable-value-initializers)
defines initializer admission, annotation checking, dependency graphs, and
cycle rejection. The current
[name-resolution specification](../specification/name-resolution.md#namespaces-and-use-roles)
defines same-module declaration identity, forward lookup, local shadowing, and
the shared function/module-value namespace. The current compiler deliberately
retains `module.value_not_executable` after those checks.

This proposal now owns only:

- imported bare and qualified lookup across package sources;
- rejection of public function aliases that target module values;
- imported and cross-module dependency validation;
- public and private visibility, including exact companion access;
- checked-core lowering and immutable runtime value construction;
- package-documentation publication of public names and annotations; and
- language-service definition, references, and rename for module values.

Qualified match refinement remains a separate dependent proposal. General
effectful module initialization and mutable global variables remain outside
this proposal.

## Remaining Lookup And Visibility

An imported module value must use the existing written-import rules. Exact
written paths take precedence over unique implicit leaf qualifiers. Distinct
imported value identities with the same bare spelling are ambiguous, including
a function/data collision. A same-module declaration must take precedence even
when that declaration is invalid; invalid local data must not expose an
imported fallback. A selected data value used as a call target must not fall
back to an imported function.

A public function alias cannot target a module value. Such a target must report
`name.kind_mismatch` rather than an unresolved-name diagnostic.

`let` is private and `pub let` is public. Another module requires an import and
an eligible public export. An exact test companion may access its private target
values under the same target-specific permission as private functions. Public
annotations must satisfy the recursive public type and variant-refinement
visibility rules. A public base annotation may hide a private initializer
variant, but a public annotation may not expose that private refinement.

## Remaining Imported Dependencies

Dependencies must connect selected module-value declarations across all
package sources. Every cross-module cyclic component must fail without
publishing its declarations or any dependent value. Independent valid
declarations must remain available for semantic diagnostics. The selected
closing reference and related declaration and dependency locations must remain
the same when source enumeration or import order changes.

## Remaining Execution And Publication

Every accepted reference must produce the same immutable finite data. Source,
import, and first-use order must not change the value. There is no effectful
startup phase, user-code initializer execution, lazy failure, or mutable
storage. Physical sharing is unspecified; declaration identity remains distinct
from storage identity and from equality of initializer data.

Checked-core and typed IR must preserve module-value declaration identity and
references. Runtime lowering must materialize admitted literal, record, and ADT
constructor trees without invoking user code. The existing
`module.value_not_executable` gate remains until this complete route is present.

Package documentation must publish each eligible public name and written
annotation without publishing its initializer. A rejected public value follows
the package-atomic documentation failure boundary. Language-service navigation
must select the declaration identity rather than an equal initializer or a
same-spelled function.

## Acceptance Model

| Concern | Observable acceptance | Planned evidence |
| --- | --- | --- |
| Imported lookup | Bare and qualified imports obey exact-path precedence, unique implicit qualifiers, ambiguity, and no-fallback rules. | Multi-module and multi-package semantic and checked command cases. |
| Function aliases | A public function alias that targets a module value reports `name.kind_mismatch`. | Focused semantic coverage and a checked diagnostic case. |
| Imported dependencies | Cross-module cycles and their dependents are quarantined while unrelated facts remain available; source and import reordering does not change cycle selection. | Multi-module semantic and checked command cases with reordered sources and imports. |
| Visibility | Private data is hidden except from its exact companion; public annotations cannot expose private types or refinements. | Human and JSON visibility cases. |
| Execution | Admitted literals, records, constructors, and references produce their declared immutable data without initializer effects. | Checked `veln run` examples with independently written expected values. |
| Publication | Public names and annotations are documented atomically without initializer text. | Package-documentation success and failure cases. |
| Navigation | Definition, references, and rename retain declaration identity across valid import spellings. | Language-service and protocol cases. |

Remove this proposal only after every remaining row is implemented and checked.

## Related Work And Alternatives

The [Rust Reference, Constant items](https://doc.rust-lang.org/reference/items/constant-items.html)
specifies an explicitly typed module value, compile-time evaluation, and no
guaranteed common storage address across uses. Veln likewise separates
declaration identity from storage, but retains its narrower finite-data
initializer language instead of adopting a general compile-time interpreter.

A zero-argument function remains the alternative for computed defaults. It
executes ordinary code and produces an expression result; it does not provide a
stable module data declaration. General effectful module initialization and
mutable global variables remain outside this proposal.
