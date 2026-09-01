---
role: implementation-record
authority: supporting
update-when: Recovery-aware source identifier casing evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Recovery-Aware Source Identifier Casing

## Completion Evidence

Current behavior is specified in
[Names And Effects](../../specification/names-effects.md). The executable
`identifier-casing-source-recovery-json`,
`identifier-casing-source-recovery-human`,
`identifier-casing-binding-positions-json`,
`identifier-casing-binding-positions-human`,
`identifier-casing-underscore-recovery-human`,
`identifier-casing-underscore-recovery-json`,
`identifier-casing-reachable-recovery`,
`identifier-casing-reachable-recovery-json`,
`identifier-casing-constructor-call-recovery`,
`identifier-casing-constructor-call-recovery-json`,
`identifier-casing-reachable-invalid-alias`,
`identifier-casing-reachable-invalid-alias-json`,
`identifier-casing-reachable-expression-type`,
`identifier-casing-reachable-expression-type-json`,
`identifier-casing-reachable-type-alias`,
`identifier-casing-reachable-type-alias-json`,
`identifier-casing-unreachable-peer`,
`identifier-casing-owned-constructor-recovery-human`,
`identifier-casing-owned-constructor-recovery-json`,
`identifier-casing-owned-nullary-constructor-recovery`,
`identifier-casing-owned-nullary-constructor-recovery-json`,
`identifier-casing-owned-payload-constructor-recovery`,
`identifier-casing-owned-payload-constructor-recovery-json`,
`identifier-casing-owned-constructor-unreachable`,
`identifier-casing-constructor-sibling-unreachable`,
`identifier-casing-constructor-sibling-unreachable-json`,
`identifier-casing-record-field-reachability`,
the checked `identifier-casing-function-value-recovery-human` and
`identifier-casing-function-value-recovery-json` cases, and the run
`identifier-casing-function-value-recovery` and
`identifier-casing-function-value-recovery-json` cases cover exact diagnostics,
parser recovery, checked-artifact blocking, and the `check`/`run` selection
boundary.
The `identifier-casing-reachable-handler-bindings` and
`identifier-casing-reachable-handler-bindings-json`,
`identifier-casing-reachable-handler-annotation`,
`identifier-casing-reachable-handler-annotation-json`,
`identifier-casing-reachable-handler-clauses`, and
`identifier-casing-reachable-handler-clauses-json` cases cover reachable
handler binding, annotation, and clause-expression diagnostics under `run`.
The checked `identifier-casing-import-recovery-isolation-json` and
`identifier-casing-public-alias-recovery-isolation-json`,
`identifier-casing-accepted-names-json`,
`identifier-casing-valid-symbol-precedence-json`,
`identifier-casing-handler-binding-quarantine-json`,
`identifier-casing-cross-class-ambiguous-recovery-json`, and
`identifier-casing-ambiguous-recovery-json` cases and the run
`identifier-casing-import-recovery-isolation-json`,
`identifier-casing-qualified-type-import-isolation-json`,
`identifier-casing-valid-function-value-precedence-json`,
`identifier-casing-cross-class-ambiguous-recovery-json`,
`identifier-casing-owned-constructor-ambiguous-recovery-json`,
`identifier-casing-owned-constructor-ambiguous-recovery-human`,
`identifier-casing-same-name-recovery-arity-json`,
`identifier-casing-valid-function-precedence-cross-arity-json`, and
`identifier-casing-valid-constructor-precedence-cross-arity-json` cases cover
the import and public-alias recovery boundaries, accepted names, valid lookup
precedence for bare function-value references and cross-arity lowering
failures, handler binding quarantine from hole repair candidates, ambiguous
recovery refusal across name classes and same-owner constructor candidates,
call-arity filtering of same-name recovery peers, and qualified type
references. Focused semantic tests cover
expression-hole preservation, invalid value-binding quarantine from lookup and
repair candidates, function-value valid symbol precedence, cross-arity valid
symbol precedence, cross-class recovery ambiguity, and import isolation.
The `identifier-casing-record-field-reachability` case covers the boundary
that record field labels do not make same-spelled invalid declarations
reachable.

## Outcome

This record completed the first reviewable casing foundation for source-written
type, constructor, function, and value-binding declarations. The implementation
preserves the existing selection boundary of `check` and the existing
selected-entry reachability boundary of `run`.

This work is the first slice of
[Identifier Casing](../../proposals/identifier-casing.md). A declaration-only validation slice
was not independently implementable. Analysis needs a surface graph before it
can compute selected-entry reachability, but an invalid declaration must not
enter that graph as a normal symbol. This slice therefore added the minimum
quarantined recovery representation needed to preserve the graph and exclude
invalid symbols from checked artifacts.

## Scope

The source positions and `name.invalid_case` diagnostic contract are specified
by [Names And Effects](../../specification/names-effects.md) and its checked
identifier-casing examples. Module identities, alias target leaves, and repair
rename remain outside this completed slice. Source-less registry validation is
completed separately in
[Identifier Casing Source-Less Symbols](identifier-casing-source-less-symbols.md).
Qualified-use path casing is completed separately in
[Identifier Casing Qualified Use Paths](identifier-casing-qualified-use-paths.md).
LSP rename conflict rejection for valid selected workspace symbols is
completed separately in
[Identifier Casing Rename Conflicts](identifier-casing-rename-conflicts.md).

The function row includes function declarations, test declaration names, and
public function alias declaration names. The type row includes source ADT type
declarations and public type alias declaration names. This slice validates
those declarations when `check` or `run` selects them. It did not add the
dedicated `test` command selection evidence that the later boundary proposal
requires.

An invalid declaration or binding is retained only as a quarantined recovery
record. It does not enter normal lookup, checked core, typed intermediate
representation, package exports, or a backend. A compatible use can link to a
unique recovery record only to suppress a derivative diagnostic. This slice
does not expose rename. Recovery navigation is completed separately in
[Identifier Casing Recovery Navigation](identifier-casing-recovery-navigation.md).

Validation preserves parse recovery for underscore-led names and retains the
complete written token span. A standalone `_` keeps its existing wildcard,
discard, or structural-error behavior.

## Command Boundary

The casing diagnostic follows each consumer's existing selected unit. It does
not become a project-wide or source-wide loading gate.

| Consumer state | Result | Evidence |
| --- | --- | --- |
| `check` selects a source with an invalid covered name. | Reports every selected casing diagnostic and produces no successful checked artifact. | Human and JSON checked cases with exact spans and details. |
| `run` reaches an invalid declaration or binding from the selected entry. | Reports the casing diagnostic and produces no executable artifact. | Run cases with a valid entry that reaches an invalid function, constructor call, invalid public function alias, invalid public type alias through a type reference, invalid type or constructor through an expression path, and invalid handler bindings. |
| `run` does not reach an invalid declaration from the selected entry. | Runs the valid reachable closure and omits the unreachable casing diagnostic. | Run case with a valid entry, a reachable same-spelled local value, and invalid unreachable peers in the same source. |

## Recovery Boundary

| Source state | Result | Evidence |
| --- | --- | --- |
| One compatible invalid declaration is referenced. | Keeps one `name.invalid_case`; suppresses only failures caused by absence of that declaration; emits no normal symbol or artifact for it. | Focused semantic and checked-artifact cases. |
| A valid declaration and an invalid recovery record share a spelling. | The valid declaration wins normal lookup. | Lookup decision-table test. |
| More than one compatible recovery record shares a spelling. | Selects no recovery record and preserves independently provable ambiguity or unresolved facts. | Ambiguous recovery test. |
| Same-spelled recovery records belong to different name classes. | Selects no recovery record for a use that could otherwise reach more than one class and preserves the unresolved call fact. | Cross-class ambiguous recovery cases. |
| Same-owner constructor recovery records would report the same owner diagnostic. | Selects no recovery record and preserves the ordinary duplicate and unresolved facts. | Owned-constructor ambiguous recovery run cases. |
| Same-spelled callable recovery records have different arity. | Selects only arity-compatible callable recovery records before applying the unique-recovery rule. | Same-name recovery arity case. |
| A use crosses an import or public alias boundary. | Does not expose the recovery record across the boundary. | Checked diagnostics and artifact assertions for both boundaries. |

The implementation may use any internal representation that satisfies these
outcomes. The quarantined representation is a required semantic boundary, not
a requirement for a particular data structure.

## Selection Boundary Follow-Up

[Identifier Casing Selection Boundaries](identifier-casing-selection-boundaries.md)
completed the implicit-prelude, `test`, `doc`, language-service snapshot and
overlay, companion, and loaded and unloaded direct-dependency boundaries. The
current specification coverage lives in
[Names And Effects](../../specification/names-effects.md) and
[Editor Support](../../specification/editor-support.md). Those boundaries
depend on this foundation but are not part of this slice's completion rule.

## Completion

This slice is complete. Current behavior lives in
[Names And Effects](../../specification/names-effects.md) and checked cases
under `examples/specification/`. This completion unblocked the
selection-boundary proposal. It did not complete the remaining
identifier-casing work.
