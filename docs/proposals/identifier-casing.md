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

## Current Boundary

The lexer distinguishes identifier text but does not assign a type-name or
value-name class from its first letter. Type declarations, ADT variants,
functions, and local bindings currently accept the same identifier token. It
tokenizes `_` as a standalone underscore and `_label` as a named hole rather
than as an identifier. The parser already interprets `_` as a non-binding
wildcard in supported binding and pattern positions.

Bare patterns already treat an uppercase initial as a constructor signal and a
lowercase initial as a binding signal. A lowercase ADT variant can therefore be
called as a constructor but cannot use the same bare spelling as a constructor
pattern. Existing navigation cases also contain a lowercase `byte` variant
that can collide with a same-spelled function or callable binding.

[Names And Effects](../specification/names-effects-full.md#name-resolution)
specifies current value shadowing. [Types](../specification/types-full.md)
specifies source ADTs and constructor resolution. Neither page defines the
identifier casing classes proposed here.

## Naming Contract

The first character of each declared name must match this table. `Uppercase`
means one ASCII letter in `A` through `Z`. `Lowercase` means one ASCII letter
in `a` through `z`.

| Name class | Required initial | Included declarations and bindings |
| --- | --- | --- |
| Type | Uppercase | Source ADT type declarations and public type aliases. |
| Constructor | Uppercase | Nullary and payload-carrying source ADT variants, whether public or private. |
| Module | Lowercase | Every segment of a written or source-path-derived module identity, import path, and import alias. |
| Function | Lowercase | Function declarations, test declarations, and public function aliases. |
| Value binding | Lowercase | Function parameters, result bindings, local `let` bindings, match and destructuring bindings, handler context parameters, operation-clause parameters, and hole `satisfy` candidates. |

The rule applies at the declaration or binding. A qualified use does not make
an invalid declaration valid. Name lookup remains case-sensitive.

A covered name must start with its required ASCII letter. An underscore does
not count as a lowercase initial. When `_name` or `_Name` occurs where the
grammar requires a covered declaration, binding, or module name, parser
recovery retains that token as an invalid name occurrence and reports
`name.invalid_case` instead of an additional missing-name parse diagnostic.
The same spelling remains a named hole in an expression position.

The standalone `_` token is not an identifier. It remains a non-binding
wildcard or discard only where the grammar already permits that form. It
produces a structural parse diagnostic, not `name.invalid_case`, where the
grammar requires a declared name or module segment. A source-path segment has
no token distinction, so `_` and underscore-led source-path segments violate
the module rule.

Identifiers outside the table keep their current casing behavior. This
proposal does not change schema names, effect names, handler names, effect
operation names, record fields, type parameters, or hole labels. These names
remain in their existing syntactic namespaces and never become ordinary
constructor or value-call candidates merely because their spelling collides
with a name in the table. Cross-namespace equal spellings remain permitted.
Duplicate checking remains within each existing namespace. Schema composition
retains its existing type-versus-schema ambiguity rule because that source
position intentionally admits both namespaces.

## Observable Rejections

Each invalid declaration or binding reports `name.invalid_case` at the complete
name span. The primary message identifies the failed fact and required class.

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| A type declaration or public type alias starts with a lowercase letter. | Reject it with a message that the type name must start with an ASCII uppercase letter. | Rejected source-surface fixture for both declarations. |
| An ADT variant starts with a lowercase letter. | Reject it with a message that the constructor name must start with an ASCII uppercase letter. | Rejected fixture covering nullary, payload, public, and private variants. |
| A written module identity, import-path segment, or import alias starts with an uppercase letter. | Reject the offending segment with a message that the module name must start with an ASCII lowercase letter. | Module and import boundary fixtures. |
| A source-path-derived module identity contains a segment that starts with an uppercase letter. | Reject the derived module with a message that names the segment and requires an ASCII lowercase initial. | Source-path module fixtures and structured diagnostic cases. |
| A function, test, or public function alias starts with an uppercase letter. | Reject it with a message that the function name must start with an ASCII lowercase letter. | Declaration and alias fixtures. |
| A value binding starts with an uppercase letter. | Reject it with a message that the binding name must start with an ASCII lowercase letter. | Table-driven parameter, result, `let`, pattern, and handler-binding cases. |
| An underscore-led token occurs in a covered declaration, binding, pattern-head, or written module position. | Reject it with `name.invalid_case` for the class required by that position. Keep `_name` as a named hole only in expression positions. | Lexer, parser-recovery, declaration, binding, pattern, module, and hole fixtures. |
| A function- or type-alias target leaf has casing incompatible with the alias kind. | Reject the leaf with `name.invalid_case` for the target class. A schema-alias target remains casing-neutral. | Ordered function-, type-, and schema-alias target fixtures. |

A declaration that violates this rule does not introduce a normal symbol under
another spelling. Analysis may retain a recovery symbol under the original
spelling as defined below. Command-specific analysis and lowering boundaries
determine whether a casing diagnostic blocks an artifact.

### Diagnostic Locations

The exact lexer-token span is authoritative for every source-written name
covered by this proposal. A qualified source name retains one authoritative
span per segment. The parser, checker, human and JSON diagnostics, and language
services use those retained spans. They do not derive a name span by rescanning
source text or slicing a whole-declaration span.

This requirement covers type, variant, function, test, public alias, parameter,
result, `let`, pattern, handler, operation-clause, and `satisfy` candidate names.
It also covers every segment of a written module identity or import path. A
synthetic missing token introduced during parser recovery has no name to
validate.

Each resolved qualified occurrence retains a semantic role for every segment.
The shared resolver classifies the longest module or import-alias prefix, an
optional type qualifier, and the final member required by the source position.
Only a role that is fixed by syntax, successful resolution, or one unique
recovery link receives a casing diagnostic. An unresolved or ambiguous
intermediate segment is not guessed from its position or spelling. A leaf whose
role is fixed by syntax or alias kind is validated even when its target is
unresolved. Checking and every language-service operation consume the same
classified segment records.

A source-path-derived module segment has no source token. Its diagnostic uses a
zero-width primary span at the start of the affected source. Casing is checked
once on the user-controlled origin segments before companion, doctest, or other
generated identity transformations add or sanitize text. The structured
details identify `source_path`, `source_kind`, the offending origin `segment`,
and its zero-based `segment_index`. Human output names the origin segment in
the primary message. No consumer invents a source-text range for path text.

| Source kind | Origin segments validated for casing | Synthetic text excluded from validation |
| --- | --- | --- |
| Regular or exported source | Package-relative path after removing `.veln`. | None. |
| Exact `.test.veln` companion | The target source path after removing `.veln`. | The `.test` marker and internal companion suffix. |
| Chained companion | None; the existing chained-companion structural rejection prevents a source-visible module identity. | The complete sanitized recovery identity. |
| Doctest | The documented source's origin module segments. | The `#doctest-...` path suffix and wrapper name. |
| Generated source | Origin module segments supplied by the generating source. | Bookkeeping paths and generated declaration names. |

A generated source without origin module metadata cannot introduce a
source-visible module. The same origin segment sequence supplies diagnostics,
module analysis, documentation, metrics, and language services.

An implicit import alias is validated at its written origin. When the alias is
derived from the final import-path segment, the segment and alias are one name
occurrence and produce at most one `name.invalid_case` diagnostic at that
segment. For example, `use net::HTTP` reports once at `HTTP`. The invalid alias
does not enter the normal import namespace. A future explicit alias with its
own token is a separate name occurrence and is validated at that token.

Every source-written `name.invalid_case` has these structured details:

| Field | Required value |
| --- | --- |
| `phase` | `name` |
| `origin` | `source` |
| `occurrence` | `declaration`, `binding`, `path_segment`, `alias_target`, or `pattern_head` |
| `name` | The exact written spelling. |
| `name_class` | `type`, `constructor`, `module`, `function`, or `value_binding` |
| `required_initial` | `ascii_uppercase` or `ascii_lowercase` |
| `observed_initial` | `ascii_uppercase`, `ascii_lowercase`, `underscore`, or `other` |

A source-path occurrence uses `origin: source_path` and `occurrence:
path_segment`. It has the same class and initial fields plus `source_path`,
`source_kind`, `segment`, and `segment_index`. A written qualified occurrence
also has its zero-based `segment_index` within the written path.

## Resolution Consequences

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

A qualified name pattern remains constructor syntax. Its final segment must be
uppercase. `item::some(x)` and `item::none` each report `name.invalid_case` at
only the final segment, with a message that the constructor name must start
with an ASCII uppercase letter. The invalid head is retained only as a recovery
pattern. Constructor lookup, arity, and exhaustiveness diagnostics for that
head are suppressed, while nested binding patterns and the arm body continue
to be checked. A qualified lowercase pattern is never reinterpreted as a value
binding.

Callability remains a type property. A lowercase binding with a non-function
type still blocks the same-spelled lowercase function according to the current
value-shadowing rule. A local binding is still not visible in its own
initializer when the current specification resolves the initializer to an
outer binding or function. This proposal removes constructor-versus-value
collisions; it does not change value-versus-value shadowing.

Checking, lowering, definition, references, prepare-rename, and rename must use
the same name class. LSP and MCP must not add adapter-specific exceptions.

### Public Alias Targets

An alias kind fixes the class of its target leaf. A public function alias
requires a function leaf. A public type alias requires a type leaf. A public
schema alias uses the casing-neutral schema namespace. A wrong-cased function
or type leaf reports `name.invalid_case` at the leaf token.

Target resolution still reports facts that do not depend on the casing error.
A valid candidate from the wrong namespace also reports the existing
target-kind diagnostic. A genuinely missing target also reports the existing
unresolved diagnostic. An exact, unique recovery symbol of the expected class
suppresses those derivative diagnostics and supplies repair navigation. An
alias with any target failure or recovery target does not enter its normal
export namespace and cannot propagate a recovery symbol.

### Invalid-Name Recovery

An invalidly cased declaration or binding is not inserted into a normal name
class. Analysis retains a quarantined recovery symbol under the original
spelling. A use links to it only when the original spelling matches, the
declaration's intended class is compatible with the syntactic use role, no
valid candidate wins, and exactly one compatible recovery symbol is in scope.
The initial-derived class filter is ignored only for this repair lookup. The
link supports cascade suppression and language-service navigation but does not
make the program valid.

| Invalid declaration | Compatible recovery uses | Incompatible recovery uses |
| --- | --- | --- |
| Function `Build` | `Build()` and a function-alias target. | Type positions and constructor patterns. |
| Constructor `some` | `some()` and qualified constructor pattern `item::some`. | Bare pattern `some`, which declares a binding. |
| Type with a lowercase initial | Type positions and type-alias targets. | Expression and pattern-binding positions. |
| Value binding with an uppercase initial | Value and callable-value positions in its lexical scope. | Type positions and constructor patterns. |

A valid candidate always takes precedence over a recovery symbol. Recovery
symbols do not create ambiguity or cross-class collisions. When multiple
recovery symbols have the same spelling, no one symbol is selected for
navigation. Definition, references, and prepare-rename may expose a unique
recovery link. Rename may use that link only to produce a class-correct repair.
No other operation treats a quarantined symbol as valid. Lowering and backends
never receive recovery symbols.

A recovery symbol is visible only in the declaring source module and in the
lexical scope that the corresponding valid declaration would have occupied. It
does not cross a written import, a public alias, exact-companion privilege, a
dependency boundary, or the implicit prelude import. An invalid public
declaration is absent from public API and package snapshot symbol indexes.
Package and workspace indexes may retain its location for diagnostics, but
downstream lookup and navigation do not expose it.

A structurally valid source path with an invalid module segment produces one
casing diagnostic for every invalid origin segment. The source remains
available for local parse and declaration diagnostics under its source
identity, but its derived identity is not registered as a normal or importable
module. It cannot contribute exports, module duplicates, cycles,
documentation modules, metrics modules, or backend reachability. Imports that
name it receive the ordinary unavailable-module diagnostic. Unrelated valid
modules continue to be analyzed. A structurally invalid path retains its
existing structural module diagnostic and does not also create a module
identity.

Every invalid name reports `name.invalid_case`. Independently provable
diagnostics still accumulate. In particular, declarations of the same kind,
scope, and original spelling still participate in duplicate checking, and an
invalidly cased public alias can also report a target-kind error when that fact
does not depend on the invalid name. Resolution, ambiguity, kind, and
unresolved diagnostics whose only cause is a quarantined symbol are
suppressed.

| Overlap at one occurrence | Required result |
| --- | --- |
| A retained token independently violates parse or module structure and casing. | Report the structural diagnostic, then `name.invalid_case`. A synthetic missing token has no casing diagnostic. |
| The spelling is independently reserved. | Report both reserved-name and casing diagnostics. |
| The original spelling duplicates the same class in the same scope. | Report casing for each invalid occurrence and the existing duplicate diagnostic. |
| An alias target resolves to a valid wrong-kind candidate. | Report casing, then the existing target-kind diagnostic. |
| Valid candidates independently create ambiguity. | Report casing and the existing ambiguity diagnostic. |
| Unresolved, callability, type-origin, constructor-arity, or exhaustiveness failure exists only because of one recovery symbol. | Report casing and suppress the derivative diagnostic. |

Existing related notes remain owned by their original diagnostics. Suppressed
diagnostics do not add explanation fields to successful JSON output; planned
decision-table tests record why each omitted cascade is absent.

Diagnostics have a deterministic order by source identity, primary-span start,
primary-span end, diagnostic priority, and diagnostic id. At the same span,
structural parse or module diagnostics precede `name.invalid_case`, which
precedes duplicate, ambiguity, kind, unresolved, type, and lowering
diagnostics.

### Rename Boundary

Rename validates the requested spelling against the selected declaration's
name class before it produces edits. It does not reinterpret the symbol class
and does not return edits that introduce a casing violation. A quarantined
`fn Build` can therefore be repaired to `build`, but a valid function `parse`
cannot be renamed to `Parse`.

A class-changing request fails with shared code `rename.invalid_case` and
details containing the symbol class, requested name, and required initial
class. LSP maps this domain failure to JSON-RPC invalid params (`-32602`). A
future MCP rename operation returns an MCP tool error with the same code and
details. Neither adapter substitutes a successful empty edit or returns partial
edits.

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

### Source-Less Symbols

`name.invalid_case` validates source-written names. A compiler-provided symbol
that participates in source name lookup must carry an explicit name class and
must have a spelling valid for that class. A violation is a toolchain invariant
failure, not a source diagnostic. Such a symbol enters only its declared name
class and follows that class's existing collision and shadowing rules.

The source-lookup registry construction gate validates every source-visible
compiler descriptor before it publishes an immutable registry. The gate runs
in release builds as well as tests. It validates the descriptor's module
segments, explicit name class, spelling, and lookup key. Failure is atomic: no
partial registry is available to lookup.

An invalid descriptor produces a span-less internal failure with code
`toolchain.invalid_symbol_case` and details containing `provider`, `name`,
`name_class`, and `required_initial`. Command and adapter initialization stop in
their existing internal-failure form. They do not convert the failure into a
source `name.invalid_case`, silently skip the entry, panic only in debug builds,
or wait until lookup to validate it. Generated tables use the same validator at
their generation or build gate without removing the release-time check.

Embedded Veln prelude sources are source-written and are validated normally.
Compiler temporaries and bookkeeping names that cannot participate in source
lookup are outside this contract. A source-less symbol must not use an invalid
spelling as a compatibility exception or reintroduce a cross-class candidate.

## Analysis And Artifact Boundary

Casing uses the selection and reachability boundary already defined by each
command. It does not add a workspace-global gate.

| Consumer | Unit affected by a casing diagnostic | Required outcome |
| --- | --- | --- |
| `check` | The selected analysis set. | Report all selected diagnostics and do not return a successful checked artifact. |
| `run` | Source and module loading for the selected project, then the selected entry's reachable executable closure. | A loading error or reachable casing error produces no executable artifact. An unreachable declaration error follows the existing non-blocking reachable-analysis rule. |
| `test` | The final selected test and doctest suite. | Any selected static casing error blocks the suite before backend compilation; every discovered selected case uses the existing `static_gate` result. |
| `doc` | The selected non-companion documentation set and its selected doctest gates. | Any casing error in that set produces no generated documentation. Excluded companion sources do not enter this gate. |
| Language service | One captured snapshot plus its open-document overlay. | Retain diagnostics and recovery navigation without producing backend artifacts or blocking an unrelated snapshot. |
| Dependency analysis | Dependency sources actually loaded for the selected consumer. | An invalid loaded dependency cannot supply lookup or backend symbols. An unselected dependency or workspace root does not block the consumer. |

No backend receives a declaration, module identity, or recovery symbol with an
invalid case. The planned command fixtures are authoritative for the exact
selection boundary when command behavior differs.

## Goals

- Make type, constructor, module, function, and value-binding names visually
  distinguishable at their declaration and use sites.
- Make bare constructor patterns consistent with constructor calls.
- Remove callable-binding-versus-constructor precedence from valid programs.
- Preserve current visibility, ambiguity, and value-shadowing rules within
  each name class.
- Promote the implemented contract to the current language specification and
  executable specification examples.

## Non-Goals

- Changing which constructors, functions, or bindings are visible.
- Changing duplicate-name rules inside one name class.
- Changing value-versus-value shadowing or initializer visibility.
- Requiring a complete CamelCase or snake_case word convention after the first
  character.
- Renaming schemas, effects, handlers, operations, fields, type parameters, or
  holes as part of this change.
- Defining unrelated MCP or LSP schemas, coordinates, project scope, or
  transport errors beyond the rename failure mapping required here.

## Migration

Implementation must update every repository-owned input that a parser,
analyzer, documentation gate, or language-service case treats as Veln source.
The inventory includes `.veln` files, embedded standard-library sources, Rust
test strings and generated test sources, executable examples, accepted and
rejected fixtures, checked Markdown doctests, editor and agent service source
cases, snapshots, and expected diagnostics, locations, or navigation edits.
Non-executable Veln examples must also use the new contract unless they preserve
an externally defined spelling or explicitly illustrate invalid casing.

In particular, lowercase source ADT variants such as the navigation fixture's
`byte` constructor must receive uppercase constructor names. An unrelated
negative fixture must be migrated so it does not acquire an extra
`name.invalid_case` diagnostic. The only sources that may retain invalid casing
are dedicated identifier-casing rejection cases with an exact expected
diagnostic id, count, message, and span. Migration evidence must audit parsed
and analyzed source carriers rather than relying only on a `.veln` file search.

The change provides no compatibility alias for an invalid old spelling. A
lowercase function such as the standard `byte` helper remains a function and
does not become a constructor.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Check one accepted declaration from every name class. | The source passes casing validation and retains its existing semantic meaning. | Checked source-surface fixture plus focused parser and semantic tests. |
| Check every row of the rejection table. | Each invalid name reports `name.invalid_case` at exactly that name with the class-specific primary message. | Table-driven diagnostic tests and human-output fixtures. |
| Place `_`, `_value`, and `_Type` in expression, declaration, binding, pattern, written-module, and source-path-module positions. | `_` remains a wildcard only in supported positions; underscore-led name occurrences report the required class error; expression holes retain hole behavior; standalone `_` in a required-name position remains structural. | Lexer, parser-recovery, human, JSON, and source-path fixtures. |
| Declare equal-spelled schemas, effects, handlers, operations, types, constructors, functions, and bindings. | Each dedicated source position selects its existing namespace, cross-namespace spellings do not create duplicates, ordinary calls exclude casing-neutral namespaces, and schema composition retains its existing ambiguity. | Namespace-by-use-role decision table with duplicate and definition cases. |
| Check lowercase and uppercase `satisfy` candidates and an uppercase hole label. | The lowercase candidate is accepted, the uppercase candidate reports one exact-span binding diagnostic without an unresolved cascade, and the hole label remains outside the rule. | Semantic, human-output, JSON, and language-service cases. |
| Check qualified uppercase and lowercase constructor patterns. | Uppercase final segments retain constructor behavior. Each lowercase final segment reports one exact-span casing diagnostic without constructor-resolution, arity, or exhaustiveness cascades. | Parser recovery, semantic, lowering, and exhaustiveness cases. |
| Classify every segment of module-only, module-and-type, and prelude-qualified paths with each segment invalid in turn. | Every syntax- or resolution-fixed role receives its class diagnostic; unresolved intermediate roles are not guessed; all language-service operations observe the same decomposition. | Expression, pattern, type, alias-target, definition, reference, and rename decision table. |
| Check function, type, and schema alias targets with independently valid and invalid declaration casing, target casing, and target kind. | Function and type leaves receive their class diagnostic; schema leaves stay casing-neutral; independently provable kind and unresolved failures coexist; recovery targets neither cascade nor export. | Exact ordered human and JSON alias-target table plus navigation cases. |
| Derive module identities from every source kind with one or more invalid origin segments. | Every invalid user-controlled segment reports one source-start diagnostic with the required origin details; synthetic segments never report casing; a chained companion reports only its existing structural failure. | Regular, exact-companion, chained-companion, doctest, generated, export, human, JSON, and LSP source-kind table. |
| Analyze an invalid derived module beside imports, duplicates, cycles, documentation, and metrics. | All invalid origin segments are reported; the source receives local diagnostics but no importable graph identity or emitted artifact; unrelated graph analysis continues. | Multi-segment module, import, duplicate, cycle, documentation, and metrics cases. |
| Import a path whose final segment is also its implicit alias. | An invalid final segment produces one diagnostic owned by that segment, not separate path and alias diagnostics. | Single- and multi-segment import cases. |
| Observe name ranges through every diagnostic and language-service consumer. | Parser-retained token spans, human and JSON spans, definition, references, prepare-rename, and rename ranges agree for each written name segment. | CRLF, preceding Unicode, multiline, recovery, and qualified-path fixtures. |
| Resolve uses near invalid declarations in every compatible and incompatible use role. | A unique class-compatible quarantined symbol suppresses only derivative cascades and supports repair navigation; valid candidates win; bare binding patterns do not become constructors; multiple candidates do not create arbitrary navigation. | Recovery decision table for functions, constructors, types, and bindings across bare and qualified expressions and patterns. |
| Cross a same-module, import, alias, exact-companion, dependency, and prelude boundary with an invalid declaration. | Recovery navigation exists only in the declaring source and lexical scope. No recovery symbol is imported, aliased, snapshotted, or lowered. | Boundary table covering diagnostics, definition, references, and artifacts. |
| Combine casing with structural, reserved-name, duplicate, ambiguity, target-kind, and unresolved failures. | Every direct and independently provable error appears once in the defined order with the required details and unchanged related notes; recovery-derived cascades do not appear. | Exact ordered human and JSON overlap tables, including an asserted reason for every expected absence. |
| Request valid, class-changing, conflicting, and invalid-declaration repair renames. | Class-preserving and repair renames return complete linked edits. Class-changing requests return `rename.invalid_case`; predictable collisions return `rename.conflict`; failures return no edits. Path-derived module segments return no prepare range or file edits. | Shared language-service, LSP error-mapping, and planned MCP error-mapping cases. |
| Register valid and invalid source-less lookup descriptors. | The release-mode registry gate either publishes one complete validated registry or returns `toolchain.invalid_symbol_case`; invalid descriptors never reach lookup, while internal names remain outside the gate. | Generated-table, injected-descriptor, release-mode, atomic-failure, and lookup-isolation tests. |
| Run each command with casing errors inside and outside its selected unit. | `check`, `run`, `test`, `doc`, language-service overlays, and dependency analysis follow the command boundary table and never send an invalid symbol to a backend. | Command fixtures covering reachable and unreachable sources, selected and unselected tests, documentation exclusions, overlays, and loaded and unloaded dependencies. |
| Use uppercase constructors and lowercase bindings in bare and qualified expressions and patterns. | Expressions, lowering targets, pattern classification, and exhaustiveness agree on the name class. | Semantic and lowering tests plus checked ADT expression and pattern examples. |
| Attempt a former same-spelled callable-binding and constructor case. | One declaration is rejected by casing; no accepted source reaches a precedence decision between the two candidates. | Negative semantic fixture covering callable and non-callable local bindings. |
| Navigate accepted function, binding, type, and constructor uses. | The language service selects only the symbol class fixed by the initial letter. | Definition, reference, and rename cases in `veln-language-service`. |
| Run the repository source-carrier audit and specification suite after migration. | Every parsed or analyzed repository-owned source follows the contract except dedicated exact-expectation casing fixtures, and unrelated negative fixtures retain their intended diagnostic sets. | Source-carrier audit, specification harness, doctest and documentation gates, and workspace tests. |

This proposal is complete when all acceptance rows pass, all repository-owned
Veln sources follow the naming contract, and the implemented behavior is
stated under `docs/specification/` and routed to checked examples under
`examples/specification/`.
