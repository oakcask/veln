---
role: proposal
update-when: Explicit import alias hover provenance or qualified completion behavior or its planned verification changes.
---

# Explicit Import Alias Source Presentation

The compiler and shared navigation service implement explicit import aliases.
Current syntax, lookup, visibility, lowering, navigation, and saved adapter
behavior are specified by
[Source Surface](../specification/source-surface.md#usage-and-declaration-forms),
[Name Resolution](../specification/name-resolution.md#explicit-import-aliases),
[Editor Support](../specification/editor-support.md#explicit-import-alias-navigation),
[MCP](../specification/mcp.md#saved-workspace-navigation), and
[Execution Boundary](../specification/execution.md#runtime-readiness-and-host-boundaries).
This proposal contains only the remaining source-presentation work.

## Remaining source-tool scope

- LSP hover displays the local qualifier while retaining package and target
  module provenance.
- Qualified LSP and MCP completion uses the local qualifier while retaining
  the same package and target-module provenance.

This work does not expand the declaration kinds supported by navigation or
completion.

## Acceptance model

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| A11 | Hover and qualified completion on a valid explicit alias and on a member reached through it | Presentation uses the written local qualifier and retains package and target-module provenance; same-spelled aliases in other modules remain distinct; invalid or recovered aliases produce no alias-qualified presentation. | Shared presentation tests plus checked LSP hover and LSP/MCP completion projections. |

## Related work

### Import bindings

- **Identity and locator:** Rust's
  [use declarations](https://doc.rust-lang.org/reference/items/use-declarations.html#as-renames)
  and Go's
  [import declarations](https://go.dev/ref/spec#Import_declarations) are the
  primary language references.
- **Relevant finding:** Both languages let an import introduce a local name
  that differs from the imported path or package identity. Go uses that local
  name as the qualifier for imported declarations.
- **Applicability and limits:** Neither reference defines editor hover,
  completion metadata, or Veln's MCP result model.
- **Design consequence:** Presentation must show the local qualifier without
  erasing the target identity needed for provenance.

### Language Server Protocol

- **Identity and locator:** Microsoft's versioned
  [Language Server Protocol 3.17 specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
  defines hover and completion requests and responses.
- **Relevant finding:** Both operations are position-based, while completion
  items and hover contents can carry language-defined presentation detail.
- **Applicability and limits:** The protocol does not decide whether Veln
  displays a local alias, target module, or package provenance, and it does not
  define MCP completion.
- **Design consequence:** Shared presentation must choose the qualifier and
  provenance once, with adapters projecting that result.

### Versioned implementation

- **Identity and locator:** `rust-analyzer` at revision
  [`0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49`](https://github.com/rust-lang/rust-analyzer/tree/0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49)
  has tests for
  [hover on renamed imports](https://github.com/rust-lang/rust-analyzer/blob/0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49/crates/ide/src/hover/tests.rs#L7127-L7184)
  and
  [completion under re-exported names](https://github.com/rust-lang/rust-analyzer/blob/0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49/crates/ide-completion/src/tests/special.rs#L408-L466).
- **Relevant finding:** Hover and completion can expose a written alias while
  retaining access to the target declaration.
- **Applicability and limits:** Rust re-exports, crates, macros, and namespaces
  differ from Veln, and the implementation does not define an MCP view.
- **Design consequence:** Veln should retain alias and target provenance in
  shared presentation data instead of reconstructing it in each adapter.

### Unsupported questions and search scope

The review inspected the official LSP operation contracts, the Rust and Go
import references in the local bibliography cache, and the pinned
`rust-analyzer` implementation above. No inspected source establishes Veln's
exact hover text, completion detail fields, MCP projection, or invalid-alias
presentation boundary. A11 therefore keeps those Veln-specific observations
and their planned executable evidence explicit.
