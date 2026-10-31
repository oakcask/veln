---
role: proposal
update-when: Module immutable-value execution, language-service navigation, or their planned verification changes.
---

# Module Immutable Data Value Completion

Complete the remaining compiler and tooling support for typed module `let`
declarations. The current specifications own the implemented syntax,
formatting, finite-data initializer checking, annotation assignability,
same-module and imported lookup, package-aware declaration identity,
same-module and cross-module dependency validation, visibility, initializer
diagnostics, and package-documentation publication.

## Scope

The current [type specification](../specification/types.md#module-immutable-value-initializers)
defines initializer admission, annotation checking, the package-aware
declaration identity and dependency graph formed by selected same-module and
imported references across package sources, and same-module and cross-module
cycle rejection. The current
[name-resolution specification](../specification/name-resolution.md#namespaces-and-use-roles)
defines package-aware declaration identity, same-module and imported lookup,
forward lookup, local shadowing, visibility, and the shared
function/module-value namespace. The current
[execution specification](../specification/execution.md#runtime-readiness-and-host-boundaries)
defines checked-core and typed-IR preservation and the remaining runtime gate.
The compiler deliberately retains `module.value_not_executable` after those
checks.

This proposal now owns only:

- immutable runtime value construction and backend code generation; and
- language-service definition, references, and rename for module values.

Qualified match refinement remains a separate dependent proposal. General
effectful module initialization and mutable global variables remain outside
this proposal.

## Remaining Execution And Navigation

Every accepted reference must produce the same immutable finite data. Source,
import, and first-use order must not change the value. There is no effectful
startup phase, user-code initializer execution, lazy failure, or mutable
storage. Physical sharing is unspecified; declaration identity remains distinct
from storage identity and from equality of initializer data.

Backend lowering must materialize admitted literal, record, and ADT constructor
trees without invoking user code. The existing
`module.value_not_executable` gate remains until this complete route is present.

Language-service navigation must select the declaration identity rather than an
equal initializer or a same-spelled function.

## Acceptance Model

| Concern | Observable acceptance | Planned evidence |
| --- | --- | --- |
| Execution | Admitted literals, records, constructors, and references produce their declared immutable data without initializer effects. | Checked `veln run` examples with independently written expected values. |
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
