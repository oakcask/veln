# Public Member Alias Re-Exports

Status: proposed

This proposal adds explicit public aliases for function and type members so a
module can publish a small API surface backed by private implementation
modules.

## Read First

- Current module headers, `use` declarations, and public API boundary:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current name resolution and module ownership rules:
  [../specification/names-effects.md](../specification/names-effects.md).
- Related import-side proposal:
  [Implicit Prelude And Unqualified Imports](implicit-prelude-and-unqualified-imports.md).

## Problem

Source modules currently expose public functions and public source ADT types by
declaring them directly with `pub fn` or `pub type`. That works for small
modules, but it makes larger modules choose between two poor shapes:

- put public declarations and implementation details in the same source module;
- expose implementation module paths as part of the public API.

The language needs a way for a module to own a narrow public interface while
delegating implementation to private modules. Reusing `use` for that job would
blur two different actions: consuming another module's public exports in the
current scope, and defining this module's own public API.

## Proposal

A module may expose a public member by declaring an alias to an existing
function or type member:

```veln
pub fn parse = parser_impl::parse
pub type Document = ast_impl::Document
```

The left side declares the public member owned by the current module. The right
side is a name reference to an existing member and is not an expression, call,
lambda, type expression, or inline implementation.

Alias declarations are aliases, not wrappers or new type definitions. The
public alias has the same signature, effects, constructors, and type identity
as the referenced member. The alias may hide the implementation module path from
callers while preserving the referenced declaration's checked behavior.

Only current module member kinds may be aliased:

- `pub fn <PublicName> = <FunctionPath>` aliases a function member.
- `pub type <PublicName> = <TypePath>` aliases a type member.

The kind on the left side must match the referenced member. A `pub fn` alias to
a type, or a `pub type` alias to a function, is rejected at the alias
declaration.

Alias declarations cannot write a function signature, type parameter list,
effect list, contract clause, constructor list, or body. The referenced member
owns that surface. Code that needs a changed signature or a different type must
declare a real function or type instead of an alias.

For example, this is an alias:

```veln
pub fn parse = parser_impl::parse
```

This is a new wrapper function, not an alias:

```veln
pub fn parse(input: String) -> Result<Document, ParseError>
	parser_impl::parse(input)
end
```

The public alias participates in module exports exactly like a public
declaration written with that public name. It is visible through qualified
lookup and through any import rule that consumes public exports.

The current module must not contain two members with the same public name in
the same namespace. A public alias conflicts with a local declaration or another
alias of the same name just as two declarations would conflict.

## Compatibility

Existing public declarations continue to define the module's public API. A
public alias adds another explicit way to define that API without exposing the
implementation module path or writing a wrapper.

Existing `use` declarations keep their import-side meaning. They do not become
re-export declarations.

## Diagnostics

Invalid public aliases should report the failed fact at the alias declaration.
The primary message should distinguish kind mismatch, non-name right side,
duplicate public member, and forbidden signature or body syntax. Related notes
may point to the referenced declaration or earlier member that caused the
conflict.

## Implementation Plan

- Parse `pub fn <name> = <path>` and `pub type <name> = <path>` as public alias
  declarations, with no signature, type parameters, effects, contracts,
  constructors, or body.
- Resolve public aliases after ordinary module member collection.
- Check that the referenced member exists and has the matching kind.
- Add the alias to the current module's public export table.
- Add diagnostics and tests for successful function aliases, successful type
  aliases, kind mismatch, non-name right sides, duplicate public members, and
  forbidden signature or body syntax.

## Out Of Scope

- Export lists beyond explicit public declaration and alias boundaries.
- Glob exports, glob imports, import renaming, hiding imports, or selective
  import lists.
- Wrapper generation or signature-changing aliases.
- Alias support for member kinds beyond `fn` and `type`.
- Method syntax, traits, type classes, or overload resolution by type.
