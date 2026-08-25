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

The first reviewable casing foundation for source-written type, constructor,
function, and value-binding declarations was implemented. The implementation
preserved the existing selection boundary of `check` and the existing
selected-entry reachability boundary of `run`.

This proposal is the required first slice of
[Identifier Casing](../../proposals/identifier-casing.md). A declaration-only validation slice
is not independently implementable. Analysis needs a surface graph before it
can compute selected-entry reachability, but an invalid declaration must not
enter that graph as a normal symbol. The first slice therefore includes the
minimum quarantined recovery representation needed to preserve the graph and
exclude invalid symbols from checked artifacts.

## Scope

The completed source positions and `name.invalid_case` diagnostic contract are
specified by the current name and diagnostics specification pages. Module
identities, qualified-use casing, alias target leaves, rename, and source-less
registries remained outside this slice.

The completed function-class declaration contexts are function declarations,
test declaration names, and public function alias declaration names. The
completed type-class declaration contexts are source ADT type declarations and
public type alias declaration names. The completed constructor context is
source ADT constructor declarations. The completed value-binding contexts are
function parameters, result bindings, local pattern bindings selected by
pattern syntax, handler context parameters, handler operation-clause
parameters, and hole `satisfy` candidate bindings. Effect-operation
declaration parameters and qualified-use path segments were explicitly
excluded from this slice. The slice validates the covered declarations when
`check` or `run` selects them. It did not add the dedicated `test` command
selection evidence that the later boundary proposal requires.

An invalid declaration or binding is retained only as a quarantined recovery
record. It does not enter normal lookup, checked core, typed intermediate
representation, package exports, or a backend. A compatible use may link to a
unique recovery record only to suppress a derivative diagnostic. This slice
does not expose recovery navigation or rename.

Validation preserves parse recovery for underscore-led names in covered
contexts and retains the complete written token span. A standalone `_` keeps
its existing wildcard, discard, or structural-error behavior.

## Command Boundary

The casing diagnostic follows each consumer's existing selected unit. It does
not become a project-wide or source-wide loading gate.

| Consumer state | Required result | Completed evidence |
| --- | --- | --- |
| `check` selects a source with an invalid covered name. | Report every selected casing diagnostic and produce no successful checked artifact. | Human and JSON checked cases with exact spans and details, including `identifier-casing-binding-origins`, `identifier-casing-test-declaration`, and `identifier-casing-parse-recovery-independent`. |
| `run` reaches an invalid declaration or binding from the selected entry. | Report the casing diagnostic and produce no executable artifact. | `identifier-casing-reachable-invalid`, `identifier-casing-entry-binding`, `identifier-casing-reachable-handler-binding`, and `identifier-casing-signature-reachable`. |
| `run` does not reach an invalid declaration from the selected entry. | Run the valid reachable closure and omit the unreachable casing diagnostic. | `identifier-casing-unreachable-peer`. |

## Recovery Boundary

| Source state | Required result | Completed evidence |
| --- | --- | --- |
| One compatible invalid declaration is referenced. | Keep one `name.invalid_case`; suppress only failures caused by absence of that declaration; emit no normal symbol or artifact for it. | Focused semantic and checked-artifact cases, including `identifier-casing-invalid-type-constructor-recovery`, plus direct artifact tests for surface exports, checked core, typed IR, and backend input. |
| A valid declaration and an invalid recovery record share a spelling. | The valid declaration wins normal lookup. | Lookup decision-table test. |
| More than one compatible recovery record shares a spelling. | Select no recovery record and preserve independently provable ambiguity or unresolved facts. | `identifier-casing-recovery-ambiguous`. |
| A use crosses an import or public alias boundary. | Do not expose the recovery record across the boundary. | `identifier-casing-import-boundary` and `identifier-casing-public-alias-boundary`. |

Same-spelled invalid declarations still participate in duplicate checking when
the duplicate fact is independently provable from the original spelling. The
checked `identifier-casing-invalid-duplicates` case fixes this overlap for
functions, types, and constructors.

The implementation may use any internal representation that satisfies these
outcomes. The quarantined representation is a required semantic boundary, not
a requirement for a particular data structure.

## Deferred Selection Boundaries

[Identifier Casing Selection Boundaries](../../proposals/identifier-casing-selection-boundaries.md)
owns the later `test`, `doc`, language-service snapshot and overlay, loaded and
unloaded dependency, companion, dependency, and implicit-prelude evidence. It
depends on this foundation and is not part of this slice's completion rule.

## Completion

This slice is complete. Covered declaration and binding names have exact human
and JSON diagnostics, underscore recovery has no missing-name cascade,
accepted names retain their behavior, invalid symbols cannot enter a checked
artifact, and every row in the `check`/`run` command table and the
same-source/import/public-alias recovery table above has executable evidence.
The checked `identifier-casing-underscore-aliases` and
`identifier-casing-value-recovery-scope` cases fix the public-alias recovery
and lexical-scope recovery boundaries of this slice. The checked
`identifier-casing-invalid-type-constructor-recovery`,
`identifier-casing-invalid-duplicates`, and
`identifier-casing-test-declaration` cases fix the reviewed recovery and
diagnostic-overlap gaps in this slice.

The completed behavior is promoted to the smallest matching pages under
`docs/specification/` and to checked cases under `examples/specification/`.
This implemented history unblocks the selection-boundary proposal. It does not
complete the remaining identifier-casing work.
