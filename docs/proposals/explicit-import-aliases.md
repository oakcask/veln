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

### Import bindings

- **Identity and locator:** Rust's
  [use declarations](https://doc.rust-lang.org/reference/items/use-declarations.html#as-renames)
  and Go's
  [import declarations](https://go.dev/ref/spec#Import_declarations) are the
  primary language references.
- **Relevant finding:** Both languages let an import introduce a local name
  that differs from the imported path or package identity. Go also makes the
  local package name the qualifier for imported declarations.
- **Applicability and limits:** Both languages have import forms and namespace
  rules that Veln does not adopt. Neither reference specifies editor identity,
  rename scope, or an MCP view.
- **Design consequence:** A11 treats the alias binding and target declaration
  as separate navigation identities. Import syntax alone does not decide which
  identity a source-tool operation selects.

### Language Server Protocol

- **Identity and locator:** Microsoft's versioned
  [Language Server Protocol 3.17 specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
  defines definition, references, hover, completion, prepare-rename, rename,
  workspace-edit, and document-synchronization messages.
- **Relevant finding:** These requests are position-based and are computed on
  synchronized document state. References can include the declaration;
  prepare-rename can reject an unsupported position; rename returns a
  workspace edit or an error for an invalid new name. The protocol leaves
  symbol identity, hover contents, completion meaning, and language-specific
  rename validity to the server.
- **Applicability and limits:** LSP determines the LSP wire contract but does
  not decide whether an alias occurrence denotes its local binding or its
  target. It does not define Veln's MCP representation, saved-workspace model,
  or compiler selection.
- **Design consequence:** A11 uses one language-service selection result for
  all operations, validates a rename before returning any edits, and tests LSP
  transport separately from Veln's alias semantics. Protocol conformance alone
  is not acceptance evidence.

### Versioned implementations

- **Identity and locator:** rust-analyzer at revision
  [`0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49`](https://github.com/rust-lang/rust-analyzer/tree/0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49)
  has tests for
  [definition on `use` aliases](https://github.com/rust-lang/rust-analyzer/blob/0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49/crates/ide/src/goto_definition.rs#L1276-L1302),
  [hover on renamed imports](https://github.com/rust-lang/rust-analyzer/blob/0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49/crates/ide/src/hover/tests.rs#L7127-L7184),
  and
  [completion under re-exported names](https://github.com/rust-lang/rust-analyzer/blob/0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49/crates/ide-completion/src/tests/special.rs#L408-L466).
  Its rename implementation explicitly
  [rejects aliases because its HIR does not track alias identity](https://github.com/rust-lang/rust-analyzer/blob/0c4d74686b21f2b3bd8872b6d3cf36ad2e740d49/crates/ide/src/rename.rs#L316-L365).
- **Relevant finding:** Definition, hover, and completion can expose a written
  alias while still resolving the target, but rename cannot safely distinguish
  the two without a represented alias identity.
- **Applicability and limits:** Rust re-exports, crates, macros, and namespaces
  differ from Veln modules. The inspected implementation is useful evidence of
  the failure mode, not a behavior to copy.
- **Design consequence:** Veln must preserve an addressable alias binding
  through selection and usage search instead of inferring alias identity from
  spelling after target resolution. The same distinction drives definition,
  references, rename, hover, and qualified completion.

- **Identity and locator:** `rust-analyzer-mcp` at revision
  [`ab6646469773992a36a1716622a9431ed9caa401`](https://github.com/zeenix/rust-analyzer-mcp/tree/ab6646469773992a36a1716622a9431ed9caa401)
  exposes MCP navigation through one LSP client. It
  [refreshes open documents from disk before workspace rename](https://github.com/zeenix/rust-analyzer-mcp/blob/ab6646469773992a36a1716622a9431ed9caa401/src/mcp/server.rs#L90-L116)
  and
  [refuses incomplete-index rename results](https://github.com/zeenix/rust-analyzer-mcp/blob/ab6646469773992a36a1716622a9431ed9caa401/src/mcp/handlers.rs#L290-L340).
- **Relevant finding:** An MCP adapter can preserve one semantic engine and
  explicitly synchronize saved files before a workspace-wide edit. Refusing a
  result from an incomplete index avoids presenting a partial rename as whole.
- **Applicability and limits:** This adapter is for Rust and forwards LSP
  results; it does not prove that independently implemented Veln LSP and MCP
  encoders agree, and it does not define Veln compiler selection.
- **Design consequence:** Saved MCP and LSP cases must exercise the same Veln
  selection and workspace snapshot, then compare semantic destinations and
  edits rather than only checking that both protocols return successful
  responses.

### Unsupported questions and search scope

The review inspected the official LSP 3.17 operation and synchronization
contracts, the Rust and Go import references already present in the local
bibliography cache, and the pinned rust-analyzer and `rust-analyzer-mcp`
implementations above. No inspected external source establishes Veln's exact
module-local alias-reference set, recovered-syntax exclusion, reserved-name and
collision policy, provenance display, or compiler/LSP/MCP equality. Those are
Veln-specific A11 requirements and remain supported by the acceptance model and
planned repository tests, not by analogy to the external sources.
