---
role: specification
authority: normative
update-when: The Veln source name resolution contract or source identifier casing diagnostics change.
specification-coverage: usage=#representative-usage; behavior=#resolution-rules; limits=#limits
---

# Name Resolution

## Representative usage

An imported qualified path is resolved through the written module name:

```veln
use math

test uses_target() -> ()
	let value = math::increment(1)
	()
end
```

An exact `math.test.veln` companion may use the same explicit import to call a
private target function. That permission is target-specific and non-transitive;
an unrelated companion or a different target remains unresolved.

This page specifies source name resolution and identifier casing behavior.

## Resolution Rules

### Namespaces and use roles

Implemented checker namespaces are:

- module imports
- schema declarations and public schema aliases
- effect declarations
- handler declarations
- effect operation declarations
- source type declarations and public type aliases
- source ADT constructor declarations
- value declarations, including functions, parameters, and `let` bindings
- record fields inside one record literal

Equal spellings in different namespaces are accepted where the source grammar
permits them. A source position selects only the namespace fixed by that
position. Type annotations select the type namespace even when a schema has the
same spelling. Schema encode and decode expressions select the schema
namespace. Effect lists and `perform Effect::operation(...)` forms select
effect and operation namespaces. `handle Body with handler(...)` forms select
the handler namespace. Ordinary value calls do not select schema, effect,
handler, or operation declarations. Schema composition remains ambiguous when
both a visible ordinary type and a visible schema use the same spelling because
that position admits both namespaces. Namespace selection is therefore fixed
by the syntactic role, while same-namespace duplicates remain errors and
unrelated namespaces may share a spelling.

### Schema navigation

For saved composition-and-operation reference navigation, a public schema
alias in a retained direct dependency or the standard library is eligible when
each hop resolves to exactly one public schema alias or, at the final hop, one
public schema declared by an exported source in the retained package. A bare
target resolves in the alias
module. A qualified target resolves through a valid local import written in the
alias's module. Imports from all retained package sources with that explicit
module identity participate, including full module paths and unique implicit
leaf aliases. The imported target can resolve to another module or back to the
alias's own module.
Consumer imports do not participate in target resolution. At each hop, the
declaration kind must match the expected kind: a non-terminal target is one
public schema alias, and the final target is one public schema. Same-spelled
declarations in unrelated namespaces do not affect eligibility. A finite,
acyclic chain of public schema aliases in the same retained package is eligible
when every hop and the terminal schema are in exported sources.
External-package targets, ambiguous imports, and recovered imports remain
ineligible. Alias chains must be finite and acyclic; a cyclic chain is rejected.
Missing, ambiguous, invalid-cased, private, non-exported, wrong-kind, or
syntax-recovered hops also make the package alias ineligible. Selecting an
ineligible alias produces an empty reference result and does not fall back to a
same-spelled schema.

Schema-alias composition-and-operation reference lookup combines written
imports from all owned sources with the same explicit workspace module
identity. A valid package import in one such source can qualify a
composition or operation leaf in another.
A colliding workspace import and package import, duplicate package imports, or
a syntax-recovered package import in that module prevents the qualified leaf
from selecting a package schema alias or falling back to a
schema imported only by the leaf's source. The positive cross-source path
keeps one alias identity; collision, duplicate, and recovered-import paths
remain unresolved. Direct, `Repeat`, and array-payload leaves do not select an
ineligible alias or enter an eligible alias reference set.

A multi-segment schema composition or operation target resolves through either
the full written import module path or its implicit leaf alias. An exact full
import path takes precedence over a same-spelled implicit leaf alias. Without
an exact match, the leaf alias resolves only when exactly one valid workspace
or package import provides it. Conflicting exact imports and colliding implicit
leaf aliases remain unresolved in either import order. Duplicate and
syntax-recovered dependency imports do not provide dependency schema
visibility. A clean or syntax-recovered package schema alias with
the selected name prevents a same-spelled package schema from acting as a
fallback composition target. A `Repeat` or array payload participates in
schema composition lookup when its count is a valid schema count expression.
This includes member paths accepted by schema analysis. Comments and strings
do not participate in schema composition lookup. These rules preserve
workspace and direct-dependency alias identity across sources. Retained
standard-library schema
targets use the same full-path and unique implicit-leaf resolution, exact-path
precedence, import-collision, repeated-count, and lexical-exclusion rules.
Their package origin keeps them distinct from same-spelled workspace and
direct-dependency schemas. Eligible standard-library schema aliases are
separate selection targets and block fallback to a same-spelled schema. Their
package origin stays distinct from workspace and dependency origins. An
eligible alias in the standard-library `prelude` module is also visible by its
bare name or an explicit `prelude::` qualifier for composition and operation
leaves. For a bare name, a same-named local schema, schema alias, type, or type
alias blocks implicit prelude fallback in composition. In `decode` and
`encode`, which select only the schema namespace, a local schema or schema
alias blocks fallback while a local type or type alias does not. An ineligible
declaration in the applicable namespace also blocks fallback. A same-named
local declaration does not block an explicit `prelude::` qualifier.
Standard-library package-source occurrences are not selectable. Lexical noise,
malformed repeats, invalid casing, and ineligible aliases do not enter a
reference set.

### Value calls and shadowing

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

### Casing and recovery

Invalid source-written type, constructor, function, public alias, and value
binding names are quarantined from normal lookup and artifacts. A use may
recover through one same-source invalid declaration or binding only when no
valid candidate wins, the use role is compatible with the invalid name class,
and the call arity is compatible for callable recovery. Cross-class recovery
collisions select no recovery record and preserve the ordinary unresolved or
ambiguous fact.
Source-written `mod` headers remain unsupported as package module identities,
but a parse-clean header whose name starts with an ASCII uppercase letter or
underscore reports `name.invalid_case` at the exact header-name token. The
diagnostic has occurrence `declaration`, name class `module`, required initial
`ascii_lowercase`, and the observed initial class. That invalid header does
not supply a normal module identity for declarations or checked artifacts. A
lowercase source-written header does not report `name.invalid_case`; it still
uses the source `mod` unsupported-module diagnostic boundary. Lowercase
headers retain that unsupported-module diagnostic without adding a casing
diagnostic.
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
remain independently reported. This quarantine applies consistently to value,
type, constructor, schema, effect, handler, and reachability consumers; an
invalid import is never promoted merely because another use mentions it.
Valid implicit standard prelude symbols remain normal lookup candidates. A
same-spelled application recovery record does not shadow the valid prelude
symbol for a function call or constructor path, and does not enter
prelude-qualified lookup.
Qualified use paths validate every segment whose role is fixed by syntax,
successful resolution, or one unique recovery link. Module-only function and
value paths validate each resolved or recovered qualifier segment as `module`
and the final segment as `function` for calls or `value_binding` for value
references. Module-and-type constructor paths validate resolved or recovered
qualifier segments as `module`, the type qualifier as `type`, and the final
segment as `constructor`. Prelude-qualified function calls validate `prelude`
as the module segment and the final segment as `function`. Prelude-qualified
type paths validate `prelude` as the module segment and the final type segment
as `type`; prelude-qualified constructor paths validate `prelude`, the type
qualifier, and the constructor segment with the same module, type, and
constructor roles. Qualified type paths in function parameters, function
returns, local annotations, handler parameters, handler operation parameter
types, effect operation parameter and return types, ADT positional payload
fields, ADT record payload fields, and schema fields use the same segment
records. Qualified nominal effect paths inside function type
`effects [...]` annotations are effect paths, not qualified type paths, and do
not produce source identifier casing diagnostics. An unresolved or
ambiguous intermediate segment is not assigned a role from spelling alone.
Each invalid role-fixed segment reports `name.invalid_case` at the exact
segment token span with occurrence `path_segment` and the zero-based
`segment_index`. A call-target diagnostic whose only cause is the resolved or
uniquely recovered invalid segment that owns that use is suppressed. Missing
targets, private imported targets, and recovery links that would cross an
import boundary still report `name.unresolved`. These role records apply to
expression, pattern, handler, declaration, ADT, effect, and schema carriers;
an unresolved intermediate segment receives no guessed casing role.
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
bindings and the match-arm body are still checked. An independently provable
constructor-pattern type mismatch is not suppressed by this recovery rule.

Compiler-provided symbols that participate in source lookup are specified by
[source-less-lookup.md](source-less-lookup.md). Embedded Veln prelude sources
remain source-written and continue to use ordinary source casing diagnostics.

### Duplicate declarations

Current duplicate checks reject:

- duplicate import paths within the same source module
- duplicate schema declaration names in the same source module
- duplicate effect declaration names in the same source module
- duplicate handler declaration names in the same source module
- duplicate operation names inside one effect declaration
- duplicate top-level function, test, or public function alias names
- duplicate top-level source type or public type alias names
- duplicate constructor names inside one source type declaration
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

### Source-path identities

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
Source-path casing validation still runs for selected regular and companion
sources that also have parse diagnostics, but those parse-failing sources are
not lowered or registered as normal derived module identities. A rejected
derived-module identity is recorded only when visible module derivation itself
fails solely with source-path `name.invalid_case` diagnostics. A parse-failing
source whose source path casing is accepted does not record a rejected
derived-module identity and does not suppress a single-segment unresolved
local import. A source path segment that starts with an ASCII lowercase letter
but contains another invalid module-identifier character reports
`module.invalid_source_path` instead of `name.invalid_case`.
Manifest export path checks use the same accepted module derivation boundary.
A selected source that is also named by `lib.exports` is classified once for
source-path casing diagnostics. A regular selected source uses
`source_kind: export`. A generated selected source still uses its generated
origin module metadata as the identity and casing authority, with
`source_kind: generated`; its generated bookkeeping path is not validated or
published as the exported module identity. The same invalid origin segment is
not reported again as a regular source diagnostic. Export origin casing
failures remain source-path diagnostics instead of generic manifest export
errors. In a direct dependency, an invalid-cased exported source path does not
contribute a normal public module identity and does not satisfy imports or
qualified uses through dependency recovery. Other valid sibling exports in
the same dependency remain importable and analyzable in the same invocation.
The invalid identity cannot satisfy an import, duplicate module, or
reachability edge while unrelated valid modules continue semantic analysis.

### Dependencies and exports

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
type-role reference selects only the visible type declaration or supported
direct-dependency public type-alias identity owned by the written qualifier.
The alias identity remains separate from the target type identity.
Schema operation selection uses the schema namespace. A bare schema path in a
`decode` or `encode` expression selects a same-module schema declaration, or an
eligible implicit standard-library schema alias when no same-spelled local
schema or alias blocks that fallback. A written import does not expose the
imported module's schemas to bare schema operation paths. Imported schemas are
selectable through accepted qualified schema paths, including
import-alias-qualified paths. An ineligible same-module public schema alias
blocks fallback to a same-spelled schema declaration.

When `veln.toml` contains manifest export data, `[modules]` is rejected and
`[lib].exports` is checked as a list of public package-relative source files.
Export entries must be selected source files, must use `.veln` file-path
spelling instead of module paths, must stay inside the package, and must derive
unique source module paths.

Named holes remain repair labels, not value declarations. Reusing a hole label
does not affect name resolution.

## Limits

Resolution is case-sensitive after the role-specific casing check. Recovery is
limited to one same-source declaration or binding, never crosses an import
boundary, and cannot override a valid candidate. Invalid module identities are
quarantined from lookup and reachability; valid sibling sources remain usable.
Schema alias navigation is finite and acyclic: cyclic chains are rejected,
while finite direct-dependency chains may be eligible when every hop and its
exported terminal satisfy the rules above. Ambiguous or recovered imports do
not become fallback targets.

## References

- Name and import analysis: `crates/veln-analysis/src/surface/` and
  `crates/veln-sema/src/name_recovery.rs`.
- Casing and source-path diagnostics: `crates/veln-sema/src/pipeline/identifier_casing/`.
- Source-less descriptors and lookup routes: [source-less-lookup.md](source-less-lookup.md).
