---
role: implementation-record
update-when: The MCP references tool workspace schema-alias selection, alias identity, saved navigation scope, or schema-alias reference evidence changes.
---

# MCP Saved Workspace Schema-Alias References

## Outcome And Readiness

Let an agent find saved workspace uses of a public schema alias without
merging those uses with its target schema or another alias. This is an
independent slice of
[Agent Language Services](../../proposals/agent-language-services.md).
It requires no package-reference expansion, pagination, client plugin, or MCP
mutation contract.

The prerequisites are implemented:

- Public schema aliases and schema composition are current source behavior in
  [Source Surface](../../specification/source-surface.md); schema operation
  execution is specified by [Execution](../../specification/execution.md).
- Saved workspace schema operation and composition references, stable capture,
  project selection, and result schemas are specified by
  [MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).
- The existing MCP JSONL assertion harness can check response-local locations,
  scope metadata, and missing failure fields. Its contract is in
  [Toolchain Test Harness](../toolchain-test-harness.md).

The capability is implemented by the shared navigation identity and exposed
unchanged through the saved-workspace MCP and LSP adapters. Current behavior
is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md)
and [Editor Support](../../specification/editor-support.md).

## Bounded Contract

An eligible alias is a valid workspace `pub schema` alias whose target resolves
directly to a public schema declaration in the same selected workspace scope.
The target can be local to the alias module or reached through a valid workspace
import. Resolving another alias is outside this slice. Eligibility follows
current schema name resolution and visibility; this slice does not change
which source programs are valid.

Alias identity is its declaring workspace source and declaration span. Two
aliases of one target remain distinct. An alias reference is the written alias
path leaf in a `decode` or `encode` operation, or in a direct or supported
repeated schema-composition payload. The leaf must resolve to that alias under
the existing rules for that source construct. Alias declarations are selectable
but are not reference locations. Alias-target expressions are not references
in this slice.

The existing `references` input and result schemas remain unchanged. Results
use the existing canonical order, deduplication, one-based Unicode-scalar
half-open ranges, workspace `file:` URIs, and navigation scope metadata. Shared
language-service results own symbol identity and reference collection; MCP
adapts saved snapshots. LSP exposes the same reference set for equivalent saved
sources when declaration inclusion is false.

## Acceptance Model

The table preserves the completed slice's acceptance boundary. Exact spans in
executable fixtures are derived from the fixture source and checked per
response, not merely counted.

| Input or state | Required observation | Verification |
| --- | --- | --- |
| Select an eligible alias declaration or one of its operation or composition leaves. | Every selection returns the same alias reference set, excluding the declaration. | Shared language-service tests and an MCP stdio case with exact ordered locations. |
| One alias is used by bare same-module and valid qualified paths in `decode` and `encode`. | Include only the written alias leaves that resolve to the selected alias. A bare imported alias without a qualifying path is not made visible by this feature. | Shared resolution tests and MCP JSONL location assertions. |
| The alias appears in direct schema fields and both currently supported repeated-payload spellings. | Include each resolved alias leaf once, together with operation uses in canonical order. | Shared composition tests plus matching MCP and LSP executable cases. |
| Two aliases target one schema; another module declares the same alias spelling. | Each alias has a separate reference set. Direct target-schema references retain their existing set and do not absorb alias uses. | Identity tests with exact sets, including selection of the target schema. |
| The alias target is an imported public workspace schema. | Include references to the alias without reporting the target token, import tokens, or references to the target under its own name. | Cross-module fixture and shared resolution tests. |
| Target resolution is private, missing, wrong-kind, ambiguous, cyclic, or passes through another alias. | Selection succeeds with an empty reference set; no lexical fallback to a same-spelled schema occurs. | Table-driven shared and MCP negative cases. |
| Select a module qualifier, invalid-casing or recovery alias, dependency alias, or standard-library alias. | Keep successful empty references and the existing scope metadata. | MCP boundary cases retaining unsupported package and recovery coverage. |
| Same-spelled values, types, fields, strings, comments, or a shadowing schema occur beside alias uses. | Exclude occurrences that do not resolve to the selected alias. | Shared symbol-isolation cases. |
| The selected manifest project has multiple owned files, a sibling project, and an unselected descendant package. | Include only selected-project owned-source references and report project-wide scope. | MCP project-isolation tests. |
| The selection has anonymous single-file scope, including an unselected descendant source. | Include only references in that source and retain single-file scope metadata. | `references_keep_anonymous_sources_isolated_for_workspace_schema_alias_selections` and `references_keep_descendant_package_sources_isolated_for_workspace_schema_alias_selections`. |
| Saved capture cannot stabilize within the existing retry limit. | Return `snapshot_changed` without reference locations or success-only scope fields; preserve selected projects and retained package resources. | `references_project_capture_exhausts_retries_for_workspace_schema_alias_selection`. |
| MCP and LSP read identical saved sources with LSP declaration inclusion disabled. | Normalized URI and range sets match, including non-BMP source text before a selected token. | Paired executable MCP/LSP cases and shared coordinate tests. |

## Evidence And Completion

The focused `references-workspace-schema-alias` executable cases under
`examples/specification/mcp/` and `examples/specification/lsp/` own the
adapter-level result evidence. Shared identity and reference coverage lives
with `veln-language-service` navigation tests. Saved-scope and failure coverage
lives with `veln-mcp` references tests.

The old successful-empty assertions changed only for eligible workspace
aliases. Negative coverage remains for aliases outside that boundary.

## Deferred Boundary

This slice adds no alias-chain traversal, package-schema or package-schema-alias
references, package-source result locations, declaration inclusion in MCP,
reference pagination, recovery or casing-neutral selection, definition or
rename support for schema aliases, or source-language changes. Those remain
separate work in the umbrella. Existing definition and rename behavior remains
unchanged. A future widening must explicitly reconsider alias identity and the
unsupported-selection cases rather than silently following target aliases.
