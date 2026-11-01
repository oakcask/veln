---
role: proposal
update-when: Module immutable-value declaration identity, import selection, language-service definition, references, rename, protocol projection, or their planned verification changes.
---

# Module Immutable-Value Navigation

Complete language-service navigation for typed module `let` declarations. The
current [type specification](../specification/types.md#module-immutable-value-initializers)
owns initializer admission, annotation checking, visibility, and dependency
validation. The current
[name-resolution specification](../specification/name-resolution.md#namespaces-and-use-roles)
owns package-aware declaration identity and source lookup. The current
[execution specification](../specification/execution.md#runtime-readiness-and-host-boundaries)
owns runtime reads of admitted values.

This proposal now owns only definition, references, and rename for module
values. Navigation must select the declaration identity rather than an equal
initializer or a same-spelled function. Direct, qualified, aliased, and
package-distinct references must retain that identity.

Qualified match refinement remains a separate proposal. General effectful
module initialization and mutable global variables remain outside this
proposal.

## Acceptance Model

| Concern | Observable acceptance | Planned evidence |
| --- | --- | --- |
| Navigation | Definition, references, and rename retain declaration identity across valid import spellings. | Language-service and protocol cases. |

Remove this proposal after the navigation row is implemented and checked.

## Related Work And Design Choice

The Language Server Protocol defines separate
[`textDocument/definition`](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition),
[`textDocument/references`](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_references),
and
[`textDocument/rename`](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_rename)
requests around a source position. Those request and response shapes do not
define Veln declaration identity or import semantics. Veln therefore applies
one compiler-owned module-value identity across the three operations and
projects the resulting locations and edits through the existing protocol
shapes. Equal initializer data and equal source spelling are not alternative
identity rules because either would join distinct declarations.
