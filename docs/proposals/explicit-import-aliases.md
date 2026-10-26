---
role: proposal
update-when: Runtime execution or source-tool navigation for implemented explicit import aliases changes.
---

# Explicit Import Alias Completion

The compiler accepts, formats, and semantically resolves explicit import
aliases. Current syntax, diagnostics, lookup, visibility, lowering, and
reachability are specified by
[Source Surface](../specification/source-surface.md#usage-and-declaration-forms),
[Name Resolution](../specification/name-resolution.md#explicit-import-aliases),
and [Execution Boundary](../specification/execution.md#runtime-readiness-and-host-boundaries).
This proposal contains only the remaining runtime validation and source-tool
integration.

## Remaining runtime scope

Alias-selected calls must execute with the same target identity as the full
written import path. Distinct modules may export the same member name without
collapsing their runtime identities. A bare call receives no candidate from an
explicitly aliased import.

The checked runtime case currently reaches JVM cache preparation but does not
reach Java launch because its generated class set fails cache validation. The
runtime contract remains planned until that case passes without weakening the
cache-integrity checks.

## Remaining source-tool scope

- Definition on an alias qualifier selects its import declaration. Definition
  on a qualified member selects the target declaration.
- References and rename include the alias declaration and qualifier uses in
  the owning module. They do not rename the written target path, member, or a
  same-spelled alias in another module.
- Rename rejects invalid casing, `prelude`, and import-namespace collisions
  without partial edits. Target-member rename retains the alias.
- Hover and qualified completion display the local qualifier while retaining
  package and target-module provenance.
- Saved LSP and MCP navigation agrees with compiler selection. Invalid and
  syntax-recovered aliases create no normal navigation target.

This work does not expand the declaration kinds supported by existing
navigation features.

## Acceptance model

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| A9 | Alias and full written path call same-named exports from distinct modules; a bare call selects only an unaliased import | Each call executes the selected declaration, preserves module identity, and produces the expected output. | Passing checked JVM runtime case with cache validation enabled. |
| A11 | Definition, references, rename, hover, and completion on an alias and member | Alias and target identity remain distinct; valid edits affect only intended occurrences; rejected rename has no partial edits; saved LSP and MCP results agree. | Shared-language-service, LSP, MCP, and saved-workspace tests. |

## Related work

Rust's [use declarations](https://doc.rust-lang.org/reference/items/use-declarations.html#as-renames)
and Go's [import declarations](https://go.dev/ref/spec#Import_declarations)
separate a local qualifier from imported identity. They support broader import
forms and do not define Veln runtime or navigation behavior, so they inform
terminology but do not determine these remaining contracts.
