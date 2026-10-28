---
role: proposal
update-when: Module immutable-value cross-module dependencies, execution, documentation publication, language-service navigation, or their planned verification change.
---

# Module Immutable Data Value Completion

Complete the remaining compiler and tooling support for typed module `let`
declarations. The current specifications own the implemented syntax,
formatting, finite-data initializer checking, annotation assignability,
same-module and imported lookup, package-aware declaration identity,
same-module dependency validation, visibility, and initializer diagnostics.

## Scope

The current [type specification](../specification/types.md#module-immutable-value-initializers)
defines initializer admission, annotation checking, dependency graphs, and
cycle rejection. The current
[name-resolution specification](../specification/name-resolution.md#namespaces-and-use-roles)
defines same-module declaration identity, forward lookup, local shadowing, and
the shared function/module-value namespace. The current compiler deliberately
retains `module.value_not_executable` after those checks.

This proposal now owns only:

- imported and cross-module dependency validation;
- checked-core lowering and immutable runtime value construction;
- package-documentation publication of public names and annotations; and
- language-service definition, references, and rename for module values.

Qualified match refinement remains a separate dependent proposal. General
effectful module initialization and mutable global variables remain outside
this proposal.

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
| Imported dependencies | Cross-module cycles and their dependents are quarantined while unrelated facts remain available; source and import reordering does not change cycle selection. | Multi-module semantic and checked command cases with reordered sources and imports. |
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
