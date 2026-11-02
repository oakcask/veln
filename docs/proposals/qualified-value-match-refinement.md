---
role: proposal
update-when: Module-value-rooted field-path match refinement, qualified module-value transparent aliases, or their planned evidence changes.
---

# Module-Value Field And Alias Match Refinement

Extend the implemented qualified module immutable-value stable root through
record fields and local transparent aliases. Qualified direct paths and import
aliases already refine by package-aware declaration identity.

## Contract

The current [type specification](../specification/types.md#inference-rules)
owns immutable local field paths, local transparent aliases, and qualified
module-value roots. This proposal adds only the compositions between those
implemented behaviors.

| Expression or condition | Proposed observation |
| --- | --- |
| Record-field path rooted in a module data value | Apply the current immutable-root field-path rules to that declaration identity. |
| Local binding initialized only from a qualified module data value | Treat the local and the selected declaration as one transparent alias group, so matching either member narrows both for the current arm. |
| Nested local aliases or immutable record fields composed from that root | Preserve the existing transitive alias, field-path, widening, finite-domain, and restoration rules. |
| Qualified constructor, function reference, call, indexing, or other computation | Do not create a module-value alias or field root. |

Import aliases continue to preserve declaration identity and visibility rather
than creating data values. The extension neither consumes values nor proves
protocol liveness, reachability, or payload predicates.

## Acceptance And Planned Verification

| Concern | Required evidence |
| --- | --- |
| Stable field paths | Semantic and command cases match nested immutable record fields rooted in direct and aliased module-value paths, including residual arms and arm restoration. |
| Transparent aliases | Matching either a qualified module value or a local initialized only from it narrows the other; transitive locals retain the same declaration identity. |
| Identity and boundaries | Distinct package declarations remain independent. Qualified constructors, functions, calls, and computed initializers do not become aliases or field roots. |
| Diagnostics and state | Human and JSON cases check impossible and redundant arms and preservation of unrelated declaration and local types after failure. |

These cases are planned. At implementation, update the type specification and
checked examples before removing this page and its catalog entry.

## Related Work And Design Choice

The current [type specification](../specification/types.md#inference-rules)
defines all three prerequisite relations: qualified module-value declaration
identity, immutable local field paths, and transparent local alias groups. The
remaining design composes those relations instead of introducing source-text
equality. This keeps import spelling from joining unrelated package values and
keeps computed expressions outside stable-value refinement.
