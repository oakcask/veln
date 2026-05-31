# Implicit Prelude And Unqualified Imports

Status: implemented

This record preserves the completed proposal that finished the ordinary import
model for the standard prelude. Use the specification and checked examples for
current behavior; use this page only for implementation history and cleanup
evidence.

## Read First

- Current module headers, `use` declarations, and public API boundary:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current name resolution, shadowing, compiler-known calls, and prelude helper
  behavior:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Executable import and runtime evidence:
  `examples/specification/check/implicit-prelude-imports/` and
  `examples/specification/run/implicit-prelude-imports/`.
- Completed source-backed helper migration history:
  [self-hosting-standard-library.md](self-hosting-standard-library.md).

## Implemented Outcome

- User modules have an implicit standard `prelude` import for public prelude
  helpers.
- Bare prelude helper calls keep working when unambiguous, while
  `prelude::helper` explicitly selects the standard helper.
- Written imports that export the same bare helper name create the same
  ambiguous-import diagnostic shape as ordinary public function imports, with
  related notes that include qualified repair spellings.
- Local bindings and declarations in the current module shadow imported
  prelude helpers; the standard helper remains reachable through `prelude::`.
- User source cannot declare `mod prelude` or a written import alias
  `prelude`.
- The embedded standard-library source module and source metadata use
  `prelude` and `crates/veln-stdlib/veln/prelude.veln`; public helper entry
  points are `pub fn` declarations and private support functions remain
  non-public.

## Problem

Source modules already exported public functions and public source ADT
constructors. A written `use` declaration created a qualified module alias and
also exposed unambiguous public function exports in bare scope. Public source
ADT constructors had the same unambiguous bare-import model.

Prelude helpers were still available as bare calls through compiler-known
prelude resolution rather than through the ordinary module import model. That
left the word `prelude` carrying several meanings: the user-facing helper
vocabulary, the embedded source module, and the reserved `prelude_builtin`
runtime-operation module.

The implementation aligns these concepts by making the prelude an ordinary
standard module import with the same conflict rules that written imports
already use.

## Name Resolution

With the implicit prelude included, name lookup for value expressions uses this
order:

- lexical bindings, parameters, and pattern bindings visible at the use site;
- declarations in the current source module;
- unqualified imports, including the implicit standard prelude import, when
  exactly one imported public export provides the requested name.

If no candidate exists, checking reports the current unresolved-name
diagnostic. If more than one imported public export provides the same
unqualified name, checking reports an ambiguous-import diagnostic and points to
qualified spellings such as `module::name` or `prelude::name`.

Local declarations and lexical bindings shadow imported names. For example, a
module-local `fn vec_len(...)` makes bare `vec_len(...)` refer to the local
declaration; the standard helper remains available as `prelude::vec_len(...)`.

Qualified lookup is not ambiguous. `alias::name` resolves through the explicit
alias created by a written `use` declaration or through the implicit `prelude`
alias.

## Prelude Source

The embedded standard-library source module is `prelude`, and the source file
is `prelude.veln` with this module header:

```veln
mod prelude
```

Public prelude entry points that users may call by bare name or
`prelude::<name>` are declared with `pub fn` in the embedded source. Private
support functions used only to implement those entry points remain ordinary
non-public functions and are not imported into user modules.

The reserved `prelude_builtin` module keeps its implementation-only purpose.
It is not a public module import.

## Completion Evidence

- Semantic tests cover qualified prelude calls, prelude/import ambiguity,
  local declaration shadowing, non-callable local shadowing, and reserved
  `prelude` module claims.
- Checked examples cover ambiguous bare helper imports and reserved aliases.
- Runtime examples cover `prelude::` fallback and local declaration shadowing.
- Standard symbol descriptor tests verify that source metadata points at
  `crates/veln-stdlib/veln/prelude.veln` and the embedded module header is
  `mod prelude`.

## Out Of Scope

- Export lists beyond the current public declaration boundary.
- Glob imports, import renaming, hiding imports, or selective import lists.
- Method syntax, traits, type classes, or overload resolution by type.
- Renaming `prelude_builtin` or exposing it to user modules.
- Changing public helper signatures or container value semantics.
