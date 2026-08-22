---
role: proposal
update-when: Source-written identifier casing recovery, command reachability boundaries, or the foundation acceptance evidence changes.
---

# Recovery-Aware Source Identifier Casing

## Outcome

Implement casing validation for source-written type, constructor, function,
and value-binding declarations without changing the selection or reachability
boundary of any command.

This proposal is the required first slice of
[Identifier Casing](identifier-casing.md). A declaration-only validation slice
is not independently implementable. Analysis needs a surface graph before it
can compute selected-entry reachability, but an invalid declaration must not
enter that graph as a normal symbol. The first slice therefore includes the
minimum quarantined recovery representation needed to preserve the graph and
exclude invalid symbols from checked artifacts.

## Scope

The source positions and `name.invalid_case` diagnostic contract are the type,
constructor, function, and value-binding rows in
[Identifier Casing](identifier-casing.md#naming-contract). Module identities,
qualified-use casing, alias target leaves, rename, and source-less registries
remain outside this slice.

An invalid declaration or binding is retained only as a quarantined recovery
record. It does not enter normal lookup, checked core, typed intermediate
representation, package exports, or a backend. A compatible use may link to a
unique recovery record only to suppress a derivative diagnostic. This slice
does not expose recovery navigation or rename.

Validation preserves parse recovery for underscore-led names and retains the
complete written token span. A standalone `_` keeps its existing wildcard,
discard, or structural-error behavior.

## Command Boundary

The casing diagnostic follows each consumer's existing selected unit. It does
not become a project-wide or source-wide loading gate.

| Consumer state | Required result | Planned evidence |
| --- | --- | --- |
| `check` selects a source with an invalid covered name. | Report every selected casing diagnostic and produce no successful checked artifact. | Human and JSON checked cases with exact spans and details. |
| `run` reaches an invalid declaration or binding from the selected entry. | Report the casing diagnostic and produce no executable artifact. | Run case with a valid entry that reaches one invalid recovery record. |
| `run` does not reach an invalid declaration from the selected entry. | Run the valid reachable closure and omit the unreachable casing diagnostic. | Run case with a valid entry and an invalid unreachable peer in the same source. |
| `test` selects a test whose reachable closure contains an invalid name. | Preserve the existing selected-suite static gate and produce no backend artifact. | Selected and unselected test cases. |
| `doc` excludes a source or companion from its documentation set. | An invalid name in the excluded source does not block generated documentation. | Included and excluded documentation-source cases. |
| A language-service snapshot contains an invalid name outside the open-document operation's selected analysis unit. | Preserve the existing snapshot and overlay boundary; do not turn the diagnostic into a workspace-global gate. | Snapshot and overlay cases. |
| A dependency is not loaded for a selected consumer. | Its invalid names do not block that consumer. | Loaded and unloaded dependency cases. |

## Recovery Boundary

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| One compatible invalid declaration is referenced. | Keep one `name.invalid_case`; suppress only failures caused by absence of that declaration; emit no normal symbol or artifact for it. | Focused semantic and checked-artifact cases. |
| A valid declaration and an invalid recovery record share a spelling. | The valid declaration wins normal lookup. | Lookup decision-table test. |
| More than one compatible recovery record shares a spelling. | Select no recovery record and preserve independently provable ambiguity or unresolved facts. | Ambiguous recovery test. |
| A use crosses an import, public alias, companion, dependency, or implicit-prelude boundary. | Do not expose the recovery record across the boundary. | Boundary table with checked diagnostics and artifact assertions. |

The implementation may use any internal representation that satisfies these
outcomes. The quarantined representation is a required semantic boundary, not
a requirement for a particular data structure.

## Completion

This foundation is complete when all covered names have exact human and JSON
diagnostics, underscore recovery has no missing-name cascade, accepted names
retain their behavior, invalid symbols cannot enter a checked artifact, and
the command and recovery decision tables above have executable evidence.

Implementation must promote the completed behavior to the smallest matching
pages under `docs/specification/` and to checked cases under
`examples/specification/`. The complete identifier-casing proposal then moves
back to Ready with only its unimplemented remainder.
