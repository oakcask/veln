---
role: implementation-record
authority: supporting
update-when: Source-written identifier casing recovery is superseded, its checked evidence changes, or current identifier-casing specification starts relying on this record as authority.
---

# Recovery-Aware Source Identifier Casing

This record preserves the completed proposal history. Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/commands.md`, and the checked cases named in those pages.

## Outcome

Implement the first reviewable casing foundation for source-written type,
constructor, function, and value-binding declarations. Preserve the existing
selection boundary of `check` and the existing selected-entry reachability
boundary of `run`.

This proposal is the required first slice of
[Identifier Casing](../../proposals/identifier-casing.md). A declaration-only validation slice
is not independently implementable. Analysis needs a surface graph before it
can compute selected-entry reachability, but an invalid declaration must not
enter that graph as a normal symbol. The first slice therefore includes the
minimum quarantined recovery representation needed to preserve the graph and
exclude invalid symbols from checked artifacts.

## Scope

The source positions and `name.invalid_case` diagnostic contract are the type,
constructor, function, and value-binding rows in
[Identifier Casing](../../proposals/identifier-casing.md#naming-contract). Module identities,
qualified-use casing, alias target leaves, rename, and source-less registries
remain outside this slice.

The function row includes function declarations, test declaration names, and
public function alias declaration names. The type row includes source ADT type
declarations and public type alias declaration names. This slice validates
those declarations when `check` or `run` selects them. It does not add the
dedicated `test` command selection evidence that the later boundary proposal
requires.

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
| `run` reaches an invalid declaration or binding from the selected entry. | Report the casing diagnostic and produce no executable artifact. | `identifier-casing-reachable-invalid`, `identifier-casing-entry-binding`, and `identifier-casing-signature-reachable`. |
| `run` does not reach an invalid declaration from the selected entry. | Run the valid reachable closure and omit the unreachable casing diagnostic. | `identifier-casing-unreachable-peer`. |

## Recovery Boundary

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| One compatible invalid declaration is referenced. | Keep one `name.invalid_case`; suppress only failures caused by absence of that declaration; emit no normal symbol or artifact for it. | Focused semantic and checked-artifact cases. |
| A valid declaration and an invalid recovery record share a spelling. | The valid declaration wins normal lookup. | Lookup decision-table test. |
| More than one compatible recovery record shares a spelling. | Select no recovery record and preserve independently provable ambiguity or unresolved facts. | `identifier-casing-recovery-ambiguous`. |
| A use crosses an import or public alias boundary. | Do not expose the recovery record across the boundary. | `identifier-casing-import-boundary` and `identifier-casing-public-alias-boundary`. |

The implementation may use any internal representation that satisfies these
outcomes. The quarantined representation is a required semantic boundary, not
a requirement for a particular data structure.

## Deferred Selection Boundaries

[Identifier Casing Selection Boundaries](../../proposals/identifier-casing-selection-boundaries.md)
owns the later `test`, `doc`, language-service snapshot and overlay, loaded and
unloaded dependency, companion, dependency, and implicit-prelude evidence. It
depends on this foundation and is not part of this slice's completion rule.

## Completion

This slice is complete when all covered declaration and binding names have
exact human and JSON diagnostics, underscore recovery has no missing-name
cascade, accepted names retain their behavior, invalid symbols cannot enter a
checked artifact, and every row in the `check`/`run` command table and the
same-source/import/public-alias recovery table above has executable evidence.

Implementation must promote the completed behavior to the smallest matching
pages under `docs/specification/` and to checked cases under
`examples/specification/`. Completion moves this proposal to implemented
history and unblocks the selection-boundary proposal. It does not complete the
remaining identifier-casing work.
