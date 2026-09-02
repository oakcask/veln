---
role: proposal
update-when: The proposed Veln identifier casing classes, rejection diagnostics, migration scope, acceptance evidence, or implementation status changes.
---

# Identifier Casing

## Summary

Separate type and value names by their first ASCII letter. Type names and
algebraic data type (ADT) variant names start with an uppercase letter. Module
and value names start with a lowercase letter.

This rule makes a bare `Name(...)` a constructor form and a bare `name(...)` a
function or callable-value form. An accepted program cannot contain a
same-spelled callable binding and constructor, so call resolution does not need
a precedence rule for that collision.

This proposal is a language-semantics prerequisite for the complete definition
and reference matrix in
[Agent Language Services](agent-language-services.md). It is independent of
MCP and LSP transport behavior.

Implementation starts with
[Recovery-Aware Source Identifier Casing](../reference/implemented-proposals/identifier-casing-source-recovery.md).
That foundation combines declaration and binding diagnostics with quarantined
recovery and `check`/`run` reachability. The dependent
[Identifier Casing Selection Boundaries](../reference/implemented-proposals/identifier-casing-selection-boundaries.md)
record covers `test`, `doc`, language-service selection, dependencies,
companions, and the implicit prelude as current behavior. The remaining work in
this proposal is now selectable from the proposal catalog.

## Current Boundary

The implemented recovery-aware source foundation validates source-written ADT
type, ADT variant, function, test, public type-alias, public function-alias,
and value-binding declarations. [Names And Effects](../specification/names-effects.md)
specifies those current casing rules, quarantined recovery records, checked
artifact exclusion, and the `check` and `run` selection boundary. The completed
implementation record is
[Recovery-Aware Source Identifier Casing](../reference/implemented-proposals/identifier-casing-source-recovery.md).

The remaining proposal work covers module identities and deferred recovery
consumers. The completed public alias
target-leaf casing boundary is specified by
[Names And Effects](../specification/names-effects.md) and checked by the
`identifier-casing-public-alias-targets-json` and
`identifier-casing-public-alias-targets-human` examples. Its completion record
is
[Identifier Casing Public Alias Targets](../reference/implemented-proposals/identifier-casing-public-alias-targets.md).
The completed `test`, `doc`, companion recovery, direct-dependency,
implicit-prelude, and language-service snapshot and open-document overlay
boundaries are specified by
[Names And Effects](../specification/names-effects.md), by
[Editor Support](../specification/editor-support.md), and by the checked
examples that those pages name.
Source-less compiler-provided source lookup descriptors are specified by
[Source-Less Lookup](../specification/source-less-lookup.md) and covered by
focused `veln-sema` registry tests. Their completion record is
[Identifier Casing Source-Less Symbols](../reference/implemented-proposals/identifier-casing-source-less-symbols.md).
Qualified constructor patterns whose final segment starts with an ASCII
lowercase letter are specified by
[Name Resolution](../specification/name-resolution.md) and
[Types](../specification/types.md), and checked by the
`identifier-casing-qualified-constructor-pattern-json`,
`identifier-casing-qualified-constructor-pattern-human`,
`identifier-casing-qualified-constructor-pattern-over-suppression-json`,
`identifier-casing-qualified-constructor-pattern-direct-diagnostics-json`, and
`identifier-casing-qualified-constructor-pattern-type-mismatch-json`
examples. Their completion record is
[Identifier Casing Qualified Constructor Patterns](../reference/implemented-proposals/identifier-casing-qualified-constructor-patterns.md).
Written import-path segments are specified by
[Name Resolution](../specification/name-resolution.md) and
[Check JSON And Diagnostics](../specification/diagnostics-json.md), and
checked by the `identifier-casing-import-path-json` and
`identifier-casing-import-path-human` examples. The
`identifier-casing-import-missing-module-overlap-json`,
`identifier-casing-import-duplicate-overlap-json`, and
`identifier-casing-import-alias-cascade-boundary-json` examples check their
overlap with missing-module, duplicate-alias, and alias-use cascade
diagnostics. The type, constructor, and schema cascade cases check the same
quarantine boundary for imported qualified names. The effect, handler, and
ordering cases check the remaining written-import consumer and source-order
boundaries. Their completion record is
[Identifier Casing Import Paths](../reference/implemented-proposals/identifier-casing-import-paths.md).
Qualified-use path casing diagnostics for module-only, module-and-type, and
`prelude`-qualified expression, pattern, and type paths are specified by
[Name Resolution](../specification/name-resolution.md) and checked by the
`identifier-casing-qualified-use-paths-json` and
`identifier-casing-qualified-use-paths-human` examples. The
`identifier-casing-qualified-use-recovery-controls-json` and
`identifier-casing-qualified-use-recovery-controls-human` examples check the
same-source type-qualifier recovery and unresolved-control boundaries. Editor
navigation for
constructor-qualified type segments, imported module-and-type constructor
paths, `prelude`-qualified function and type paths, and module-only qualified
public functions is specified by
[Editor Support](../specification/editor-support.md) and checked by the
`identifier-casing-qualified-use-navigation`,
`identifier-casing-qualified-module-type-navigation`,
`identifier-casing-qualified-prelude-navigation`, and
`identifier-casing-qualified-function-navigation` examples. Their completion
record is
[Identifier Casing Qualified Use Paths](../reference/implemented-proposals/identifier-casing-qualified-use-paths.md).
The remaining qualified-use proposal scope covers recovery behavior beyond
those compiler and language-service diagnostics.

The LSP single-file diagnostics helper now receives the same parse-clean
source invalid-name records as `check`. That helper behavior is part of the
implemented source foundation. Workspace snapshot and open-document overlay
casing diagnostics and invalid-symbol index exclusion are also current
language-service behavior. LSP definition, references, prepare-rename, and
rename now expose a unique class-compatible invalid source declaration or
binding recovery record when no valid symbol wins. That recovery navigation
and rename boundary is specified by
[Editor Support](../specification/editor-support.md), checked by the
`identifier-casing-recovery-navigation` and
`identifier-casing-handler-binding-navigation` LSP examples and focused
`veln-language-service` tests. The LSP examples cover valid-symbol precedence,
ambiguous recovery rejection, incompatible-role rejection, shadowed occurrence
rejection, qualified occurrence rejection, lexical out-of-scope rejection,
successful complete recovery rename edits, and edit-free rename failures,
including a callable parameter call target. The focused language-service tests
also cover local-binding initializer exclusion, callable parameter and
local-binding use through both value and call positions, valid nullary
constructor precedence over function or binding recovery, recovery rename
class validation, and recovery rename conflict rejection. This behavior is
recorded in
[Identifier Casing Recovery Navigation And Rename](../reference/implemented-proposals/identifier-casing-recovery-navigation.md).
MCP `definition` exposes the same unique source declaration or binding
recovery definition boundary through
[MCP Workspace Projects, Diagnostics, And Definitions](../specification/mcp.md)
and the shared language-service selector. The `definition-recovery-navigation`
MCP example covers source declaration recovery, ambiguous recovery refusal,
and valid-symbol precedence.
LSP rename now rejects class-changing replacements
for selected valid workspace type, constructor, function, and value-binding
symbols and predictable rename conflicts as specified by
[Editor Support](../specification/editor-support.md) and checked by the
`identifier-casing-rename-boundary` example. That current boundary includes
rejection of ambiguous bare imported type-role references and qualified type
identity preservation for type rename, conflicts with visible type aliases in
the type namespace, and rejection of unedited bare imported function
ambiguities. It also rejects constructor ambiguity through public type-alias
re-export visibility and handler parameter captures for edited bare function
calls and function-value references. This proposal still owns MCP rename
mapping and rename evidence for the remaining casing surfaces until those rows
are implemented. LSP source-path-derived module segments are not rename
targets: prepare-rename returns `null`, and rename returns an empty workspace
edit without resource operations.
The namespace-by-use-role casing boundary for equal-spelled schemas, effects,
handlers, operations, types, constructors, functions, and value bindings is
specified by [Name Resolution](../specification/name-resolution.md), by
[Editor Support](../specification/editor-support.md), and checked by the
`identifier-casing-namespace-use-roles` example plus focused
`veln-language-service` tests. Its completion record is
[Identifier Casing Namespace Use Roles](../reference/implemented-proposals/identifier-casing-namespace-use-roles.md).

The lexer still tokenizes `_` as a standalone underscore and `_label` as a
named hole rather than as an identifier. The parser already interprets `_` as a
non-binding wildcard in supported binding and pattern positions.

Bare patterns already treat an uppercase initial as a constructor signal and a
lowercase initial as a binding signal. A lowercase ADT variant can therefore be
called as a constructor but cannot use the same bare spelling as a constructor
pattern. Existing navigation cases also contain a lowercase `byte` variant
that can collide with a same-spelled function or callable binding.

[Names And Effects](../specification/names-effects.md#name-resolution)
specifies current value shadowing. [Types](../specification/types.md)
specifies source ADTs and constructor resolution. The short
[Names And Effects](../specification/names-effects.md) route names the checked
casing evidence for the implemented source foundation; this proposal retains
only the unimplemented casing extensions.

## Remaining Naming Contract

[Names And Effects](../specification/names-effects.md) is the source of truth
for current source-written type, constructor, function, test, public
type-alias, public function-alias, value-binding declaration casing, public
alias target-leaf casing, and class-preserving LSP rename validation for
selected valid workspace type, constructor, function, and value-binding
symbols. [Name Resolution](../specification/name-resolution.md) and
[Types](../specification/types.md) specify the current qualified
constructor-pattern leaf boundary. Its completion record is
[Identifier Casing Qualified Constructor Patterns](../reference/implemented-proposals/identifier-casing-qualified-constructor-patterns.md).
Written import-path segments are specified by
[Name Resolution](../specification/name-resolution.md) and
[Check JSON And Diagnostics](../specification/diagnostics-json.md).
Qualified-use path casing diagnostics and the covered qualified-use
language-service operations are specified by
[Name Resolution](../specification/name-resolution.md),
[Check JSON And Diagnostics](../specification/diagnostics-json.md), and
[Editor Support](../specification/editor-support.md). Their completion record
is
[Identifier Casing Qualified Use Paths](../reference/implemented-proposals/identifier-casing-qualified-use-paths.md).
Source-path-derived module identity segments are specified by
[Name Resolution](../specification/name-resolution.md) and
[Check JSON And Diagnostics](../specification/diagnostics-json.md), with
their LSP diagnostic and rename boundary specified by
[Editor Support](../specification/editor-support.md). They are checked by the
`identifier-casing-source-path-json`,
`identifier-casing-exported-source-path-json`,
`identifier-casing-source-path-human`,
`identifier-casing-chained-companion-boundary-json`, and
`identifier-casing-source-path-boundary` examples. Their completion record is
[Identifier Casing Source Path Module Identities](../reference/implemented-proposals/identifier-casing-source-path-module-identities.md).
This proposal now specifies only identifier-casing work that remains
incomplete.

The remaining proposal keeps the same class initials: type and constructor
roles require an ASCII uppercase initial, and module, function, and
value-binding roles require an ASCII lowercase initial. The remaining work
applies those initials to this not-yet-current surface:

| Surface | Proposed rule |
| --- | --- |
| Module identities | Every written module identity and explicit import alias starts with an ASCII lowercase letter. |

Name lookup remains case-sensitive. Identifiers outside the current
specification and the remaining surfaces above keep their existing casing
behavior. This proposal does not change schema names, effect names, handler
names, effect operation names, record fields, type parameters, or hole labels.
These names remain in their existing syntactic namespaces and never become
ordinary constructor or value-call candidates merely because their spelling
collides with a cased name. Cross-namespace equal spellings remain permitted.
Duplicate checking remains within each existing namespace. Schema composition
retains its existing type-versus-schema ambiguity rule because that source
position intentionally admits both namespaces.

## Observable Rejections

For implemented source declaration and binding positions, current behavior is
specified by [Names And Effects](../specification/names-effects.md) and checked
identifier-casing examples. The rejection rows below remain proposal scope.
The primary message identifies the failed fact and required class.

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| A written module identity or explicit import alias starts with an uppercase letter. | Reject the offending segment with a message that the module name must start with an ASCII lowercase letter. | Module and import boundary fixtures. |
| An underscore-led token occurs in a remaining written module position. | Reject it with `name.invalid_case` for the module class. | Module parser-recovery and import fixtures. |

An invalid remaining-scope name does not introduce a normal symbol under
another spelling. Command-specific analysis and lowering boundaries determine
whether a casing diagnostic blocks an artifact.

### Diagnostic Locations

The exact lexer-token span is authoritative for each written remaining-scope
name. The current qualified-use segment span and classification contract is
specified by [Name Resolution](../specification/name-resolution.md),
[Check JSON And Diagnostics](../specification/diagnostics-json.md), and
[Editor Support](../specification/editor-support.md). This proposal keeps only
the not-yet-current module identity and future explicit import-alias span
boundaries here. A synthetic missing token introduced during parser recovery
has no name to validate.

An implicit import alias derived from the final import-path segment is current
behavior specified by
[Name Resolution](../specification/name-resolution.md). A future explicit
alias with its own token is a separate name occurrence and is validated at that
token.

The current structured `name.invalid_case` details for source-written
declarations, bindings, and public alias targets are specified by
[Names And Effects](../specification/names-effects.md). Remaining
source-written `name.invalid_case` diagnostics use the same details when the
occurrence is a path segment:

| Field | Required value |
| --- | --- |
| `phase` | `name` |
| `origin` | `source` |
| `occurrence` | `declaration`, `binding`, `path_segment`, or `pattern_head` |
| `name` | The exact written spelling. |
| `name_class` | `type`, `constructor`, `module`, `function`, or `value_binding` |
| `required_initial` | `ascii_uppercase` or `ascii_lowercase` |
| `observed_initial` | `ascii_uppercase`, `ascii_lowercase`, `underscore`, or `other` |

A written qualified occurrence also has its zero-based `segment_index` within
the written path. Source-path occurrences are current behavior specified by
[Check JSON And Diagnostics](../specification/diagnostics-json.md).

## Resolution Consequences

The implemented source foundation quarantines invalid source declarations,
bindings, and public alias targets for `check`, `run`, and LSP single-file
diagnostics. The remaining rules in this section are proposal scope where they
require module identity validation or the deferred language-service selection
boundary.

For sources without casing diagnostics, ordinary expression calls and
constructor patterns use these candidate classes. Dedicated schema, effect,
operation, and handler positions continue to use their own namespaces.

| Source form | Candidate class |
| --- | --- |
| Bare `Name` or `Name(...)` with an uppercase initial. | ADT constructor only. |
| Bare `name` or `name(...)` with a lowercase initial. | Value binding, function, or other existing lowercase call target; never an ADT constructor. |
| Qualified `path::Name` with an uppercase final segment. | ADT constructor under the existing qualifier and visibility rules. |
| Qualified `path::name` with a lowercase final segment. | Existing non-constructor value or call target under the qualifier's rules. |

Qualified paths are decomposed by semantic role rather than segment position:

| Resolved form | Segment classes |
| --- | --- |
| `module::value` | Module, then function or value binding as required by the use. |
| `module::Type` | Module, then type. |
| `Type::Constructor` | Type, then constructor. |
| `module::Type::Constructor` | Module, type, then constructor. |
| `prelude::Type::Constructor` | Reserved module alias, type, then constructor. |

The decomposition validates every role-classified written segment. It does not
treat every prefix as a module and does not infer an unresolved intermediate
role from capitalization alone.

The qualified constructor-pattern leaf boundary and role-fixed qualified-use
path casing diagnostics are now current behavior, specified by
[Name Resolution](../specification/name-resolution.md) and
[Types](../specification/types.md). The remaining qualified-use proposal scope
does not reinterpret lowercase qualified constructor patterns as value
bindings.

Callability remains a type property. A lowercase binding with a non-function
type still blocks the same-spelled lowercase function according to the current
value-shadowing rule. A local binding is still not visible in its own
initializer when the current specification resolves the initializer to an
outer binding or function. This proposal removes constructor-versus-value
collisions; it does not change value-versus-value shadowing.

For remaining proposal surfaces, checking, lowering, definition, references,
prepare-rename, and rename must use the same name class. LSP and MCP must not
add adapter-specific exceptions.

### Remaining Invalid-Name Recovery

[Names And Effects](../specification/names-effects.md) specifies current
quarantined recovery for source declarations and bindings selected by `check`,
`run`, LSP single-file diagnostics, workspace snapshot and open-document
overlay selection, and exact companion source and target boundaries. The
remaining proposal defines how that recovery model extends to remaining
qualified-use recovery links and remaining companion cases for invalid module
or qualified roles. Source-path-derived module identity
failures are current
behavior specified by [Name Resolution](../specification/name-resolution.md)
and [Check JSON And Diagnostics](../specification/diagnostics-json.md); this
proposal covers their unimplemented interactions with graph, artifact, and
deferred recovery consumers.

An invalid remaining-scope module segment or qualified segment is not inserted
into a normal name class. A use links to a recovery record only when the
original spelling matches, the recovered class is compatible with the
syntactic or resolved use role, no valid candidate wins, and exactly one
compatible recovery record is in scope. The initial-derived class filter is
ignored only for this repair lookup. The link supports cascade suppression,
but it does not make the program valid.

| Invalid record | Compatible recovery uses | Incompatible recovery uses |
| --- | --- | --- |
| Qualified constructor-call leaf `some` | Constructor-call positions where resolution fixes a constructor role. | Bare pattern `some`, which declares a binding. |
| Qualified type leaf with a lowercase initial | Qualified type positions. | Expression and pattern-binding positions. |
| Qualified function leaf with an uppercase initial | Qualified callable-value positions. | Type positions and constructor patterns. |
| Invalid module segment | Diagnostics and local source analysis for the declaring source identity. | Imports, exports, public aliases, dependencies, companions, prelude lookup, and backend reachability. |

A valid candidate always takes precedence over a recovery record. Recovery
records do not create ambiguity or cross-class collisions. When multiple
recovery records have the same spelling, no one record is selected for
navigation. Definition, references, prepare-rename, and LSP rename exposure
for source declaration and binding recovery records is current editor behavior
specified by [Editor Support](../specification/editor-support.md). MCP
`definition` exposure for those recovery records is current behavior specified
by
[MCP Workspace Projects, Diagnostics, And Definitions](../specification/mcp.md).
No other operation treats a quarantined record as valid. Lowering and backends
never receive recovery records.

For the remaining boundary work, a recovery record is visible only in the
declaring source module and in the lexical scope that the corresponding valid
declaration would have occupied. It does not cross exact-companion privilege,
language-service snapshot or overlay ownership, or the implicit prelude import.
An invalid remaining-scope public surface is absent from public API and package
snapshot symbol indexes. Direct-dependency source declaration and binding
recovery isolation is current behavior specified by
[Names And Effects](../specification/names-effects.md). Package and workspace
indexes may retain remaining-scope locations for diagnostics, but downstream
lookup and navigation do not expose them.

Current source-path-derived module identity casing reports one diagnostic for
each invalid origin segment and withholds the invalid derived identity from
normal module registration. That implemented registration invariant also means
that the invalid identity cannot satisfy a local import, participate in a
duplicate derived-module relationship, or add a reachable module-graph edge.
A second valid source cannot derive the same invalid-cased identity because
every accepted derived identity satisfies the same segment validation.
[Name Resolution](../specification/name-resolution.md) specifies this current
boundary and routes to the focused executable examples. Duplicate diagnostics
consume only registered identities, so there is no separate invalid-versus-valid
collision input to construct.

The remaining proposal separates unimplemented consumers by their observable
source-error boundary:

| Consumer boundary | Required outcome | Evidence boundary |
| --- | --- | --- |
| Artifact commands that reject source-graph errors. | The command returns the source diagnostic envelope and no artifact or policy result. | Metrics is now covered separately by [Metrics JSON](../specification/metrics-json.md). |
| Export, documentation, backend, and deferred recovery consumers. | Each consumer follows its existing source-error contract and exposes no normal artifact identity for the invalid source. A tolerant consumer continues unrelated valid-module analysis; a fail-fast consumer returns its specified error result without an artifact. | Consumer-specific cases state whether the command is tolerant or fail-fast and assert the corresponding valid-module or no-artifact boundary. |

A target must not require an invalid source to reach an artifact stage that the
current command specification blocks on source diagnostics. [Metrics Partial
Source Analysis](../reference/implemented-proposals/metrics-partial-source-analysis.md)
owns the implemented metrics command exception. A structurally invalid path
retains its existing structural module diagnostic and does not also create a
module identity.

The `test` command's source dependency selection graph is a separate consumer.
Its source-identity, parse-failure, completeness, and widening rules do not
verify source module registration. Changes to those rules require command-level
acceptance cases under the test selection contract. Metrics dependency-cycle
policy history remains owned by
[Metrics Partial Source Analysis](../reference/implemented-proposals/metrics-partial-source-analysis.md).

Every invalid name reports `name.invalid_case`. Independently provable
diagnostics still accumulate. In particular, remaining-scope names with the
same kind, scope, and original spelling still participate in duplicate
checking.
Resolution, ambiguity, kind, and unresolved diagnostics whose only cause is a
quarantined record are suppressed.

| Overlap at one occurrence | Required result |
| --- | --- |
| A retained token independently violates parse or module structure and casing. | Report the structural diagnostic, then `name.invalid_case`. A synthetic missing token has no casing diagnostic. |
| The spelling is independently reserved. | Report both reserved-name and casing diagnostics. |
| The original spelling duplicates the same class in the same scope for a remaining-scope name. | Report casing for each invalid occurrence and the existing duplicate diagnostic. |
| Valid candidates independently create ambiguity. | Report casing and the existing ambiguity diagnostic. |
| Unresolved, callability, type-origin, constructor-arity, or exhaustiveness failure exists only because of one recovery record. | Report casing and suppress the derivative diagnostic. |

Existing related notes remain owned by their original diagnostics. Suppressed
diagnostics do not add explanation fields to successful JSON output; planned
decision-table tests record why each omitted cascade is absent.

Diagnostics have a deterministic order by source identity, primary-span start,
primary-span end, diagnostic priority, and diagnostic id. At the same span,
structural parse or module diagnostics precede `name.invalid_case`, which
precedes duplicate, ambiguity, kind, unresolved, type, and lowering
diagnostics.

### Remaining Rename Boundary

Current LSP rename validates selected valid workspace type, constructor,
function, and value-binding symbols before it produces edits. That implemented
behavior is specified by [Editor Support](../specification/editor-support.md).
For type-role references, the current boundary rejects ambiguous bare imported
type selections, preserves qualified type identity before returning rename
edits, and covers the implemented qualified-use path segment selections named
above. LSP recovery rename through quarantined source declaration and binding
recovery records is also current behavior specified by
[Editor Support](../specification/editor-support.md). The remaining rename
proposal covers MCP error mapping and deferred module surfaces.

Current rename conflict rejection for valid selected workspace symbols is
specified by [Editor Support](../specification/editor-support.md), including
same-scope declaration duplicates, edit-scope-based lexical shadowing, type
alias conflicts, function-to-test duplicate rejection, affected scopes, and
conflicting declaration locations for local bindings, function parameters,
result bindings, handler context parameters, and handler operation clause
parameters. Future MCP rename surfaces must preserve that shared conflict code
and edit-free failure boundary when they add their transport-specific
behavior.

Source-path-derived module segments are not LSP rename targets. Current
behavior is specified by [Editor Support](../specification/editor-support.md)
and checked by the `identifier-casing-source-path-boundary` example. A future
module-rename capability must define filesystem and client resource-operation
behavior separately.

## Analysis And Artifact Boundary

Casing uses the selection and reachability boundary already defined by each
command. It does not add a workspace-global gate. The `check` and `run`
boundaries for current source-written declaration and binding casing, including
loaded and unloaded direct dependencies, are specified by
[Names And Effects](../specification/names-effects.md). The implicit-prelude
selection boundary is complete and recorded in
[Identifier Casing Selection Boundaries](../reference/implemented-proposals/identifier-casing-selection-boundaries.md).
The remaining proposal scope is limited to module identity, MCP rename
mapping, and deferred language-service consumers listed in the
acceptance model.

No backend receives a remaining-scope module identity or recovery record with
an invalid case. The planned command fixtures are authoritative for the exact
selection boundary when command behavior differs.

## Goals

- Make type, constructor, module, function, and value-binding names visually
  distinguishable at their declaration and use sites.
- Make bare constructor patterns consistent with constructor calls.
- Remove callable-binding-versus-constructor precedence from valid programs.
- Preserve current visibility, ambiguity, and value-shadowing rules within
  each name class.
- Extend the current source foundation to the remaining module, selection, and
  MCP rename mapping surfaces.

## Non-Goals

- Changing which constructors, functions, or bindings are visible.
- Changing duplicate-name rules inside one name class.
- Changing value-versus-value shadowing or initializer visibility.
- Requiring a complete CamelCase or snake_case word convention after the first
  character.
- Renaming schemas, effects, handlers, operations, fields, type parameters, or
  holes as part of this change.
- Defining unrelated MCP or LSP schemas, coordinates, project scope, or
  transport errors beyond the remaining rename failure mappings required here.

## Migration

Remaining implementation work must update every repository-owned input that a
parser, analyzer, documentation gate, or language-service case treats as Veln
source for the affected surface. The inventory includes `.veln` files,
embedded standard-library sources, Rust test strings and generated test
sources, executable examples, accepted and rejected fixtures, checked Markdown
doctests, editor and agent service source cases, snapshots, and expected
diagnostics, locations, or navigation edits. Non-executable Veln examples must
also use the new contract unless they preserve an externally defined spelling
or explicitly illustrate invalid casing.

The only sources that may retain invalid casing are dedicated
identifier-casing rejection cases with an exact expected diagnostic id, count,
message, and span. Migration evidence must audit parsed and analyzed source
carriers rather than relying only on a `.veln` file search.

The change provides no compatibility alias for an invalid old spelling. A
lowercase function such as the standard `byte` helper remains a function and
does not become a constructor.

## Acceptance Model

Rows covered by
[Recovery-Aware Source Identifier Casing](../reference/implemented-proposals/identifier-casing-source-recovery.md)
and completed rows in
[Identifier Casing Selection Boundaries](../reference/implemented-proposals/identifier-casing-selection-boundaries.md)
are no longer planned work. The table below retains only the unimplemented
identifier-casing remainder.

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Analyze an invalid derived module beside remaining artifact consumers. | The invalid source contributes no export, documentation module, backend reachability, or deferred recovery consumer result. Each case follows the consumer's specified fail-fast or diagnostic-tolerant boundary and proves continued unrelated analysis only when that consumer produces analysis despite source errors. Source module registration, import resolution, duplicate detection, and reachable module-edge isolation are current behavior. | Export, documentation, backend, and deferred recovery consumer cases with an explicit source-error boundary. |
| Observe name ranges through every diagnostic and language-service consumer. | Parser-retained token spans, human and JSON spans, definition, references, prepare-rename, and rename ranges agree for each written name segment. | CRLF, preceding Unicode, multiline, recovery, and qualified-path fixtures. |
| Resolve uses near invalid declarations in qualified and module-derived roles not covered by current behavior. | A unique class-compatible quarantined symbol suppresses only derivative cascades where the selected operation permits recovery; valid candidates win; bare binding patterns do not become constructors; multiple candidates do not create arbitrary navigation. | Recovery decision table for remaining qualified, module, and boundary cases. |
| Cross remaining module or qualified boundaries with an invalid declaration. | A recovery symbol remains limited to the declaring source and lexical scope. No recovery symbol is imported, aliased, lowered, or exposed as a module-derived or qualified recovery result. | Boundary table covering diagnostics, module-derived and qualified navigation attempts, and artifacts for deferred boundaries. |
| Combine casing with structural, reserved-name, duplicate, ambiguity, and unresolved failures. | Every direct and independently provable error appears once in the defined order with the required details and unchanged related notes; recovery-derived cascades do not appear. | Exact ordered human and JSON overlap tables, including an asserted reason for every expected absence. |
| Request remaining transport rename mappings. | MCP mappings return no edits for unsupported recovery or module selections and preserve shared failure codes where the transport exposes them. | Planned MCP error-mapping cases. |
| Run each remaining deferred language-service consumer with casing errors inside and outside its selected unit. | Remaining service operations apply the same selected-unit boundary as checking, and no invalid module or qualified recovery symbol is returned as a normal service result. | Language-service fixtures covering the remaining module-derived and qualified surfaces. |
| Navigate accepted function, binding, type, and constructor uses. | The language service selects only the symbol class fixed by the initial letter. | Definition, reference, and rename cases in `veln-language-service`. |
| Run the repository source-carrier audit and specification suite after migration. | Every parsed or analyzed repository-owned source follows the contract except dedicated exact-expectation casing fixtures, and unrelated negative fixtures retain their intended diagnostic sets. | Source-carrier audit, specification harness, doctest and documentation gates, and workspace tests. |

This proposal is complete when all acceptance rows pass, all repository-owned
Veln sources follow the remaining naming contract, and the implemented behavior is
stated under `docs/specification/` and routed to checked examples under
`examples/specification/`. The completion change must also update
[the proposal catalog](README.md): remove identifier casing from `Ready`, and
move the saved workspace function-reference slice in
[Agent Language Services](agent-language-services.md) from `Blocked` to
`Ready`.
