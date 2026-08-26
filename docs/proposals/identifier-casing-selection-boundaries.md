---
role: proposal
update-when: Identifier casing direct-dependency selection, selected-entry reachability, recovery isolation, or implicit-prelude evidence changes.
---

# Identifier Casing Selection Boundaries

## Outcome

Extend source identifier casing diagnostics and quarantined recovery across
the implicit-prelude boundary.

This proposal depends on
[Recovery-Aware Source Identifier Casing](../reference/implemented-proposals/identifier-casing-source-recovery.md).
That foundation is implemented and specified as current behavior.

## Scope

The direct-dependency source-selection, selected-entry reachability, and
recovery-quarantine rows are executable current behavior. The checked
`identifier-casing-loaded-dependency-json` and
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

## Ready Implicit-Prelude Slice

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| A use crosses the implicit-prelude boundary. | A valid prelude symbol may win normal lookup; an invalid recovery record cannot enter or escape the prelude namespace. | Valid-prelude precedence and invalid-recovery isolation cases. |

## Completion

This proposal is complete when the implicit-prelude slice has executable
evidence. Completion moves this proposal to implemented history without
claiming completion of module, qualified-name, language-service navigation,
rename, or source-less registry casing.
