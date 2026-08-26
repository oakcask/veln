---
role: proposal
update-when: Identifier casing direct-dependency selection, selected-entry reachability, recovery isolation, or implicit-prelude evidence changes.
---

# Identifier Casing Selection Boundaries

## Outcome

Extend source identifier casing diagnostics and quarantined recovery across
direct-dependency and implicit-prelude boundaries.

This proposal depends on
[Recovery-Aware Source Identifier Casing](../reference/implemented-proposals/identifier-casing-source-recovery.md).
That foundation is implemented and specified as current behavior.

## Scope

The direct-dependency source-selection, selected-entry reachability, and
recovery-quarantine rows are executable current behavior. The checked
`identifier-casing-loaded-dependency-json`,
`identifier-casing-loaded-unreachable-dependency-json`, and
`identifier-casing-unloaded-dependency-json` run cases cover the selected
consumer boundary. Focused analysis tests cover dependency-to-consumer and
consumer-to-dependency recovery isolation.

The Ready slice now retains implicit-prelude recovery isolation. The `test`,
`doc`, companion recovery, direct-dependency, and language-service snapshot and
overlay boundaries already have executable evidence and are not planned work
here.

This proposal does not add module-identity casing, qualified-use casing,
alias-target leaf casing, rename behavior, or source-less registry validation.
Those capabilities remain in
[Identifier Casing](identifier-casing.md).

## Implemented Direct-Dependency Slice

### Command And Selection Boundary

| Dependency state for `run` | Invalid declaration state | Required result | Evidence |
| --- | --- | --- | --- |
| The selected consumer imports the dependency module. | The selected entry reaches the invalid covered declaration through the dependency. | Report `name.invalid_case`, keep the invalid declaration out of lookup, and produce no backend artifact. | Loaded-and-reachable dependency JSON case with the diagnostic and static-gate outcome. |
| The selected consumer imports the dependency module. | The invalid covered declaration is outside the selected-entry reachable closure. | Run the valid reachable closure without reporting the unreachable casing diagnostic. | Loaded-but-unreachable dependency JSON case with successful consumer output and no dependency diagnostic. |
| The manifest declares the dependency, but the selected consumer does not import its module. | The dependency contains an invalid covered declaration. | Run the consumer without reporting the unselected dependency diagnostic. | Manifest-only unloaded dependency JSON case with successful consumer output and no dependency diagnostic. |

The first two rows preserve the current `run` selected-entry reachability
contract. The second and third rows are separate requirements. Loading a
dependency module is not evidence that every declaration in that module is
reachable.

### Recovery Boundary

| Recovery source | Cross-package use | Required result | Evidence |
| --- | --- | --- | --- |
| Dependency | Consumer | The dependency recovery record does not resolve the consumer use. | Focused analysis case that requires the ordinary unresolved diagnostic and no reachable invalid dependency record. |
| Consumer | Dependency | The consumer recovery record does not resolve the dependency use. | Focused analysis case that requires the ordinary unresolved diagnostic and no reachable invalid consumer record. |

## Ready Implicit-Prelude Slice

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| A use crosses the implicit-prelude boundary. | A valid prelude symbol may win normal lookup; an invalid recovery record cannot enter or escape the prelude namespace. | Valid-prelude precedence and invalid-recovery isolation cases. |

## Completion

This proposal is complete when the implicit-prelude slice has executable
evidence. Completion moves this proposal to implemented history without
claiming completion of module, qualified-name, language-service navigation,
rename, or source-less registry casing.
