---
role: implementation-record
update-when: Direct-dependency schema operation reference selection, saved navigation scope, package identity, or MCP and LSP reference evidence changes.
---

# MCP Saved Direct-Dependency Schema Operation References

## Implemented Outcome

Let an agent find saved workspace `decode` and `encode` uses of a public schema
from a direct dependency. This completed slice belongs to
[Agent Language Services](../../proposals/agent-language-services.md). It
required no new source syntax, tool schema, pagination, package composition
resolution, or client plugin contract.

The implementation reuses these prerequisites:

- Package schema definition selection, retained direct-dependency captures,
  saved project scope, and reference result schemas are specified by
  [MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).
- Shared saved-source navigation and LSP adaptation are specified by
  [Editor Support](../../specification/editor-support.md).
- The JSONL harness supports exact response-local location assertions; see
  [Toolchain Test Harness](../toolchain-test-harness.md).

Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md)
and [Editor Support](../../specification/editor-support.md). Executable protocol
evidence lives in `examples/specification/mcp/references-dependency-schema-operation/`
and `examples/specification/lsp/references-dependency-schema-operation/`.
Successful-empty boundary evidence lives in
`examples/specification/mcp/references-dependency-schema-operation-boundaries/`.

## Bounded Contract

An eligible selection is the schema path leaf of a saved workspace `decode`
or `encode` expression that resolves to a public schema declaration in an
exported module of a retained direct dependency. Eligibility follows the
existing package definition visibility and import rules. All currently
admitted direct-dependency source kinds are eligible; no new dependency
materialization is introduced.

For an eligible selection, return all such operation leaves in the selected
navigation scope that resolve to the same package schema declaration. Identity
includes the dependency and declaring source location, not only its spelling.
Results contain workspace `file:` locations only. They exclude the declaration,
package source occurrences, schema composition fields, schema aliases and
alias-target expressions, import tokens, and module qualifiers.

Existing ordering, deduplication, one-based Unicode-scalar half-open ranges,
project selection, and scope metadata apply. Input and result schemas remain
unchanged. Shared language-service resolution owns identity and collection;
MCP uses saved captures. LSP with declaration inclusion disabled exposes the
same set for equivalent saved sources.

## Acceptance Model

These cases record the implemented acceptance boundary. Exact location
assertions identify source leaves rather than only count them.

| Input or state | Required observation | Verification |
| --- | --- | --- |
| Select either a `decode` or an `encode` leaf resolving to one eligible schema. | Both return the same exact ordered set of operation leaves across selected-project owned sources. | Shared navigation tests and an MCP JSONL specification case. |
| Use the full written imported module path and its valid implicit leaf alias. | Both spellings resolve under existing import rules and contribute to one schema identity. | Shared import tests and exact MCP ranges. |
| Two dependencies export the same module and schema names; a workspace schema also has that name. | Each selection returns only uses resolving to its own declaration. | Dependency identity and workspace shadowing fixtures. |
| Private, non-exported, mismatched-import, transitive, invalid-casing, recovery, or unresolved schema leaves are selected. | Successful empty references; no lexical fallback to an eligible same-spelled schema. | Shared negative table and MCP graph-aware boundary cases. |
| Select a package schema alias, package schema alias chain, composition leaf, or module qualifier. | Keep successful empty references for those unsupported selections. | MCP JSONL and focused server boundary assertions. |
| Select a standard-library schema. | Keep a successful empty reference result with the selected project scope. | Focused MCP server test with an injected public standard-library schema. |
| An eligible schema also occurs in package sources, composition fields, aliases, comments, strings, or unrelated symbol classes. | None of those occurrences enter the operation reference set. | Exact-set shared and adapter tests. |
| A sibling selected project and an unselected descendant package contain similar operations. | Include only the requested project's owned sources and retain project-wide scope metadata. | MCP project-isolation cases. |
| An anonymous single-file source is selected. | Preserve current single-file scope and package eligibility; do not borrow a neighboring project's dependency context. | MCP anonymous-scope regression case. |
| Saved capture exhausts its existing stabilization retries. | Return `snapshot_changed` without locations or success-only scope fields; preserve project selection and retained resources. | MCP capture failure test for an eligible schema selection. |
| MCP and LSP read identical saved sources, including non-BMP text before an operation leaf. | Normalized URI and range sets match when LSP declaration inclusion is false. | Paired MCP and LSP executable cases. |
| Resolve a supported operation through each admitted direct-dependency source kind. | Source kind does not alter identity or the workspace-only result boundary. | Shared saved-capture cases for path, vendor, mirror, and locally available git sources. |

## Evidence

Focused executable cases under `examples/specification/mcp/` and
`examples/specification/lsp/`, shared navigation tests in
`crates/veln-language-service`, and scope and capture-failure tests in
`crates/veln-mcp` verify the acceptance rows. Successful-empty checks preserve
the excluded package schema classes. A focused MCP server test injects a public
standard-library schema and verifies a successful empty result with the
selected project scope.

## Deferred Boundary

Package schema composition references, standard-library schema references,
package schema aliases, alias-chain traversal, transitive dependencies,
package-source results, pagination, recovery and casing-neutral references,
and new definition or rename behavior remain outside this slice. In
particular, package and mixed-import composition collision rules do not block
operation-only lookup. They need their own acceptance contract before package
composition references become selectable work.
