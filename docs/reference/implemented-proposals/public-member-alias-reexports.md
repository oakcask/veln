# Public Member Alias Re-Exports

Status: implemented

This page records the completed public member alias target. Current source
syntax, name resolution, and executable examples live under
`../../specification/` and `../../../examples/specification/`; this record is
history and completion evidence.

## Read First

- Current source declaration syntax:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current function name resolution, effect propagation, and reachability:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Checked example coverage:
  `../../../examples/specification/check/public-member-alias-reexports/`.

## Implemented Outcome

Modules can publish explicit public aliases for existing function and source
ADT members:

```veln
pub fn parse = impl::parse
pub type Document = impl::Document
```

The alias declaration owns the exported member name in the declaring module.
The target remains the implementation member: function aliases use the
referenced function signature and effects, and type aliases expose the
referenced source ADT constructors through the aliasing module path.

Alias declarations do not accept signatures, type parameters, effect lists,
contract clauses, constructor lists, or bodies. `use` declarations remain
import-only and do not re-export another module's public API.

## Completion Evidence

- Parser and source-surface fixtures accept `pub fn` and `pub type` member
  alias declarations, and reject alias forms that add call syntax or a
  function signature.
- Semantic tests cover function aliases resolving through imported module
  paths and type aliases exposing imported constructors.
- Executable specification examples cover successful explicit alias
  re-exports, public alias diagnostics, and the unchanged rule that `use`
  alone is not a re-export.

## Boundary

- Dedicated export lists, glob exports, import renaming, hiding imports, and
  selective import lists remain unimplemented.
- Aliases are limited to `fn` and source ADT `type` members.
- Aliases are not wrappers and cannot change signatures, effects, contracts,
  constructors, or type identity.

## Skip Unless Needed

- Do not read this page for current syntax or lookup rules.
- Use this record only when auditing why the completed alias target is no
  longer listed under planned proposal work.
