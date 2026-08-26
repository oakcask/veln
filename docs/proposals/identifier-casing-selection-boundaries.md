---
role: proposal
update-when: Identifier casing recovery isolation across the implicit prelude changes.
---

# Identifier Casing Selection Boundaries

## Outcome

Extend source identifier casing quarantined recovery across the implicit
prelude boundary.

This proposal depends on
[Recovery-Aware Source Identifier Casing](../reference/implemented-proposals/identifier-casing-source-recovery.md).
That foundation is implemented and specified as current behavior.

## Scope

This proposal retains only the remaining recovery quarantine evidence for the
implicit prelude.

This proposal does not add module-identity casing, qualified-use casing,
alias-target leaf casing, rename behavior, or source-less registry validation.
Those capabilities remain in
[Identifier Casing](identifier-casing.md).

## Command And Selection Boundary

The remaining proposal scope has no command or source-selection boundary row.
Implemented selection boundaries are current behavior under
`docs/specification/`.

## Recovery Boundary

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| A use crosses the implicit-prelude boundary. | A valid prelude symbol may win normal lookup; an invalid recovery record cannot enter or escape the prelude namespace. | Valid-prelude precedence and invalid-recovery isolation cases. |

## Completion

This proposal is complete when the implicit-prelude recovery row has
executable evidence. The evidence must show valid-prelude precedence and that
invalid recovery records do not enter or escape the prelude namespace.

Implementation must promote completed behavior to the smallest matching pages
under `docs/specification/` and to checked cases under
`examples/specification/`. Completion moves this proposal to implemented
history without claiming completion of module, qualified-name, rename, or
source-less registry casing.
