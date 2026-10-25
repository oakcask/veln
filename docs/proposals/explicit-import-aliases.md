---
role: proposal
update-when: Explicit import-alias syntax, qualified-only import visibility, alias lookup and collision rules, alias casing and recovery, or the planned compiler and tooling acceptance cases change.
---

# Explicit Import Aliases

Explicit aliases give imported modules a chosen local qualifier and let callers
avoid collisions between bare exports. This proposal owns their complete
syntax, lookup, and casing contract. The
[current name-resolution specification](../specification/name-resolution.md)
and [executable grammar](../specification/source-surface-executable.pl) describe
imports without explicit aliases; the extension below is planned behavior.

## Source Surface

The planned grammar is:

```text
use_declaration = "use" module_path ["as" alias] ["from" string] newline
alias = identifier
```

`as` is recognized contextually in this import position. It does not reserve a
new keyword in unrelated declarations or expressions. The alias is one
identifier, not a module path, wildcard, string, or parenthesized expression.
The `as` clause precedes `from`. Existing import placement and newline rules
remain unchanged. The source string continues to select package origin;
without `from`, the existing local import rules apply.

```veln
use transport::net as wire from "std"
use application::model as domain
```

Each explicit alias must begin with an ASCII lowercase letter. The remaining
characters follow the existing identifier rules. Uppercase and underscore-led
identifier tokens, including `_`, are retained for casing diagnostics; `_`
does not mean an unused or side-effect-only import. Import-path segments keep
their own existing casing checks. An alias neither repairs nor replaces an
invalid written target path.

## Visibility and Identity

An explicit alias binds one name in the importing module's existing import
namespace. Its scope is the existing module-local import scope, including
sources combined under that module identity where the current consumer does
so. A local value or type with the same spelling does not hide or duplicate
that import binding: syntactic use roles select the relevant namespace.
The alias is not a value and cannot be passed as an argument.

`use a::b as local` contributes these forms:

| Use form | Resolution |
| --- | --- |
| `local::member` | Select the imported module, then resolve `member` in the namespace required by the use. |
| `a::b::member` | Retain full-written-path access where the existing import consumer supports it. |
| `b::member` | No implicit leaf alias is contributed by this declaration. |
| Bare `member` | No bare imported-export candidate is contributed by this declaration. |

The last two rules do not block independently visible declarations or imports.
For a one-segment target such as `use net as wire`, `net::member` can still
resolve as the full written path; it is not an additional implicit alias.
Explicit aliases do not create new submodules: `local::nested::member` works
only if the ordinary target supports that remaining path, such as a
constructor qualified through its source type.

Imports without `as` retain their current implicit-leaf and bare-export rules.
An explicit alias equal to the target leaf is still qualified-only. This gives
callers a deliberate way to suppress bare candidates without renaming the
qualifier. To repair a bare ambiguity, callers can alias one of the imports
and qualify uses of that module.

An alias selects the same package and module identity as its target import.
Public visibility, exact-companion private access, nominal type and effect
identity, schema identity, signatures, and runtime dispatch are unchanged.
It does not grant access through a private or non-exported package source.
Imports in other modules do not become visible transitively. Existing public
function, type, and schema aliases may target an explicitly aliased import;
their targets resolve in the declaring module, and their existing publication
rules remain in force. This proposal adds no `pub use` form.

The qualifier applies consistently to values and calls, types and variant
refinements, constructors and patterns, schemas and schema operations, effects
and operations, and handlers. Consumers must not reconstruct the qualifier
from the target path's final segment. Inference, lowering, and reachability
use the selected target identity, not a second alias-derived module identity.

## Lookup and Collision Rules

Lookup first selects an imported module and then selects its member. It never
chooses between colliding module bindings by inspecting which module happens
to export the requested member. Exact full-written import paths retain their
existing precedence over local qualifiers. If no exact path wins, a qualifier
must select exactly one eligible import binding. A failed or ambiguous exact
match does not fall through to an alias, bare member, or implicit prelude
candidate.

Collision checks involving an explicit alias use source-written imports,
including invalid-cased imports, so quarantine does not hide independently
provable duplicate facts. Two unaliased imports that have different full
written paths keep their existing declaration behavior when their implicit
leaf qualifiers are equal. The declarations are accepted without a new
duplicate diagnostic. Lookup through that implicit qualifier remains
unresolved, while each unambiguous full written path remains eligible.

| Input | Required result |
| --- | --- |
| Two imports bind the same qualifier and at least one binding is an explicit alias | `name.duplicate` at the later qualifier, with the first binding in related context. Lookup through the colliding qualifier is unresolved in either order. |
| Two unaliased imports have different full written paths and the same implicit leaf qualifier | Preserve the existing declaration behavior: report no qualifier duplicate. Lookup through the implicit qualifier is unresolved in either order. Each unambiguous full written path remains eligible. |
| The same origin and full target path are imported twice, even with different aliases | Preserve the duplicate-import-path error; adding an alias is not a way to import that module twice. Neither duplicate import is eligible for normal lookup. |
| The same written path comes from distinct package origins under distinct aliases | Distinct qualified aliases are valid; the full-written-path form is ambiguous and cannot fall through. |
| An explicit alias is `prelude` | `name.reserved` at that alias. No user binding replaces the implicit standard prelude. |
| A target path would have an implicit leaf `prelude`, but a different explicit alias is supplied | The reserved-alias rule applies to the explicit alias; target module availability and source-identity restrictions still apply independently. |
| A type, value, or other non-import declaration has the alias spelling | No import-namespace duplicate; each use selects its normal namespace. |
| One import has both a duplicate target path and duplicate qualifier with the same earlier import | One `name.duplicate` for that repeated import, not two descriptions of the same conflict. |
| The target-path and qualifier collisions involve different earlier imports | Preserve both independently provable conflicts, with each earlier import in its related context. |

For multiple source files with the same module identity, diagnostic order uses
the repository's stable source order and token order, not discovery order.
Within one invalid import, parser diagnostics precede casing diagnostics;
reserved-name and duplicate facts follow casing, and target-resolution facts
remain independent. A duplicate target-path diagnostic uses the written path.
A qualifier diagnostic uses the explicit alias token when the later binding is
explicit, or the implicit leaf token when a later unaliased binding collides
with an earlier explicit alias. Two implicit bindings do not produce that
diagnostic.
The ordinary unresolved import diagnostic remains at the target path, even
when the explicit alias is itself invalid.

## Casing and Recovery

An uppercase or underscore-led alias reports `name.invalid_case` at its exact
token, with occurrence `declaration`, name class `module`, required initial
`ascii_lowercase`, and the corresponding observed initial. The import is
quarantined atomically: it supplies no explicit qualifier, implicit leaf,
full-written-path candidate, or bare export candidate to normal lookup.
Casing errors in the path quarantine the entire import in the same way.

The invalid spelling still participates in independent collision and target
checks. If a valid declaration and an invalid-cased declaration import the same
target from the same origin, neither declaration supplies a normal exact-path,
qualifier, or bare route. Semantic lookup and reachability apply that target
collision in either declaration order. An independent non-colliding exact path
remains eligible. Recovery may suppress a derivative unresolved-use,
type-origin, constructor-arity, exhaustiveness, unknown-effect, or
unknown-handler error only when the existing quarantine proof establishes one
matching target export and proves that invalid casing is the sole failure. It
does not restore normal lookup or executable readiness. A valid competing
candidate wins over recovery. Missing modules, missing or private exports,
wrong kinds, and ambiguous recovery retain their own diagnostics.

For parser recovery, retain the target path, package source when parseable,
and any identifier-shaped alias token with its span. Missing aliases report a
parse diagnostic at the point where an identifier is required. A malformed
non-identifier alias reports it at the offending token. Do not synthesize an
alias from the target leaf. A retained uppercase or underscore-led identifier
is a semantic casing error rather than a missing-alias parse error.

| Malformed form | Recovery boundary and result |
| --- | --- |
| `use a as` followed by newline | Diagnose missing alias; leave the next declaration available for parsing; bind nothing. |
| `use a as from "pkg"` | Diagnose missing alias before `from`; retain the package source for independent target checking; bind nothing. |
| `use a as "local"` or `use a as b::c` | Diagnose malformed alias/trailing path; recover to the declaration newline; bind nothing. |
| `use a from "pkg" as local` | Diagnose misplaced trailing alias; do not reinterpret it as an accepted order; bind nothing. |
| A repeated `as` clause or missing package string | Diagnose the malformed import; recover to newline; bind nothing. |

A syntax-recovered import contributes no normal lookup candidates, even if a
valid-looking alias or target was retained. Recovery must not consume the
following `use`, function, or type declaration after the newline. The retained
spelling supports diagnostics only; it must not become a partial valid import
in another compiler or editor consumer.

## Source Tools

Formatting preserves `as alias` before `from` and retains the written target
path; repeated formatting is idempotent. Definition on an alias qualifier
selects its import declaration; member definition selects the target member.
References and rename for an explicit alias include its declaration and
qualifier uses in the owning module, without renaming the target module,
written import path, member, or same-spelled alias in another module. An alias
rename to invalid casing, `prelude`, or a conflicting import binding is rejected
without partial edits. A target-member rename continues to include member
uses through the alias and leaves the alias unchanged.

Source hover and qualified completion present the local spelling while
retaining the selected package and module provenance. Saved LSP and MCP
navigation must agree with compiler selection. Invalid or syntax-recovered
aliases do not manufacture normal target navigation. This proposal does not
expand which target declaration kinds each existing navigation feature
supports; it requires existing supported kinds to recognize explicit aliases.

## Related Work and Decisions

The inspected Rust Reference
[use-declaration grammar and `as` renames](https://doc.rust-lang.org/reference/items/use-declarations.html#as-renames)
provide an explicit local rename after a path. Veln adopts `as` in that position
because the target remains readable before the chosen local name. Rust also
supports item imports, globs, nested trees, and public re-exports. Those forms
are not adopted: this proposal binds only module qualifiers and preserves
Veln's existing public-alias declarations.

The inspected Go specification
[import declarations](https://go.dev/ref/spec#Import_declarations)
separates a package's local qualifier from its import path and distinguishes
qualified access from dot-imported bare names. Veln adopts the separation of
local qualifier and target identity, and uses explicit aliases for
qualified-only access. Veln's unaliased imports already supply bare candidates,
so that existing behavior remains; Go's file scope, dot syntax, and blank
side-effect imports do not transfer. Keeping bare candidates for aliased
imports was rejected because it would rename qualification without solving
bare-export collisions.

Neither external language establishes Veln's casing, exact-path precedence,
companion access, or quarantine rules. Those choices extend the inspected
[current name-resolution contract](../specification/name-resolution.md).
The current import parser in
`crates/veln-syntax/src/parser/functions_and_imports.rs` retains a path and optional package source but has no explicit alias. Therefore
this proposal is an implementation target, not a report of supported syntax.

## Acceptance Model

All evidence below is planned. The grammar and decision tables above define
the independent expected results.

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| A1 | Local and package import with `as`; import without `as` | Accept the selected grammar; unaliased behavior remains current. | Accepted executable-grammar fixtures and compiler `check` cases. |
| A2 | Aliased module with a bare export colliding with another import; alias equal to leaf | Only qualified uses gain candidates from the aliased import; bare use selects the remaining import. | Name-resolution and executable call cases. |
| A3 | Alias qualifier, full written path, old leaf, and single-segment target | Alias and existing full-path rules work; no old implicit leaf is added; single-segment exact path remains distinguishable. | Check cases with exact selected identities and negative lookup assertions. |
| A4 | Same explicit qualifier on different imports; explicit-to-implicit collision; two unaliased paths with the same implicit leaf; repeated target with different aliases; same path from distinct origins | Explicit-involved duplicate and ambiguity rules match the table in either import order; implicit-only collisions retain accepted declarations, unresolved implicit lookup, and eligible exact paths; no member-based fallback. | Ordered JSON and human diagnostics, accepted unaliased check and exact-path cases, plus origin-aware resolution cases. |
| A5 | Explicit `prelude`, aliased target leaf `prelude`, and same-spelled non-import declaration | Reserved qualifier rejected; independent target restrictions retained; separate namespaces accepted. | Compiler diagnostic and namespace cases. |
| A6 | Uppercase and underscore-led alias; invalid path plus valid alias; mixed invalid/valid repeated target in both declaration orders | Exact casing fields and token spans; atomic quarantine; independent errors retained in deterministic order; neither repeated target enters semantic lookup or reachability; an independent exact route remains eligible. | Human/JSON casing and overlap cases, direct semantic selection, reachable-set cases, and a non-colliding exact-path control. |
| A7 | Invalid alias used for valid, missing, private, or wrong-kind members | Only proven derivative cascades suppressed; independent failures remain; no executable readiness through recovery. | Recovery and checked-artifact cases. |
| A8 | Each malformed-import row followed by a valid declaration | Parse error at the offending token/boundary; following declaration retained; no implicit or partial import installed. | Parser recovery and rejected executable-grammar fixtures. |
| A9 | Alias used across value, type, constructor, schema, effect, and handler carriers | Same target identities, visibility, effects, and reachable code as valid original qualification. | Compiler carrier matrix and representative runtime call/handler cases. |
| A10 | Public aliases target the import; another module uses or tries to bypass it | Existing publication and privacy rules hold; local import aliases do not leak transitively. | Package and companion visibility fixtures. |
| A11 | Formatting, definition, references, completion, and rename on alias and member | Alias/source identity stays distinct; valid edits affect only intended occurrences; rejected rename has no partial edits. | Existing formatter, shared-language-service, LSP, and MCP tests. |
| A12 | Reorder selected sources with combined module identity | Lookup and collision outcomes stable; diagnostics use stable source/token ordering. | Analysis and navigation permutation cases. |

Implementation must update the executable grammar, parser and checker cases,
and the existing command/tooling harnesses for the contracts identified above.
These are compiler and source-tool contracts, not standard-library API tests.
Run the focused suites through the guarded test runner. Update the current
name-resolution, source grammar, diagnostics, and relevant tooling
specifications only when their executable evidence passes. Then remove this
proposal and its catalog entry. Also update any active proposal dependency that
still describes the implemented compiler alias syntax as future work. No
repository-wide source migration is needed before the new syntax exists.
