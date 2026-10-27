---
role: proposal
update-when: Source-tool navigation or its planned verification for implemented explicit import aliases changes.
---

# Explicit Import Alias Source Tooling

The compiler accepts, formats, and semantically resolves explicit import
aliases. Current syntax, diagnostics, lookup, visibility, lowering, and
reachability are specified by
[Source Surface](../specification/source-surface.md#usage-and-declaration-forms),
[Name Resolution](../specification/name-resolution.md#explicit-import-aliases),
and [Execution Boundary](../specification/execution.md#runtime-readiness-and-host-boundaries).
This proposal contains only the remaining source-tool integration.

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
| A11 | Definition, references, rename, hover, and completion on an alias and member | Alias and target identity remain distinct; valid edits affect only intended occurrences; rejected rename has no partial edits; saved LSP and MCP results agree. | Shared-language-service, LSP, MCP, and saved-workspace tests. |

## Related work

Rust's [use declarations](https://doc.rust-lang.org/reference/items/use-declarations.html#as-renames)
and Go's [import declarations](https://go.dev/ref/spec#Import_declarations)
separate a local qualifier from imported identity. They support broader import
forms and do not define Veln runtime or navigation behavior, so they inform
terminology but do not determine these remaining contracts.
