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

The remaining proposal work covers module identities, qualified-use segment
casing beyond constructor-pattern leaves, recovery navigation, repair rename,
and rename conflict prediction. The completed public alias target-leaf casing
boundary is specified by
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

The LSP single-file diagnostics helper now receives the same parse-clean
source invalid-name records as `check`. That helper behavior is part of the
implemented source foundation. Workspace snapshot and open-document overlay
casing diagnostics and invalid-symbol index exclusion are also current
language-service behavior. LSP rename now rejects class-changing replacements
for selected valid workspace type, constructor, function, and value-binding
symbols as specified by [Editor Support](../specification/editor-support.md)
and checked by the `identifier-casing-rename-boundary` example. That current
boundary includes rejection of ambiguous bare imported type-role references
and qualified type identity preservation for type rename. This proposal still
owns recovery navigation, repair rename, rename conflict prediction, MCP
rename mapping, and rename evidence for the remaining casing surfaces until
those rows are implemented.

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
Source-path-derived module identity segments are specified by
[Name Resolution](../specification/name-resolution.md) and
[Check JSON And Diagnostics](../specification/diagnostics-json.md), and
checked by the `identifier-casing-source-path-json`,
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
applies those initials to these not-yet-current surfaces:

| Surface | Proposed rule |
| --- | --- |
| Module identities | Every written module identity and explicit import alias starts with an ASCII lowercase letter. |
| Qualified uses | Every written segment with a syntax-fixed or resolution-fixed role satisfies that role's name class. Unresolved or ambiguous intermediate segments are not guessed from spelling. |

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
name. A qualified source name retains one authoritative span per segment. The
parser, checker, human and JSON diagnostics, and language services use those
retained spans. They do not derive a name span by rescanning source text or
slicing a whole-declaration span. A synthetic missing token introduced during
parser recovery has no name to validate.

Each resolved qualified occurrence retains a semantic role for every segment.
The shared resolver classifies the longest module or import-alias prefix, an
optional type qualifier, and the final member required by the source position.
Only a role that is fixed by syntax, successful resolution, or one unique
recovery link receives a casing diagnostic. An unresolved or ambiguous
intermediate segment is not guessed from its position or spelling. A leaf whose
role is fixed by syntax or alias kind is validated even when its target is
unresolved. Checking and every language-service operation consume the same
classified segment records.

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
require module identity validation, qualified-use segment validation,
navigation, rename, or the deferred language-service selection boundary.

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

The qualified constructor-pattern leaf boundary is now current behavior,
specified by [Name Resolution](../specification/name-resolution.md) and
[Types](../specification/types.md). The remaining qualified-use proposal scope
does not reinterpret lowercase qualified constructor patterns as value
bindings.

Callability remains a type property. A lowercase binding with a non-function
type still blocks the same-spelled lowercase function according to the current
value-shadowing rule. A local binding is still not visible in its own
initializer when the current specification resolves the initializer to an
outer binding or function. This proposal removes constructor-versus-value
collisions; it does not change value-versus-value shadowing.

Checking, lowering, definition, references, prepare-rename, and rename must use
the same name class. LSP and MCP must not add adapter-specific exceptions.

### Remaining Invalid-Name Recovery

[Names And Effects](../specification/names-effects.md) specifies current
quarantined recovery for source declarations and bindings selected by `check`,
`run`, LSP single-file diagnostics, workspace snapshot and open-document
overlay selection, and exact companion source and target boundaries. The
remaining proposal defines how that recovery model extends to qualified-use
roles, remaining companion cases for invalid module or qualified roles, and
recovery navigation. Source-path-derived module identity failures are current
behavior specified by [Name Resolution](../specification/name-resolution.md)
and [Check JSON And Diagnostics](../specification/diagnostics-json.md); this
proposal covers their unimplemented interactions with graph, artifact, and
deferred recovery consumers.

An invalid remaining-scope module segment or qualified segment is not inserted
into a normal name class. A use links to a recovery record only when the
original spelling matches, the recovered class is compatible with the
syntactic or resolved use role, no valid candidate wins, and exactly one
compatible recovery record is in scope. The initial-derived class filter is
ignored only for this repair lookup. The link supports cascade suppression and
language-service navigation where the selected operation permits recovery, but
it does not make the program valid.

| Invalid record | Compatible recovery uses | Incompatible recovery uses |
| --- | --- | --- |
| Qualified constructor-call leaf `some` | Constructor-call positions where resolution fixes a constructor role. | Bare pattern `some`, which declares a binding. |
| Qualified type leaf with a lowercase initial | Qualified type positions. | Expression and pattern-binding positions. |
| Qualified function leaf with an uppercase initial | Qualified callable-value positions. | Type positions and constructor patterns. |
| Invalid module segment | Diagnostics and local source analysis for the declaring source identity. | Imports, exports, public aliases, dependencies, companions, prelude lookup, and backend reachability. |

A valid candidate always takes precedence over a recovery record. Recovery
records do not create ambiguity or cross-class collisions. When multiple
recovery records have the same spelling, no one record is selected for
navigation. Definition, references, and prepare-rename may expose a unique
recovery link. Rename may use that link only to produce a class-correct repair.
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
normal module registration. The remaining proposal separates consumers by
their observable source-error boundary:

| Consumer boundary | Required outcome | Evidence boundary |
| --- | --- | --- |
| Diagnostic-tolerant analysis, including import resolution and duplicate-module-content analysis. | The invalid identity does not satisfy an import or collide with a valid module. Independently provable diagnostics from unrelated valid modules still appear. | Checked command cases assert the casing diagnostic, the absent graph-derived diagnostic or candidate, and an unrelated valid-module diagnostic. |
| Artifact commands that reject source-graph errors, including the current metrics command. | The command returns the source diagnostic envelope and no artifact or policy result. A would-be dependency cycle through the invalid identity produces no cycle policy violation because source errors already block the report. | Command cases assert the source diagnostic and the absence of report and policy output. They do not claim that an invalid source reached artifact graph construction. |
| Export, documentation, backend, and deferred recovery consumers. | Each consumer follows its existing source-error contract and exposes no normal artifact identity for the invalid source. A tolerant consumer continues unrelated valid-module analysis; a fail-fast consumer returns its specified error result without an artifact. | Consumer-specific cases state whether the command is tolerant or fail-fast and assert the corresponding valid-module or no-artifact boundary. |

A target must not require an invalid source to reach an artifact stage that the
current command specification blocks on source diagnostics. A proposal that
changes such a command to return a partial artifact must first define the new
error, output, selection, and policy-evaluation contract. A structurally
invalid path retains its existing structural module diagnostic and does not
also create a module identity.

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
type selections and preserves qualified type identity before returning rename
edits. The remaining rename proposal covers repair rename through quarantined
invalid-name recovery records, predictable conflict rejection, source-path
module rename exclusion, MCP error mapping, and the deferred module and
qualified-use surfaces.

A repair rename edits the declaration and every occurrence linked to the same
unique recovery symbol, including an occurrence whose initial-derived valid
class differs from the repaired class. It does not edit a valid symbol,
incompatible use role, shadowed occurrence, ambiguous recovery occurrence, or
text that merely has the same spelling.

Before returning edits, rename checks the requested spelling in every affected
scope represented by the current analysis snapshot. If the complete edit would
create a duplicate in the symbol's namespace or an ambiguity that is already
provable in an affected scope, rename fails atomically with shared code
`rename.conflict` and details that identify the symbol class, requested name,
conflicting declaration, and affected scope. LSP maps this failure to JSON-RPC
invalid params (`-32602`), and a future MCP operation preserves the shared code
and details. Rename does not claim to validate consumers outside the current
snapshot.

Source-path-derived module segments are not rename targets in this proposal.
Prepare-rename returns no range for them. Rename produces no file operation,
including a case-only filesystem rename. A future module-rename capability
must define filesystem and client resource-operation behavior separately.

## Analysis And Artifact Boundary

Casing uses the selection and reachability boundary already defined by each
command. It does not add a workspace-global gate. The `check` and `run`
boundaries for current source-written declaration and binding casing, including
loaded and unloaded direct dependencies, are specified by
[Names And Effects](../specification/names-effects.md). The implicit-prelude
selection boundary is complete and recorded in
[Identifier Casing Selection Boundaries](../reference/implemented-proposals/identifier-casing-selection-boundaries.md).
The remaining proposal scope is limited to module identity, qualified-use,
recovery navigation, repair rename, rename conflict prediction, MCP rename
mapping, and deferred language-service consumers listed in the acceptance
model.

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
- Extend the current source foundation to the remaining module, qualified-use,
  selection, recovery navigation, repair rename, rename conflict prediction,
  and MCP rename mapping surfaces.

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
| Declare equal-spelled schemas, effects, handlers, operations, types, constructors, functions, and bindings. | Each dedicated source position selects its existing namespace, cross-namespace spellings do not create duplicates, ordinary calls exclude casing-neutral namespaces, and schema composition retains its existing ambiguity. | Namespace-by-use-role decision table with duplicate and definition cases. |
| Classify every segment of module-only, module-and-type, and prelude-qualified paths with each segment invalid in turn. | Every syntax- or resolution-fixed role receives its class diagnostic; unresolved intermediate roles are not guessed; all language-service operations observe the same decomposition. | Expression, pattern, type, definition, reference, and rename decision table. |
| Analyze an invalid derived module beside imports and duplicate-module contents. | Current source-path diagnostics remain attached to the source. The invalid identity does not satisfy an import or participate in duplicate-module-content analysis. An unrelated valid module still produces an independently provable diagnostic. | Import and duplicate checked-command cases that assert both isolation and continued valid-module analysis. |
| Request metrics for sources that include an invalid derived identity and declarations that would form a cycle if that identity were accepted. | The current source-error gate returns the casing diagnostic without a metrics report or dependency-cycle policy result. This case specifies fail-fast command behavior; it does not require the invalid source to enter the metrics graph. | A metrics command case that asserts the diagnostic envelope and the absence of report and policy fields. |
| Analyze an invalid derived module beside remaining artifact consumers. | The invalid source contributes no export, documentation module, backend reachability, or deferred recovery consumer result. Each case follows the consumer's specified fail-fast or diagnostic-tolerant boundary and proves continued unrelated analysis only when that consumer produces analysis despite source errors. | Export, documentation, backend, and deferred recovery consumer cases with an explicit source-error boundary. |
| Observe name ranges through every diagnostic and language-service consumer. | Parser-retained token spans, human and JSON spans, definition, references, prepare-rename, and rename ranges agree for each written name segment. | CRLF, preceding Unicode, multiline, recovery, and qualified-path fixtures. |
| Resolve uses near invalid declarations in qualified, module-derived, navigation, and rename roles not covered by current behavior. | A unique class-compatible quarantined symbol suppresses only derivative cascades and supports repair navigation where the selected operation permits recovery; valid candidates win; bare binding patterns do not become constructors; multiple candidates do not create arbitrary navigation. | Recovery decision table for remaining qualified, module, boundary, definition, reference, and rename cases. |
| Cross remaining module or qualified boundaries with an invalid declaration. | Recovery navigation exists only in the declaring source and lexical scope. No recovery symbol is imported, aliased, or lowered. | Boundary table covering diagnostics, definition, references, and artifacts for deferred boundaries. |
| Combine casing with structural, reserved-name, duplicate, ambiguity, and unresolved failures. | Every direct and independently provable error appears once in the defined order with the required details and unchanged related notes; recovery-derived cascades do not appear. | Exact ordered human and JSON overlap tables, including an asserted reason for every expected absence. |
| Request conflicting and invalid-declaration repair renames. | Repair renames return complete linked edits. Predictable collisions return `rename.conflict`; failures return no edits. Path-derived module segments return no prepare range or file edits. | Shared language-service, LSP error-mapping, and planned MCP error-mapping cases. |
| Run each remaining deferred language-service consumer with casing errors inside and outside its selected unit. | Remaining service operations apply the same selected-unit boundary as checking, and no invalid module or qualified recovery symbol is returned as a normal service result. | Language-service fixtures covering the remaining module, qualified, definition, references, prepare-rename, and rename surfaces. |
| Navigate accepted function, binding, type, and constructor uses. | The language service selects only the symbol class fixed by the initial letter. | Definition, reference, and rename cases in `veln-language-service`. |
| Run the repository source-carrier audit and specification suite after migration. | Every parsed or analyzed repository-owned source follows the contract except dedicated exact-expectation casing fixtures, and unrelated negative fixtures retain their intended diagnostic sets. | Source-carrier audit, specification harness, doctest and documentation gates, and workspace tests. |

This proposal is complete when all acceptance rows pass, all repository-owned
Veln sources follow the remaining naming contract, and the implemented behavior is
stated under `docs/specification/` and routed to checked examples under
`examples/specification/`.
