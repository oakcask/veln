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
- value declarations, including functions, parameters, and local `let` bindings
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

### Explicit import aliases

`use a::b as local` binds `local` in the importing module's import namespace.
`local::member` and the full written path `a::b::member` select the same target
module identity. The declaration contributes neither the implicit `b`
qualifier nor bare exported members. This remains true when the explicit alias
equals `b`. Imports without `as` retain their implicit qualifier and bare
exports. An unaliased standard-package import also retains its canonical
`std::`-qualified path in addition to its written relative path. Import aliases
are not values and do not leak through another module.

An exact full written path is selected before a local qualifier. An ambiguous
exact path is unresolved and does not fall through. A qualifier must select
exactly one eligible import. Two implicit leaf qualifiers can remain
declaration-valid but are ambiguous at lookup. If either colliding qualifier is
explicit, the later qualifier reports `name.duplicate` and neither binding is
eligible through that qualifier. A non-colliding exact written path remains
eligible when only the qualifier collides. Repeating one origin and written
target path reports a duplicate import even when the aliases differ, and both
imports are quarantined. The same written path from distinct package origins
is ambiguous in its exact form but remains available through distinct aliases.
Import selection chooses the module before member lookup. A selected module
that lacks the requested member does not redirect lookup to a different module
whose alias or implicit qualifier has the same spelling.

An explicit alias named `prelude` reports `name.reserved`. An uppercase or
underscore-led alias reports `name.invalid_case` at the alias token as a
module-class declaration. Invalid target casing, invalid alias casing, syntax
recovery, and duplicate target imports quarantine the complete import. They
supply no qualifier, full-path, bare-export, or reachable-code candidate.
Invalid-cased source imports still participate in explicit qualifier and
duplicate-target checks, so removing an invalid candidate cannot make a
colliding qualifier resolve. A syntax-recovered import participates in neither
check and cannot quarantine a later valid import. Independent target,
collision, and casing diagnostics remain visible. Casing recovery suppresses
a derivative member diagnostic only when one visible target export proves
that casing is the sole failure. Missing, private, wrong-kind, ambiguous, and
colliding targets do not gain normal lookup or executable readiness through
recovery.

The selected import retains its package and module identity for values and
calls, types and refinements, constructors and patterns, schemas, effects,
handlers, lowering, and reachability. Existing visibility rules still apply:
an alias does not expose a private package declaration, extend exact companion
access, or make an import transitive. Public function, type, and schema aliases
may select an explicit import in their own declaring module, but the local
import qualifier does not become part of the published identity.

A typed module `let` enters the declaring module's value namespace with the
identity of its module and declared name. Its written annotation supplies the
reference type. A same-module reference can use the bare name, the complete
declaring module path, or the declaring module path's final segment as its only
qualifier. Any longer qualifier must equal the complete declaring module path;
an added prefix does not resolve. An exact written import that matches a
qualified spelling takes precedence over same-module leaf shortening. Failure
to find the requested imported member does not fall back to the local module
value. These references resolve throughout the module, including before the
declaration. A parameter or local binding wins over a same-spelled module value
for a bare reference. Functions, public function aliases, and module values
share one duplicate-name check.
A collision makes every module-value declaration with that identity
unavailable, and declarations that depend on it are unavailable while
unrelated module values retain their semantic facts. A module-value declaration
with invalid casing is unavailable under the same dependency rule. An
unavailable same-module identity still reserves its name. A value read from
that identity has no type fact, and a call does not select a declaration, emit
`type.not_callable`, or fall through to another callable with the same name.
Call arguments are still checked independently. A selected available module
value used as a call target reports `type.not_callable` with that declaration's
context and does not fall through to callable lookup. Only a missing identity
continues through ordinary value or call resolution. Imported lookup and
visibility for module values are not implemented.

A public function alias cannot select an available module value. A bare alias
target selects an available value in the alias's module. A qualified alias
target selects an available public value only when a valid written import
identifies one module. Either selection reports `name.kind_mismatch` with
`expected_kind: "function"` and `actual_kind: "module_value"`. A missing,
ambiguous, invalid, quarantined, private imported, or otherwise unavailable
target continues to report `name.unresolved`. This alias-target classification
does not make imported module values available to expressions or calls.
Checked human and JSON command cases cover the same-module and written-import
forms.

Explicit type arguments on a bare or qualified call target do not change that
selection. A visible local binding still takes precedence. Otherwise an
available module value is selected as the non-callable declaration, and an
unavailable identity remains reserved without exposing another callable. The
checked [`typed call`](../../examples/specification/check/module-value-typed-call-json/)
and [`unavailable typed call`](../../examples/specification/check/module-value-unavailable-typed-call-json/)
cases cover both outcomes.

A finite chain of public type aliases is a valid type-alias target when it
terminates in a resolved source ADT. Every alias in the chain retains the
terminal ADT's nominal identity while remaining a selectable type name. A
direct function target reports `name.kind_mismatch`. A target that does not
resolve to a type reports `name.unresolved`. The
[type compatibility rules](types.md#compatibility-and-limits) define how a
resolved chain qualifies variant refinements and preserves display provenance.

A qualified target in a public type-alias declaration resolves through imports
declared in that alias's source module. An exact written import path takes
precedence over a same-spelled local qualifier. Without an exact match, an
explicit alias or implicit leaf qualifier must identify exactly one eligible
imported module before the checker looks for the target type in that module.
Colliding qualifiers therefore remain unresolved even when only one candidate
module declares the requested type leaf. Reordering imports or selected source
modules does not change these results.

A qualified variant-refinement base first selects a matching source module
identity. When that source type has the same leaf spelling as a compiler-known
type, the source declaration wins. The compiler-known identity remains the
fallback only when the qualified path does not identify a source declaration.
This precedence applies to finite ADTs and to source declarations without
variants; it does not turn the latter into an opaque compiler-known type.

Source identity selection precedes generic-arity and descriptor-uniqueness
checks. A current-module ADT or another imported ADT with the same type leaf
cannot satisfy a different written module or import qualifier, regardless of
descriptor registration order. An exact written import path takes precedence
over a same-spelled local qualifier. Without an exact match, colliding local
qualifiers remain unresolved even when only one imported module declares the
requested type leaf. If the selected identity has the wrong arity or has
duplicate matching descriptors, lookup fails instead of falling through to the
namesake. An unqualified type leaf continues to prefer the current-module ADT
over imported namesakes. A public ADT from another module participates in
unqualified type lookup only when an unaliased `use` declared in the current
module names its module. An explicitly aliased import supplies no bare type
candidate. Imports declared in other modules neither supply a candidate nor
make the current lookup ambiguous. This module-local import boundary applies
both when lookup checks an exact generic arity and when recovery accepts any
arity.

Each public type-alias declaration resolves its own target, including when
another declaration in the same module has the same alias name. The later
declaration reports `name.duplicate`. An unresolved or wrong-kind target also
reports its target diagnostic at the invalid declaration, regardless of which
duplicate is written first. A valid duplicate does not make the invalid
declaration resolve.

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
leaves. The explicit qualifier uses a standard-library alias only when
`prelude` does not resolve as a written import. An exact workspace import named
`prelude` selects the workspace alias, and an exact package import named
`prelude` selects that package alias. If both imports provide the exact name,
the qualifier remains unresolved in either import order and does not fall back
to the standard library. These rules apply consistently to direct, `Repeat`,
array-payload, `decode`, and `encode` leaves. Written `prelude` imports still
report `name.reserved`; a parse-clean reserved import participates in
navigation precedence, while a syntax-recovered import does not. For a bare
name, a same-named local schema, schema alias, type, or type alias blocks
implicit prelude fallback in composition. In `decode` and
`encode`, which select only the schema namespace, a local schema or schema
alias blocks fallback while a local type or type alias does not. An ineligible
declaration in the applicable namespace also blocks fallback. This includes a
syntax-recovered local schema alias whose target is missing. A same-named local
declaration does not block an explicit `prelude::` qualifier.
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
- public function aliases whose target has a published callable signature,
  through the declaring module path
- implicit standard prelude helper imports by bare name or `prelude::function`
  form

A public function whose variant-refinement visibility check fails anywhere in
a parameter or result can retain an all-`unknown` signature for same-module
recovery without publishing a callable signature. A public source ADT whose
public payload fails the same check retains its unambiguous same-module source
identity without publishing an imported public type fact. Publication closure
then excludes every public ADT whose public payload contains a resolved named
reference to a quarantined ADT. The reference can be an ordinary named type, a
refinement base, a named type argument, or a dependency reached through a
transparent alias. The closure is independent of declaration order and
terminates for recursive dependency cycles. A public function whose parameter
or result contains an ADT in that closure also retains same-module recovery
state without publishing a callable signature.
Public type aliases whose terminal target belongs to the closure are also
excluded from imported public type facts. Valid public ADTs outside that
dependency closure remain importable. These rules include
a written base that directly names its ADT or resolves through a finite
transparent type-alias chain. A public alias of an unpublished function is not a
qualified call target or an unqualified import candidate, and another module
cannot republish it through a further alias. An alias of a valid public function
remains available through each supported alias chain. This publication rule
does not change invalid-name function recovery, whose callable-signature
behavior is defined by the
[type-checking contract](types.md#compatibility-and-limits).

When a public alias uses a bare target name, a same-module function shadows an
imported function with that name before publication eligibility is checked. If
the local function is unpublished because its signature refinement visibility
check failed, the alias is also unpublished; resolution does not fall back to
the imported namesake. The imported function remains available through its
qualified module path.

After `use net from "std"`, `net::connect` selects the exported `std::net`
function declaration. Its source signature, nominal `net::IO` effect, and
package definition location are authoritative. There is no compiler-known
public network fallback.

Unresolved values and call targets produce `name.unresolved` diagnostics. A
qualified call does not fall back to a bare function with the same final
segment when no matching import alias exists.

#### Removed network call spellings

The following call targets are the closed set of spellings removed from the
compiler-known network surface:

- `net::receive_chunk`
- `net::send_chunk`
- `net::listen`
- `net::connect`
- `net::accept`
- `net::accept_or_end`
- `net::accept_until`
- `net::accept_until_cancellable`
- `net::listener_local_addr`
- `net::read_chunk`
- `net::stream_local_addr`
- `net::stream_peer_addr`
- `net::stream_can_read`
- `net::stream_can_write`
- `net::stream_is_closed`
- `net::read_chunk_until`
- `net::read_chunk_until_cancellable`
- `net::read_chunk_or_end`
- `net::write_chunk`
- `net::write_chunk_until`
- `net::write_chunk_until_cancellable`
- `net::write_chunks`
- `net::write_chunks_until`
- `net::write_chunks_until_cancellable`
- `net::shutdown_write`
- `net::shutdown_read`
- `net::close_stream`
- `net::close_listener`

An unimported call target in this set produces one `name.unresolved`
diagnostic. Its primary message names only the unresolved call target. Three
related notes identify the required `use net from "std"` import, the nominal
`net::IO` effect, and the `net::system()` application handler. An unknown
`net::...` call target outside this set retains the ordinary unresolved-name
shape without those migration notes. The notes also remain absent when a
removed spelling appears in a value position or the source module declares an
explicit import whose alias is `net`.

In a match, an unresolved constructor path also produces `name.unresolved`
when the scrutinee has a singleton or finite variant-refinement type and is an
immutable binding written as a bare name or with redundant parentheses. The
primary span selects the final path segment, the namespace is `constructor`,
and the diagnostic symbol retains the complete written path. The unresolved
path does not select a constructor identity, and the arm covers no variant.
Other match scrutinee forms retain their existing constructor-path and
exhaustiveness behavior. The
[type inference rules](types.md#inference-rules) define the resulting match coverage, and the
[diagnostic contract](diagnostics-json.md#diagnostic-families) defines the
machine-readable projection.
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

In type annotations, bare `WallTime` selects the implicit public
`std::prelude` type when no source type or type alias with that spelling wins.
`prelude::WallTime` selects the same standard type through the explicit prelude
qualifier when `prelude` does not resolve as a written import. A same-module
source type or type alias named `WallTime` shadows only the bare standard name,
so aliases targeting that local declaration retain its nominal identity
instead of the standard structural record shape.

A wildcard let target, `_`, evaluates its expression without declaring a local
name. It can be annotated for type checking, but it is never a resolvable
binding.

Each `begin` body introduces a nested lexical scope. Bindings declared in that
body stop at its closing `end`. Each `defer` block has its own nested lexical
scope, and bindings declared inside it stop at the block's closing `end`. The
block sees only bindings visible at the `defer` statement, so a later binding
remains unresolved inside the block. When the block references a visible
binding, it captures that binding identity. A later `let` may shadow the
captured binding. References in the deferred block keep the captured identity,
while references after the later `let` resolve to the new binding.
A `let` binding becomes visible only after its complete initializer. When that
initializer is a multiline `begin`, references inside the initializer continue
to resolve to the preceding local, parameter, function, or imported identity;
the new binding starts after the `begin` closes.

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
records. This includes local annotations in
nested `begin` and `defer` bodies and in a `begin` used as a handler operation
clause. Qualified nominal effect
paths inside function type
`effects [...]` annotations are effect paths, not qualified type paths, and do
not produce source identifier casing diagnostics. An unresolved or
ambiguous intermediate segment is not assigned a role from spelling alone.
An ordinary qualified generic type such as `Alias::Container<Int>` keeps
`Container` as its final type segment. Structural variant-refinement metadata
that resolution does not select leaves those ordinary qualified-type roles
unchanged. A selected variant-refinement candidate uses its refinement roles
instead of also assigning the same written segments ordinary qualified-type
roles. When its base resolves, casing analysis classifies the qualifiers as
modules, the base leaf as a type, and the selected final segment as a
constructor. Each classification retains the written segment, span, and
segment index. If changing only an invalid-cased base leaf selects exactly one
type, casing analysis retains that type classification. It reports
`name.invalid_case` and still checks the recovered base's independent
variant-refinement eligibility. For a refinement anywhere in a public function
parameter or result, or in a public source-ADT payload, that unique recovered
terminal ADT identity also participates in the visibility check when the
written base names it directly or reaches it through a finite transparent
type-alias chain. An uppercase final must resolve exactly. An ASCII-lowercase
final remains a parse failure, but an initial-only repair that selects exactly
one constructor reaches casing and visibility analysis without making the
annotation valid. The privacy diagnostic
retains the written base casing. Ambiguous or unresolved base recovery, or an
ambiguous or missing exact final segment, does not manufacture a privacy
diagnostic. This includes a source-less base
published by the type-syntax provider: an invalid-cased generic spelling
reports casing and keeps an independently provable generic-arity failure.
An unresolved refinement base, including a base without type arguments,
reports `name.unresolved` at the written base path. The final segment receives
no constructor role, and the unresolved alternative supplies no identity for
visibility, union comparison, or assignability checks. A declaration type
parameter remains a known zero-arity type identity in its declaration context;
applying arguments to it reports the ordinary arity failure without assigning
a constructor role to the final segment.
For a non-generic path whose prospective base and final segments both start
with ASCII lowercase letters, unique initial-only base recovery first
selects the refinement interpretation. A unique initial-only final recovery
then selects the constructor. Both segments report `name.invalid_case`, the
parse diagnostic remains first, and the selected constructor participates in
the same independent checks. Without a selected base, the path remains an
ordinary named type and its final segment is not assigned a constructor role.
Qualified type arguments nested in the base retain their own type-path
records. A uniquely recovered invalid-cased type in such an argument reports
`name.invalid_case` without
suppressing an independently provable eligibility failure for the enclosing
refinement base. An ambiguous or unresolved recovery does not assign a type
role or manufacture an eligibility failure. The
[diagnostic specification](diagnostics-json.md#diagnostic-families) owns the
overlap order and downstream suppression rules. An explicit expression type
argument is a navigation-only carrier for variant refinements: casing analysis
does not classify its qualifier, base, final variant, or nested generic
arguments and does not emit `name.invalid_case` for those segments. The shared
language service still uses their resolved type and constructor identities for
variant-refinement definition, references, and rename. The observable LSP and
MCP selection rules are specified by
[editor support](editor-support.md#lsp-navigation-formatting-and-rename) and
[MCP](mcp.md#saved-workspace-navigation).
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
  duplicate parameters, unless an earlier `defer` block captured the binding
  that the later `let` shadows
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
sources that also have parse diagnostics. A source whose only parse diagnostics
are recoverable ASCII-lowercase variant-refinement final segments is lowered
and registered under its valid derived module identity for semantic diagnostic
recovery. The invalid annotation remains an error. Any unrelated parse
diagnostic prevents lowering and normal module registration. A rejected
derived-module identity is recorded only when visible module derivation itself
fails solely with source-path `name.invalid_case` diagnostics. A source
excluded by an unrelated parse diagnostic whose source path casing is accepted
does not record a rejected derived-module identity and does not suppress a
single-segment unresolved local import. A source path segment that starts with
an ASCII lowercase letter but contains another invalid module-identifier
character reports `module.invalid_source_path` instead of
`name.invalid_case`.
Before executable doctest extraction, the documented source path must also be
a canonical package-relative virtual-source origin. An invalid origin reports
`module.invalid_source_path` once against the documented source and creates no
source-visible doctest module identity. Other selected sources continue
through module derivation and semantic analysis.
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
- Unqualified ADT import lookup:
  `crates/veln-sema/src/adt/tests/variant_lookup.rs`.
- Explicit-import-alias acceptance evidence:

  | Cases | Evidence |
  | --- | --- |
  | A1 | `docs/specification/source-surface-executable.pl`, `examples/specification/check/explicit-import-alias-canonical-standard/` |
  | A2-A3 | `crates/veln-sema/src/tests/calls_pipeline_and_float_types/explicit_import_aliases.rs`, `examples/specification/run/explicit-import-alias-runtime/` |
  | A4 | `crates/veln-sema/src/adt/tests/variant_lookup.rs`, `examples/specification/check/explicit-import-alias-distinct-origins/`, `examples/specification/check/explicit-import-alias-duplicate-human/`, and `examples/specification/check/explicit-import-alias-duplicate-json/` |
  | A5-A7 | `crates/veln-sema/src/tests/declarations_and_names/import_aliases.rs`, `crates/veln-sema/src/tests/identifier_casing/explicit_import_aliases.rs`, `crates/veln-sema/src/name_recovery.rs`, `crates/veln-analysis/src/surface/tests/contracts_and_imports.rs`, `examples/specification/check/explicit-import-alias-casing-human/`, and `examples/specification/check/explicit-import-alias-casing-json/` |
  | A8 | `crates/veln-syntax/src/tests/modules_and_contracts.rs` and the accepted and rejected fixtures under `docs/specification/source-surface-fixtures/` |
  | A9 | `examples/specification/check/explicit-import-alias-carriers/`, `examples/specification/run/explicit-import-alias-runtime/`, `examples/specification/run/explicit-import-alias-handler/`, and `examples/specification/run/explicit-import-alias-handler-identity/` |
  | A10 | `examples/specification/check/explicit-import-alias-package/`, `examples/specification/check/explicit-import-alias-package-boundaries/`, and publication and visibility tests in `crates/veln-sema/src/tests/declarations_and_names/import_aliases.rs` and `crates/veln-sema/src/tests/calls_pipeline_and_float_types/explicit_import_aliases.rs` |
  | A12 | `combined_module_import_diagnostics_use_stable_source_order` in `crates/veln-sema/src/tests/declarations_and_names/import_aliases.rs` |
- Deferred binding visibility:
  `examples/specification/check/defer-binding-visibility/`.
- Module-value function-alias diagnostics:
  `examples/specification/check/module-value-function-alias-kind-human/` and
  `examples/specification/check/module-value-function-alias-kind-json/`.
- Casing and source-path diagnostics: `crates/veln-sema/src/pipeline/identifier_casing/`.
- Source-less descriptors and lookup routes: [source-less-lookup.md](source-less-lookup.md).
