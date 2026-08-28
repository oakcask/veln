---
role: specification
authority: normative
update-when: The Veln source name resolution contract, source identifier casing diagnostics, or executable name-resolution evidence changes.
---

# Name Resolution

This page specifies source name resolution and identifier casing behavior.

Implemented checker namespaces are:

- module imports
- value declarations, including functions, parameters, and `let` bindings
- record fields inside one record literal

Bare names resolve to local bindings. Function calls resolve to:

- compiler-known stdio calls
- local bindings with function type
- declarations in the current source module by bare name
- unambiguous public function exports from written imports by bare name
- discovered function signatures through a `use` alias in `alias::function`
  form
- source path derived local imports through their full written module path in
  `module::path::function` form
- public function aliases through the declaring module path
- implicit standard prelude helper imports by bare name or `prelude::function`
  form

Unresolved values and call targets produce `name.unresolved` diagnostics. A
qualified call does not fall back to a bare function with the same final
segment when no matching import alias exists.
When more than one import provides the same bare function name, including a
conflict between a written import and the implicit prelude import, the checker
reports `name.ambiguous` at the bare name and lists qualified spellings in
related notes.
Duplicate declarations in the same implemented namespace produce
`name.duplicate` diagnostics at the later declaration, with the first
declaration reported as related context.

Local value bindings and declarations in the current source module shadow
imported names for both bare values and calls. The standard prelude remains
available through `prelude::` when a local declaration shadows its bare name.
The `StreamInput` standard ADT constructors are available as `Chunk(bytes)`,
`End`, `StreamInput::Chunk(bytes)`, `StreamInput::End`,
`prelude::Chunk(bytes)`, `prelude::End`,
`prelude::StreamInput::Chunk(bytes)`, and `prelude::StreamInput::End`.
The `StreamAdapterAction` standard ADT constructors are available through the
same bare, type-qualified, prelude-qualified, and prelude-type-qualified
forms.
The `AcceptOutcome` standard ADT constructors are available through the same
bare, type-qualified, prelude-qualified, and prelude-type-qualified forms.

A wildcard let target, `_`, evaluates its expression without declaring a local
name. It can be annotated for type checking, but it is never a resolvable
binding.

Invalid source-written type, constructor, function, public alias, and value
binding names are quarantined from normal lookup and artifacts. A use may
recover through one same-source invalid declaration or binding only when no
valid candidate wins, the use role is compatible with the invalid name class,
and the call arity is compatible for callable recovery. Cross-class recovery
collisions select no recovery record and preserve the ordinary unresolved or
ambiguous fact.
Every written import path segment is a module-class path segment. Each segment
must start with an ASCII lowercase letter. An uppercase-led or underscore-led
segment reports `name.invalid_case` at the exact segment token span with
occurrence `path_segment`, name class `module`, required initial
`ascii_lowercase`, the observed initial class, and the zero-based
`segment_index` inside the written import path. When the final segment would
also provide the implicit import alias, the path segment and alias are one
occurrence and produce at most one casing diagnostic. An import with an
invalid module path segment does not enter normal import lookup for value,
call, type, constructor, schema, effect, handler, inference, lowering, or
reachability consumers. The original written import still participates in
duplicate import-alias analysis. If no selected source derives the written
local module path, `module.unresolved_import` is still reported. A use through
the invalid implicit alias may suppress a derivative unresolved, type-origin,
constructor-arity, or exhaustiveness diagnostic only when one matching
selected source export proves that quarantine is the sole failure. This
same quarantine proof suppresses derivative unknown-effect and
unknown-handler diagnostics for public effect and handler exports. Missing
target modules, missing exports, private targets, and wrong-kind targets
remain independently reported.
The
`identifier-casing-import-path-json` and
`identifier-casing-import-path-human` examples check multi-segment and
single-segment import paths, uppercase and underscore initials, exact spans,
detail fields, and the unresolved-module boundary when the selected source
derives a different lowercase module path. The
`identifier-casing-import-missing-module-overlap-json`,
`identifier-casing-import-duplicate-overlap-json`, and
`identifier-casing-import-alias-cascade-boundary-json` examples check the
overlap with missing-module, duplicate-alias, and function alias-use
diagnostics. The
`identifier-casing-import-type-cascade-boundary-json`,
`identifier-casing-import-constructor-cascade-boundary-json`,
`identifier-casing-import-missing-type-control-json`,
`identifier-casing-import-missing-type-export-json`, and
`identifier-casing-import-missing-constructor-control-json` examples check
that qualified imported types and constructors keep independently provable
type, call-target, missing-target, and missing-export failures. The
`identifier-casing-import-schema-cascade-boundary-json` and
`identifier-casing-import-private-schema-boundary-json` examples check the
same boundary for schema composition: schema targets that are missing because
the invalid module path has no matching selected module remain independently
reported. The
`identifier-casing-import-effect-cascade-boundary-json` and
`identifier-casing-import-handler-cascade-boundary-json` examples check the
same independently provable boundary for effect and handler consumers. The
`identifier-casing-import-order-json` example checks source-ordering between
an invalid import path segment and a later invalid declaration. The
`identifier-casing-import-alias-run-boundary-json` example checks the same
invalid implicit-alias boundary for `run` reachability. The
`identifier-casing-unselected-import-path-json` and
`identifier-casing-unused-import-path-json` examples check that `run` does not
promote invalid written import paths outside the selected entry closure or
unused by that closure.
Valid implicit standard prelude symbols remain normal lookup candidates. A
same-spelled application recovery record does not shadow the valid prelude
symbol for a function call or constructor path, and does not enter
prelude-qualified lookup.
Qualified constructor patterns keep constructor syntax. A qualified
constructor pattern whose final segment starts with an ASCII lowercase letter
reports `name.invalid_case` at that final segment with occurrence
`path_segment`, name class `constructor`, required initial `ascii_uppercase`,
observed initial `ascii_lowercase`, and the segment index inside the written
path. That invalid head remains only as a recovery constructor pattern.
Constructor resolution, constructor-pattern type mismatch, and match
exhaustiveness diagnostics whose only cause is that invalid head are
suppressed. An exhaustiveness diagnostic is suppressed only for the constructor
found by changing the invalid final segment's first ASCII lowercase letter to
uppercase and resolving the resulting path through ordinary case-sensitive
constructor lookup. A different constructor spelling that remains unresolved
after that initial-only repair is not treated as covered. Nested pattern
bindings and the match-arm body are still checked. The
`identifier-casing-qualified-constructor-pattern-json`,
`identifier-casing-qualified-constructor-pattern-human`, and
`identifier-casing-qualified-constructor-pattern-over-suppression-json`
examples check the diagnostic shape, suppressed cascades, and exhaustiveness
over-suppression boundary. The
`identifier-casing-qualified-constructor-pattern-direct-diagnostics-json`
example checks that nested binding patterns and the match-arm body are still
checked while head-derived cascades are suppressed. The
`identifier-casing-qualified-constructor-pattern-type-mismatch-json` example
checks that an independently provable constructor-pattern type mismatch is not
suppressed.

Compiler-provided symbols that participate in source lookup are specified by
[source-less-lookup.md](source-less-lookup.md). Embedded Veln prelude sources
remain source-written and continue to use ordinary source casing diagnostics.

Current duplicate checks reject:

- duplicate import paths within the same source module
- duplicate top-level function, test, or public function alias names
- duplicate top-level source type or public type alias names
- duplicate parameter names in one function
- a result binding that duplicates a parameter name
- duplicate `let` names in the same function value scope, including names that
  duplicate parameters
- duplicate field names in one record literal
- duplicate pattern binding names in one match arm, including names that
  duplicate bindings already visible at the arm
- duplicate field names in one record pattern

Record type annotations also require unique field names. Duplicate record type
fields are reported through invalid type annotation diagnostics because they are
part of annotation parsing rather than value-name resolution.

For selected package-relative sources, the command analysis path derives local
module identity from the source path before semantic checks run. Written
imports are scoped to the source module that declares them. Bare public imports
and qualified module paths from another same-package module are visible only in
that declaring source module. User source cannot derive module identity
`prelude` or write an import path whose alias is `prelude`; both names are
reserved for the implicit standard prelude import and report `name.reserved`.

Each source-path-derived module segment is a module-class path segment and
must start with an ASCII lowercase letter. Regular source paths validate the
package-relative path after removing `.veln`. Exact `.test.veln` companions
validate the target source path before the internal companion suffix is added.
Doctests validate the documented source path before the doctest suffix and
wrapper name are added. Generated sources validate the origin module segments
supplied by the generating source before generated bookkeeping paths or
declaration names are considered. Generated sources without origin module
metadata do not introduce a source-visible module. Chained companions do not
derive a source-visible module identity and keep the existing
`module.chained_companion` diagnostic instead of source-path casing
diagnostics. Every invalid user-controlled origin segment reports one
zero-width `name.invalid_case` diagnostic at the start of the affected source
with `origin: source_path`, `occurrence: path_segment`, `name`,
`name_class: module`, `required_initial: ascii_lowercase`,
`observed_initial`, `source_path`, `source_kind`, `segment`, and the
zero-based `segment_index`. The `source_kind` value is `regular`,
`export`, `companion`, `doctest`, or `generated`. A source with an invalid
origin segment is not registered as a normal derived module identity.
Diagnostic-tolerant source analysis treats that absent identity as absent for
local import resolution, duplicate source-path detection, and test dependency
graph edges. The invalid source still reports its source-path casing
diagnostic, and unrelated valid modules continue to produce their own source
and semantic diagnostics.
Source-path casing validation still runs for selected regular and companion
sources that also have parse diagnostics, but those parse-failing sources are
not lowered or registered as normal derived module identities. A source path
segment that starts with an ASCII lowercase letter but contains another
invalid module-identifier character reports `module.invalid_source_path`
instead of `name.invalid_case`.
Manifest export path checks use the same accepted module derivation boundary.
A selected source that is also named by `lib.exports` is classified once for
source-path casing diagnostics. A regular selected source uses
`source_kind: export`. A generated selected source still uses its generated
origin module metadata as the identity and casing authority, with
`source_kind: generated`; its generated bookkeeping path is not validated or
published as the exported module identity. The same invalid origin segment is
not reported again as a regular source diagnostic. Export origin casing
failures remain source-path diagnostics instead of generic manifest export
errors. The checked
`identifier-casing-source-path-json`,
`identifier-casing-exported-source-path-json`,
`identifier-casing-source-path-import-isolation-json`,
`identifier-casing-source-path-duplicate-isolation-json`,
`identifier-casing-source-path-human`,
`identifier-casing-chained-companion-boundary-json`, and
`identifier-casing-source-path-boundary` examples fix JSON, human, and LSP
diagnostic spans, details, and graph-isolation boundaries. Focused
`veln-test` unit coverage fixes the test dependency graph boundary for imports
declared by a source without a normal module identity.

External `use path from "package"` declarations resolve `path` inside an
already available direct `path`, `vendor`, `mirror`, or locally materialized
`git` dependency whose dependency table key is `package`. For a `git`
dependency, an accepted `subdir` selects the dependency package root below the
available repository tree. The dependency manifest's `[package].name` must
match that package identity, and external modules are importable only when
their derived source module path is listed by the dependency package's
`[lib].exports`. The import exposes only public declarations and public
aliases from that exported module; private names remain private even when the
dependency source is loaded for analysis.

Editor-facing type-role selection uses the same module and import visibility.
A bare type-role reference selects the same-module source type first. Without
a same-module type, it selects one visible public imported type only when the
type identity is unique. If multiple visible imports provide the same type
leaf, the bare reference has no selected language-service symbol. A qualified
type-role reference selects only the visible type identity owned by the written
qualifier.

When `veln.toml` contains manifest export data, `[modules]` is rejected and
`[lib].exports` is checked as a list of public package-relative source files.
Export entries must be selected source files, must use `.veln` file-path
spelling instead of module paths, must stay inside the package, and must derive
unique source module paths.

Named holes remain repair labels, not value declarations. Reusing a hole label
does not affect name resolution.
