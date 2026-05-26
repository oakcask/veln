# Discussion Result: Scoping and Name Resolution

Status: implemented

## Picked Question

- What first-slice scoping and name-resolution policy should apply to duplicate
  `let` names, imports, public API membership, result bindings, and named holes?

## Decision

Use lexical scope with explicit namespaces, reject duplicate declarations in
the same scope and namespace, and keep named holes outside name resolution.

The first slice should have at least these checker-facing namespaces:

- module names and import aliases
- type names, including built-in and opaque named types
- value names, including functions, parameters, `let` bindings, and
  contract-facing result bindings
- diagnostic labels, including named-hole labels

A source reference resolves by namespace and nearest lexical scope. If two
candidate declarations are equally near in the same namespace, resolution fails
with an ambiguity or duplicate-definition diagnostic instead of choosing one by
source order.

## Rationale

Name resolution should be a deliberately modeled relation from a reference to a
declaration, not a side effect of whichever checker phase reaches the name
first. Neron, Tolmach, Visser, and Wachsmuth's scope-graph work frames name
resolution as a language-independent path search from references to
declarations, with language-specific rules constructing the graph. That is a
good fit for Veln because imports, local bindings, contract names, and later
module boundaries all need stable provenance for diagnostics and rename-like
repairs.

The follow-on scope-graph work in *Scopes as Types* is also relevant even
though Veln's first type system is intentionally small. It argues for using
scope structure as part of static semantics, not merely as a parser artifact.
For Veln, that supports storing binding facts in the same analysis-table style
chosen by the AST phase-boundary decision, so type, contract, effect, and hole
diagnostics can all point to the same declaration origins.

Rust's reference provides a useful mature-language warning: name resolution,
namespaces, visibility, imports, and staged resolution become complicated even
when the user-facing syntax is compact. Veln should not copy Rust's exact
rules, but it should adopt the explicit split between namespaces and scopes.
The first slice has no macros and no type-relative method lookup, so Veln can
choose the simpler invariant that equally ranked candidates are errors.

For compiler diagnostics, Barik, Ford, Murphy-Hill, and Parnin's work supports
messages that provide evidence and, when possible, a resolution. Duplicate and
ambiguous-name diagnostics should therefore show the current reference, the
conflicting declarations, the namespace, and the smallest repair options:
rename one declaration, qualify an import, remove an export, or choose a
different result binding.

Typed-hole practice points in a different direction for hole names. GHC typed
holes are designed to expose expected type, local bindings, constraints,
candidate fits, and provenance. Veln's named-hole decision already says a
named hole is a repair label rather than a binding. Preserving that separation
prevents duplicate hole labels from changing program meaning, while still
allowing the checker to include local scope information in hole diagnostics.

## First-Slice Rules

- A lexical scope has one declaration table per namespace.
- Declaring the same name twice in the same scope and namespace is an error.
  This includes duplicate parameters, duplicate `let` names in one block,
  duplicate import aliases, duplicate public value names, duplicate type names,
  duplicate record fields, and duplicate result bindings.
- Sequential same-block `let` rebinding is not shadowing in the first slice.
  A later `let name = ...` in the same block conflicts with the earlier one.
- Nested lexical scopes may shadow outer value names. The resolved declaration
  is the nearest one in lexical scope order, and diagnostics should include a
  `shadows` related span when shadowing is likely to explain a later error.
- Imports create declarations in the module namespace and, when an explicit
  unqualified-import form exists, in the imported member's namespace. Two
  unqualified imports that provide the same value or type name make the bare
  name ambiguous until the source qualifies it or changes an alias.
- The public API of a module is a single exported name table per namespace.
  Two public declarations cannot export the same name in the same namespace.
- A function's explicit result binding is a value name visible only to that
  function's `ensure` clauses and contract diagnostics. It is not visible to
  `require` clauses, the function body, callers, or other functions.
- A result binding must not duplicate a parameter name or another contract
  value name in the same function contract environment.
- Named holes do not introduce value names. Duplicate named-hole labels are
  allowed because each occurrence remains a separate missing expression, but
  the checker may emit a style hint when repeated labels in one function make
  repair targeting unclear.
- Pattern bindings declare value names for the match arm. Duplicate pattern
  binding names in one arm are errors, including names that duplicate values
  already visible at the arm. Record pattern field names are also unique within
  one record pattern.
- Unresolved-name, duplicate-definition, ambiguous-import, and shadowing
  diagnostics should report the namespace, candidate declaration spans, and a
  stable node ID for the reference when one exists.

## Open Details

The exact syntax for explicit unqualified imports and export lists remains
open. This decision only requires that whichever syntax is chosen feeds a
single module-level export table and reports ambiguous bare names rather than
using source-order precedence.

The first slice does not add explicit shadowing syntax such as `shadow let`.
If examples show that accidental local shadowing is common, the checker can
promote same-function shadowing from a hint to a warning without changing name
resolution semantics.

## Consequence

The checker gets one coherent binding model before parser, type, contract, and
hole diagnostics become fixtures. Agents can rely on deterministic name
resolution, stable declaration provenance, and repair-oriented duplicate-name
errors without treating hole labels as semantic variables.

## References

- Neron, P., Tolmach, A., Visser, E., & Wachsmuth, G. A Theory of Name
  Resolution. *Programming Languages and Systems*, 205-231.
  https://doi.org/10.1007/978-3-662-46669-8_9
- van Antwerpen, H., Bach Poulsen, C., Rouvoet, A., & Visser, E.
  Scopes as Types. *Proceedings of the ACM on Programming Languages*,
  2(OOPSLA), Article 114. https://doi.org/10.1145/3276484
- The Rust Reference contributors. *Name resolution*. The Rust
  Reference.
  https://doc.rust-lang.org/reference/names/name-resolution.html
- Barik, T., Ford, D., Murphy-Hill, E., & Parnin, C. How Should
  Compilers Explain Problems to Developers? *ESEC/FSE*.
  https://doi.org/10.1145/3236024.3236040
- GHC contributors. *Typed Holes*. Glasgow Haskell Compiler User's
  Guide. https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/typed_holes.html
