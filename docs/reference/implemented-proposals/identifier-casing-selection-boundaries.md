---
role: implementation-record
authority: supporting
update-when: Identifier casing selection-boundary evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Selection Boundaries

## Outcome

Source identifier casing diagnostics and quarantined recovery now preserve
normal lookup and recovery isolation across direct-dependency and implicit
standard-prelude boundaries.

This record depends on
[Recovery-Aware Source Identifier Casing](identifier-casing-source-recovery.md).
That foundation is implemented and specified as current behavior.

## Scope

The direct-dependency source-selection, selected-entry reachability, and
recovery-quarantine rows are executable current behavior. The checked
`identifier-casing-loaded-dependency-json`,
`identifier-casing-loaded-unreachable-dependency-json`, and
`identifier-casing-unloaded-dependency-json` run cases cover the selected
consumer boundary. Focused analysis tests cover dependency-to-consumer and
consumer-to-dependency recovery isolation.

The implicit-prelude recovery-isolation slice is complete. The `test`, `doc`,
companion recovery, direct-dependency, and language-service snapshot and
overlay boundaries also have executable evidence and are not planned work here.

This proposal does not add module-identity casing or MCP rename mapping.
Public alias target-leaf casing is completed separately in
[Identifier Casing Public Alias Targets](identifier-casing-public-alias-targets.md).
Source-less registry validation is completed separately in
[Identifier Casing Source-Less Symbols](identifier-casing-source-less-symbols.md).
Qualified-use path casing is completed separately in
[Identifier Casing Qualified Use Paths](identifier-casing-qualified-use-paths.md).
LSP rename conflict rejection for valid selected workspace symbols is
completed separately in
[Identifier Casing Rename Conflicts](identifier-casing-rename-conflicts.md).
The other capabilities remain in
[Identifier Casing](../../proposals/identifier-casing.md).

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

## Implemented Implicit-Prelude Slice

| Source state | Required result | Evidence |
| --- | --- | --- |
| A function or constructor use crosses the implicit-prelude boundary. | A valid prelude symbol may win normal lookup; an invalid recovery record cannot enter or escape the prelude namespace. | Focused semantic tests cover application-to-prelude and prelude-to-application function and constructor recovery isolation, ordinary unresolved diagnostics after isolation, and valid-prelude function and constructor precedence. The checked `identifier-casing-implicit-prelude-boundary-json` case covers the user-visible valid-prelude function precedence boundary with a return-type mismatch that only follows from selecting the prelude signature. The checked `identifier-casing-implicit-prelude-isolation-json` case covers the user-visible qualified-prelude unresolved boundary. |

## Completion

This proposal is complete. Current behavior lives in
[Names And Effects](../../specification/names-effects.md) and checked cases
under `examples/specification/`. Completion does not claim completion of
module-identity casing or MCP rename mapping. Recovery navigation and source
declaration or binding recovery rename are completed separately in
[Identifier Casing Recovery Navigation And Rename](identifier-casing-recovery-navigation.md).
