---
role: proposal
update-when: Direct-dependency schema composition reference selection, import resolution, adapter parity, or planned acceptance evidence changes.
---

# Direct-Dependency Schema Composition References

## Scope And Readiness

Extend saved-source reference lookup for a public schema in an exported,
retained direct-dependency module to include composition leaves in the selected
project. This lets a caller find embedded uses of a dependency schema together
with its existing `decode` and `encode` uses.

The package capture, declaration identity, operation-reference lookup, and
workspace composition foundations are implemented in
[Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and [MCP Navigation](../specification/mcp.md#saved-workspace-navigation).
Package composition is still explicitly excluded there. This slice needs no
new source syntax, MCP tool, pagination contract, or plugin support and is ready
independently of the remaining
[agent-language-services inventory](agent-language-services.md).

## Acceptance Contract

This table owns the planned contract. Evidence names below are intended
additions, not existing passing cases. A composition leaf is the schema name
in a direct field, `Repeat(count, module::Packet)`, or
`[module::Packet; count]` payload. The selected identity includes the retained
dependency and schema declaration, not only its name or module spelling.

| Input or condition | Required observation | Planned evidence |
| --- | --- | --- |
| A saved project imports an exported dependency module containing public schema `Packet`; owned sources contain direct, `Repeat`, array-payload, `decode`, and `encode` uses. | Selecting any supported composition or operation leaf returns the same deduplicated union of all five forms for that declaration. Each range covers only the schema-name leaf. | Paired `references-dependency-schema-composition` LSP and MCP executable cases. |
| A composition uses the full written imported module path or its unique implicit leaf alias. | Both select the same package declaration. Imports shared across owned sources of the same explicit module identity have the visibility specified by [Name Resolution](../specification/name-resolution.md). Importing the module does not introduce a bare schema name. | Shared language-service import and cross-source cases. |
| A full written import path also matches another import's implicit leaf alias. | The unique valid exact full import wins. Conflicting exact imports select no identity and do not fall back to an implicit alias. With no exact match, an implicit alias selects an identity only when its candidate is unique across workspace and package imports. Duplicate or recovered imports do not grant visibility. | Import-order permutations with dependency/dependency and workspace/dependency collisions, including exact-path conflicts. |
| A workspace schema, another direct dependency, or another module has the same schema name. | Only uses resolved to the selected declaration appear. Package composition does not enter the workspace schema's set. | Shared identity-isolation cases extending the existing package/workspace collision tests. |
| The dependency's own sources contain composition or operation uses; another selected project uses the same dependency. | Neither source set enters the selected project's results. Results contain only owned workspace `file:` locations. | Shared package-source exclusion and MCP selected-project isolation cases. |
| LSP requests either declaration policy; MCP requests references for the same saved source. | Package declarations are excluded for both LSP policies. With no overlays, adapter results agree after coordinate conversion, including a non-BMP prefix. Existing ordering and scope metadata remain unchanged. | Paired protocol cases with exact ranges and both LSP declaration policies. |
| The dependency is retained through path, vendor, mirror, or locally available git input. | Equivalent retained declarations have the same reference behavior without fetching a dependency during navigation. | Shared retained-source-kind matrix. |
| A composition selects a private or non-exported schema, a standard-library schema, a transitive dependency, a schema alias or alias chain, an unresolved or mismatched import, or a recovered or invalid-casing leaf. | It contributes no direct-dependency schema reference and does not fall back to a same-spelled workspace schema. Existing operation and workspace-alias behavior stays intact. | Shared boundary matrix and MCP successful-empty selection cases. |
| A composition-looking spelling is in an import, module qualifier, unrelated name class, string, or comment. | It does not enter the schema reference set. | Shared exact-set noise cases. |

## Boundaries

This slice does not add package schema-alias composition, standard-library
schema references, transitive navigation, pagination, recovery navigation,
casing-neutral lookup, or rename. It does not widen definition coverage or
change the existing LSP overlay policy and MCP saved-capture failure contract.
Existing declaration-policy differences from the umbrella's final capability
remain intentional until a separate proposal changes them.

## Verification And Completion

The acceptance table is a reviewable model until implementation adds checked
fixtures. Shared cases belong with
`crates/veln-language-service/src/tests/dependencies_schema_references.rs` and
`navigation_schema_references.rs`. Protocol cases belong under
`examples/specification/lsp/` and `examples/specification/mcp/`; use the
existing specification harness described in `examples/specification/README.md`.
Run focused Rust and protocol harness tests through `bash scripts/agent-test`
with the appropriate package and case filters.

Completion requires passing evidence for every row, preservation of existing
workspace and dependency operation-reference cases, and current-behavior
updates to the two specification pages linked above. Remove this proposal and
its Ready entry after those artifacts cover the implemented slice. Keep the
remaining package-alias, standard-library, and broader navigation work in the
umbrella inventory.
