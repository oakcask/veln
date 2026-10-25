---
role: proposal
update-when: Definition, references, completion, hover, rename, or saved LSP and MCP navigation for explicit import aliases changes.
---

# Explicit Import Alias Source Tools

Explicit import aliases are implemented by the compiler and formatter. The
[source-surface](../specification/source-surface.md),
[name-resolution](../specification/name-resolution.md), and
[diagnostics](../specification/diagnostics-json.md) specifications own that
current behavior. This proposal retains only the source-tool work that is not
implemented yet.

## Planned behavior

Definition on an alias qualifier selects its import declaration. Definition
on a member selects the target member. References and rename include the alias
declaration and qualifier uses in the owning module. They do not rename the
target module, written import path, member, or a same-spelled alias in another
module.

An alias rename to invalid casing, `prelude`, or a conflicting import
qualifier is rejected without partial edits. A target-member rename continues
to include member uses through the alias and leaves the alias unchanged.

Hover and qualified completion present the local qualifier while retaining
the selected package and module provenance. Saved LSP and MCP navigation must
agree with compiler selection. Invalid, colliding, reserved, and
syntax-recovered aliases do not manufacture navigation targets.

This work does not expand which declaration kinds each source tool supports.
It makes the existing supported kinds recognize explicit aliases consistently.

## Acceptance model

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| A11 | Definition on alias and member | Alias selects its import; member selects its target. | Shared language-service, LSP, and MCP definition tests. |
| A11 | References and rename | Only the owning alias occurrences change; target identity and other modules remain unchanged. | Shared reference and rename tests, including rejected edits. |
| A11 | Completion and hover | Results use the local qualifier and retain target provenance. | Completion and hover tests. |
| A11 | Saved navigation | Saved LSP and MCP selection agrees with compiler lookup. | Saved-workspace LSP and MCP tests. |

## Related work and decisions

The inspected Rust Reference
[use-declaration grammar and `as` renames](https://doc.rust-lang.org/reference/items/use-declarations.html#as-renames)
shows that a local rename remains distinct from its target path. The inspected
Go specification
[import declarations](https://go.dev/ref/spec#Import_declarations) separates a
local package qualifier from import identity. Veln source tools must preserve
the same distinction established by Veln's compiler contract; Rust item trees,
globs, public re-exports, and Go dot or blank imports remain outside this work.
