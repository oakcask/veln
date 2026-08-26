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

The Ready slice retains direct-dependency source-selection, selected-entry
reachability, and recovery-quarantine evidence. Source selection determines
whether a dependency module participates in command analysis. Selected-entry
reachability determines which declarations in a loaded module can block
`run`.

Implicit-prelude recovery isolation remains in this proposal, but it is
blocked until the direct-dependency slice is executable current behavior. The
`test`, `doc`, companion recovery, and language-service snapshot and overlay
boundaries already have executable evidence and are not planned work here.

This proposal does not add module-identity casing, qualified-use casing,
alias-target leaf casing, rename behavior, or source-less registry validation.
Those capabilities remain in
[Identifier Casing](identifier-casing.md).

## Ready Direct-Dependency Slice

### Command And Selection Boundary

| Dependency state for `run` | Invalid declaration state | Required result | Planned evidence |
| --- | --- | --- | --- |
| The selected consumer imports the dependency module. | The selected entry reaches the invalid covered declaration through the dependency. | Report `name.invalid_case`, keep the invalid declaration out of lookup, and produce no backend artifact. | Loaded-and-reachable dependency JSON case with the diagnostic and static-gate outcome. |
| The selected consumer imports the dependency module. | The invalid covered declaration is outside the selected-entry reachable closure. | Run the valid reachable closure without reporting the unreachable casing diagnostic. | Loaded-but-unreachable dependency JSON case with successful consumer output and no dependency diagnostic. |
| The manifest declares the dependency, but the selected consumer does not import its module. | The dependency contains an invalid covered declaration. | Run the consumer without reporting the unselected dependency diagnostic. | Manifest-only unloaded dependency JSON case with successful consumer output and no dependency diagnostic. |

The first two rows preserve the current `run` selected-entry reachability
contract. The second and third rows are separate requirements. Loading a
dependency module is not evidence that every declaration in that module is
reachable.

### Recovery Boundary

| Recovery source | Cross-package use | Required result | Planned evidence |
| --- | --- | --- | --- |
| Dependency | Consumer | The dependency recovery record does not resolve the consumer use. | Focused analysis case that requires the ordinary unresolved diagnostic and no reachable invalid dependency record. |
| Consumer | Dependency | The consumer recovery record does not resolve the dependency use. | Focused analysis case that requires the ordinary unresolved diagnostic and no reachable invalid consumer record. |

## Blocked Implicit-Prelude Slice

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| A use crosses the implicit-prelude boundary. | A valid prelude symbol may win normal lookup; an invalid recovery record cannot enter or escape the prelude namespace. | Valid-prelude precedence and invalid-recovery isolation cases. |

## Completion

The direct-dependency slice is complete when every row in its command and
recovery tables has executable evidence. The loaded-and-reachable case must
show the diagnostic and absence of a backend artifact. The two non-blocking
cases must each show successful consumer output and absence of the dependency
diagnostic.

After the direct-dependency behavior is promoted to the current specification,
the implicit-prelude slice can move to Ready. This proposal is complete when
that slice also has executable evidence.

Implementation must promote completed behavior to the smallest matching pages
under `docs/specification/` and to checked cases under
`examples/specification/`. Completion moves this proposal to implemented
history without claiming completion of module, qualified-name,
language-service navigation, rename, or source-less registry casing.
