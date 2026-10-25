---
role: reference
authority: supporting
update-when: Explicit import alias clause order or qualified-only visibility changes.
---

# Explicit Import Alias Source Shape

The current [source-surface](../../specification/source-surface.md) and
[name-resolution](../../specification/name-resolution.md) specifications own
the implemented behavior. This record explains why an explicit alias follows
its target path and why the import is qualified-only.

## Decision

Veln writes the target before its local qualifier: `use target as local`. The
target remains readable before the source introduces the chosen local name.
The form also keeps the optional package source in its existing final position:
`use target as local from "package"`.

An explicitly aliased import contributes qualified routes through the local
alias and the supported written target path. It contributes no bare imported
member candidates. Retaining bare candidates would rename qualification
without solving collisions between bare exports, which is the reason to choose
an explicit local qualifier.

## Source Support And Limits

The inspected Rust Reference
[use-declaration grammar and `as` renames](https://doc.rust-lang.org/reference/items/use-declarations.html#as-renames)
places an explicit local rename after its path. Veln adopts that readable
target-before-local-name order, but does not adopt Rust item imports, globs,
nested trees, or public re-exports.

The inspected Go specification
[import declarations](https://go.dev/ref/spec#Import_declarations) separates a
package's local qualifier from its import path and distinguishes qualified
access from dot-imported bare names. Veln adopts the separation between local
qualifier and target identity. Veln keeps its existing unaliased-import
behavior and does not adopt Go file scope, dot imports, or blank imports.

Neither source determines Veln's exact-path precedence, casing, privacy, or
recovery behavior. Those are current Veln contracts in the linked
specifications rather than consequences of the external syntax comparisons.
