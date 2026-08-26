---
role: proposal
update-when: Identifier casing selection boundaries for test, doc, language services, dependencies, companions, or the implicit prelude change.
---

# Identifier Casing Selection Boundaries

## Outcome

Extend source identifier casing diagnostics and quarantined recovery across
the command and source-selection boundaries that are not part of the initial
`check` and `run` foundation.

This proposal depends on
[Recovery-Aware Source Identifier Casing](../reference/implemented-proposals/identifier-casing-source-recovery.md).
That foundation is implemented and specified as current behavior.

## Scope

This proposal retains the remaining selection evidence for loaded or unloaded
dependencies and the remaining recovery quarantine evidence for companions,
dependencies, and the implicit prelude. The `test`, `doc`, and
language-service snapshot and overlay rows have executable evidence.

This proposal does not add module-identity casing, qualified-use casing,
alias-target leaf casing, rename behavior, or source-less registry validation.
Those capabilities remain in
[Identifier Casing](identifier-casing.md).

## Command And Selection Boundary

| Consumer state | Required result | Evidence |
| --- | --- | --- |
| `test` selects a test whose reachable closure contains an invalid covered name. | Preserve the selected-suite static gate and produce no backend artifact. | Implemented by `examples/specification/test/identifier-casing-selected-static-gate-json/` and `examples/specification/test/identifier-casing-unselected-peer-json/`. |
| `doc` includes a source with an invalid covered name. | Reject the selected documentation set before publishing generated documentation. | Implemented by `examples/specification/doc/identifier-casing-included-source/`. |
| `doc` excludes a source or companion with an invalid covered name. | Generate documentation without reporting the excluded casing diagnostic. | Implemented by `examples/specification/doc/identifier-casing-excluded-source/` and `examples/specification/doc/identifier-casing-excluded-companion/`. |
| A language-service snapshot or open-document overlay contains an invalid covered name inside the selected analysis unit. | Publish the casing diagnostic for that selected unit and keep invalid symbols out of snapshot indexes. | Implemented by `examples/specification/lsp/identifier-casing-snapshot-boundary/`, `examples/specification/lsp/identifier-casing-overlay-boundary/`, and `examples/specification/lsp/identifier-casing-handler-binding-navigation/`. |
| An invalid covered name exists outside the language-service operation's selected analysis unit. | Preserve the operation's snapshot and overlay boundary; do not report a workspace-global casing diagnostic. | Implemented for workspace snapshot and overlay selection by `examples/specification/lsp/identifier-casing-snapshot-boundary/` and `examples/specification/lsp/identifier-casing-overlay-boundary/`; dependency selection remains open. |
| A dependency containing an invalid covered name is loaded for a selected consumer. | Report the selected diagnostic and prevent the invalid symbol from entering lookup or an artifact. | Language-service retained dependency definition exclusion is covered by `veln-language-service` tests. Loaded dependency diagnostic and artifact evidence remains open. |
| A dependency containing an invalid covered name is not loaded for a selected consumer. | Do not report its casing diagnostic or block the consumer. | Unloaded dependency case. |

## Recovery Boundary

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| A use crosses a companion boundary. | Do not expose the recovery record to the companion or its target source. | Companion diagnostics and artifact assertions. |
| A use crosses a dependency boundary. | Do not expose the recovery record across the package boundary. | Loaded dependency diagnostics, lookup, and artifact assertions. |
| A use crosses the implicit-prelude boundary. | A valid prelude symbol may win normal lookup; an invalid recovery record cannot enter or escape the prelude namespace. | Valid-prelude precedence and invalid-recovery isolation cases. |

## Language-Service Acceptance Evidence

| Covered invalid source name | Selection points | Operations | Evidence |
| --- | --- | --- | --- |
| Function declaration in a saved snapshot. | Declaration and same-source call. | Definition, references, rename. | `examples/specification/lsp/identifier-casing-snapshot-boundary/` |
| Function declaration in an open-document overlay that replaces saved source. | Overlay declaration and same-source call. | Definition. | `examples/specification/lsp/identifier-casing-overlay-boundary/` |
| Handler context binding in a saved snapshot. | Binding declaration and in-scope clause-body use. | Definition, references, prepare-rename, rename. | `examples/specification/lsp/identifier-casing-handler-binding-navigation/` |
| Handler operation-clause binding in a saved snapshot. | Binding declaration and in-scope clause-body use. | Definition, references, prepare-rename, rename. | `examples/specification/lsp/identifier-casing-handler-binding-navigation/` |

## Completion

This proposal is complete when every command, selection, and recovery row
above has executable evidence. The evidence must show both diagnostic
selection and artifact exclusion where the row requires both outcomes.

Implementation must promote completed behavior to the smallest matching pages
under `docs/specification/` and to checked cases under
`examples/specification/`. Completion moves this proposal to implemented
history without claiming completion of module, qualified-name,
language-service navigation, rename, or source-less registry casing.
