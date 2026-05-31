# Implicit Prelude And Unqualified Imports

Status: proposed

This proposal makes `use` import public module exports into unqualified scope
when doing so is unambiguous, and then defines the standard prelude as an
implicit `use prelude` available to every user module.

## Read First

- Current module headers, `use` declarations, and public API boundary:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current name resolution, shadowing, compiler-known calls, and prelude helper
  behavior:
  [../specification/names-effects.md](../specification/names-effects.md).
- Completed source-backed helper migration history:
  [../reference/implemented-proposals/self-hosting-standard-library.md](../reference/implemented-proposals/self-hosting-standard-library.md).

## Problem

Current source modules already export public functions and public source ADT
constructors, and constructor use has moved toward a model where exported
default constructors are usable without spelling the module qualifier when the
import is unambiguous.

Function imports do not yet share that model. A `use` declaration creates a
qualified module alias, so callers must spell imported functions as
`alias::function(...)`. Meanwhile prelude helpers are available as bare calls
through compiler-known prelude resolution rather than through the ordinary
module import model. That leaves the word `prelude` carrying several meanings:
the user-facing helper vocabulary, the embedded `core_prelude` source module,
and the reserved `prelude_builtin` runtime-operation module.

This proposal aligns these concepts by making ordinary imports capable of
introducing unqualified names, then making the prelude an ordinary standard
module import with the same conflict rules.

## Proposal

`use <module>` continues to introduce the current qualified alias. It also
introduces the imported module's public exports into the importing module's
unqualified import scope when no other imported export or local declaration
claims the same value name.

For example, after `use math`, both calls are valid when `math` publicly
exports `add` and no local declaration or other import conflicts with it:

```veln
add(1, 2)
math::add(1, 2)
```

If two imported modules export the same value name, the bare name is ambiguous
and callers must use a qualified path:

```veln
use left
use right

fn call_left() -> Int
	left::size()
end

fn call_right() -> Int
	right::size()
end
```

The same unqualified-import rule applies to public functions and public source
ADT constructors in the value namespace. If a type export and constructor
export use separate namespaces, this proposal changes only the value lookup
used for calls, constructor patterns, and callable-value references.

Every user module is checked as though it had an implicit `use prelude`.
Therefore public exports from the standard `prelude` module are available by
bare name when unambiguous and by qualified name as `prelude::<name>` when
qualification is useful or required.

This proposal consumes whatever public exports a module declares. A separate
implemented record covers public member aliases as another way for a module to
declare those exports:
[Public Member Alias Re-Exports](../reference/implemented-proposals/public-member-alias-reexports.md).

## Prelude Module Rename

Rename the embedded standard-library source module from `core_prelude` to
`prelude`. The repository source file should move from the current
`core_prelude.veln` spelling to `prelude.veln`, and the module header should
become:

```veln
mod prelude
```

Public prelude entry points that users may call by bare name or
`prelude::<name>` must be declared with `pub fn` in the embedded source. Private
support functions used only to implement those entry points remain ordinary
non-public functions and are not imported into user modules.

The reserved `prelude_builtin` module keeps its current name and purpose. It is
not a public module import. It remains an implementation-only escape hatch for
standard-library source that needs to call runtime operations without spelling
them as recursive calls to public prelude helpers.

The `prelude` alias is reserved for the standard prelude import in user
modules. User source cannot declare `mod prelude`, and a written import whose
final alias segment is `prelude` conflicts with the implicit standard prelude
alias. The diagnostic should ask the user to choose a non-conflicting module
name or import path rather than silently replacing the standard prelude.

## Name Resolution Rules

Name lookup for value expressions should use this order:

- lexical bindings, parameters, and pattern bindings visible at the use site;
- declarations in the current source module;
- unqualified imports, including the implicit standard prelude import, when
  exactly one imported public export provides the requested name.

If no candidate exists, checking reports the current unresolved-name
diagnostic. If more than one imported public export provides the same
unqualified name, checking reports an ambiguous-import diagnostic and points to
qualified spellings such as `module::name`.

Local declarations and lexical bindings shadow imported names. For example, a
module-local `fn vec_len(...)` makes bare `vec_len(...)` refer to the local
declaration; the standard helper remains available as `prelude::vec_len(...)`.

Qualified lookup is not ambiguous. `alias::name` resolves through the explicit
alias created by a written `use` declaration or through the implicit `prelude`
alias.

## Compatibility

Existing prelude bare calls continue to work. Their explanation changes from
special prelude helper lookup to ordinary unqualified import lookup from the
implicit standard `prelude` module.

Existing qualified imports continue to work. Code that currently writes
`alias::function(...)` is still valid.

New bare imported calls may introduce ambiguity in source that imports two
modules exporting the same value name and also writes that name bare. The
repair is local and explicit: qualify the call or add a local declaration that
intentionally shadows the imports.

## Diagnostics

Ambiguous unqualified imports should report the failed fact at the bare name
span. Related notes should name the imported modules that contributed matching
public exports and should suggest qualified paths.

When a prelude export conflicts with an explicit import, diagnostics should
treat `prelude` like any other imported module. When a local declaration
shadows a prelude export, no ambiguity diagnostic is needed because ordinary
local shadowing decides the lookup.

## Implementation Plan

- Extend module export collection so public functions and public source ADT
  constructors are available for unqualified import lookup.
- Keep the existing qualified alias table for `alias::name` lookup.
- Add an implicit `prelude` import to every checked user module.
- Rename the embedded standard-library module and source metadata from
  `core_prelude` to `prelude`.
- Reserve the user-visible `prelude` module alias for the standard prelude and
  diagnose user modules or written imports that attempt to claim it.
- Mark public prelude entry-point functions in the embedded source with
  `pub fn`; keep implementation support functions non-public.
- Rephrase prelude helper checking so bare helper calls and
  `prelude::<name>` calls share the standard import model before lowering to
  the existing runtime targets.
- Add diagnostics and tests for unambiguous imports, imported-name ambiguity,
  local shadowing over imports, qualified fallback, and prelude conflicts.

## Out Of Scope

- Export lists beyond the current public declaration boundary.
- Glob imports, import renaming, hiding imports, or selective import lists.
- Method syntax, traits, type classes, or overload resolution by type.
- Renaming `prelude_builtin` or exposing it to user modules.
- Changing public helper signatures or container value semantics.
