---
role: proposal
update-when: Module immutable-value declarations, imported declaration identity, qualified stable-value match refinement, or their planned evidence change.
---

# Qualified Immutable-Value Match Refinement

Extend stable-value match refinement to qualified references to module immutable
data. This proposal depends on
[Module Immutable Values](module-immutable-values.md). It becomes selectable
only after module-value execution is implemented. Imported lookup and
visibility are now implemented. Typed module `let` declarations still do not
produce runtime values.

## Contract

The current [type specification](../specification/types.md#inference-rules)
owns refinement of immutable local bindings, transparent aliases, and record
paths. This proposal adds the same behavior for references whose resolved
declaration is a module immutable data value.

| Expression or condition | Proposed observation |
| --- | --- |
| Qualified module data value matched against an admitted constructor | References to the same declaration in that arm have the intersected finite variant domain. |
| Two import paths or aliases resolve to the same value declaration | Both references share the arm's feasible domain; package and declaration identity determine equality. |
| Identical spelling resolves to values in different packages | Refinement of one does not narrow the other. |
| Record-field path rooted in a module data value | Apply the current immutable-root field-path rules to that declaration identity. |
| Local binding shadows the module value's bare name | Narrow only the resolved declaration; a qualified module reference continues to identify its module value. |
| Qualified constructor, function reference, call, indexing, or other computation | Repeated spelling does not establish stable-value identity. Existing constructor and function typing still applies independently. |
| Refined finite match domain, residual catch-all, impossible or redundant arm | Use the existing stable-value domain and diagnostic rules. |
| Control leaves an arm or the match expression | Do not publish an arm-local assumption into an unrelated control-flow path. |

Import aliases preserve declaration identity and visibility. They do not create
new data values. This extension neither consumes values nor proves protocol
liveness, reachability, or payload predicates.

## Acceptance And Planned Verification

| Concern | Required evidence |
| --- | --- |
| Actual module data | Checked source and runtime state-machine examples use the prerequisite declaration form and recover a refined value through a match. |
| Identity and visibility | Semantic and command cases cover direct imports, public aliases, distinct package identities, private values, and local shadowing. Explicit import aliases use the implemented source and name-resolution contracts and must preserve the selected declaration identity. |
| Stable field paths | Nested immutable record fields, transparent local aliases, widening, finite domains, and residual arms retain the existing rules. |
| Excluded expressions | Qualified constructors and function references/calls remain negative boundary evidence, rather than substitutes for module data. |
| Diagnostics and state | Human and JSON cases check impossible and redundant arms, deterministic ordering, and preservation of unrelated declaration types after failure. |

These cases are planned. At implementation, update the smallest matching type
and name-resolution specifications and checked examples before removing this
page and its catalog entry.

## Related Work And Design Choice

The current [type specification](../specification/types.md#module-immutable-value-initializers)
defines finite-data initializer typing, and the current
[name-resolution specification](../specification/name-resolution.md#namespaces-and-use-roles)
defines same-module and imported lookup, package-aware declaration identity,
and visibility. Execution remains in the
[module immutable data completion proposal](module-immutable-values.md).
That remaining boundary explains why a same-module declaration and a qualified
constructor are both insufficient evidence for this extension.

The existing [stable-value rules](../specification/types.md#inference-rules)
provide the semantic model to extend. This proposal chooses declaration
identity rather than source-text equality so import spelling cannot make a
computed expression stable or join unrelated package values.
