---
role: proposal
update-when: Direct-dependency schema-alias operation selection, alias identity, saved navigation scope, or planned MCP and LSP reference evidence changes.
---

# MCP Saved Direct-Dependency Schema-Alias Operation References

## Outcome And Readiness

Let an agent find saved workspace `decode` and `encode` uses of a public schema
alias exported by a direct dependency. Keep that alias distinct from its
target schema and other aliases. This independently actionable slice is
extracted from [Agent Language Services](agent-language-services.md).

The required foundations are implemented:

- Direct-dependency schema operation references, package visibility, saved
  capture, project scope, and reference result schemas are specified by
  [MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md).
- Workspace schema-alias identity and shared saved-source navigation are
  specified by [Editor Support](../specification/editor-support.md).
- Public schema aliases are existing syntax in
  [Source Surface](../specification/source-surface.md).
- Exact response-local location assertions are available through
  [Toolchain Test Harness](../reference/toolchain-test-harness.md).

Current MCP behavior returns successful empty results for package schema
aliases. The new capability is alias-specific consumer reference search, not
a new source syntax or a relaxation of identifier casing. Package composition
resolution, alias chains, pagination, plugins, and MCP rename are not
prerequisites.

## Bounded Contract

An eligible alias is a valid `pub schema` alias in an exported module of a
retained direct dependency. Its target is a bare name resolving uniquely to a
public schema declaration in the alias's own module. Alias and target names
must satisfy current casing and declaration rules. Qualified targets, targets
in another module or package, and targets that are aliases are outside this
slice, even if another language capability can resolve them.

An eligible selection is the written schema-alias path leaf of a parsed saved
workspace `decode` or `encode` expression. The path must resolve to the alias
through a valid direct-dependency import under current import and visibility
rules. Full written module paths and their valid implicit leaf aliases select
the same identity. Bare names do not acquire imported visibility.

Alias identity includes the dependency identity and the alias declaration's
source location. Selecting an eligible leaf returns all eligible operation
leaves with that identity in the selected navigation scope. It excludes the
alias declaration, target declaration, alias-target expressions, composition
leaves, import tokens, module qualifiers, and package-source occurrences.
Direct target-schema references retain their existing identity and do not
absorb uses written through an alias.

The input and result schemas, canonical ordering, deduplication, one-based
Unicode-scalar half-open ranges, workspace `file:` locations, and scope
metadata remain unchanged. Shared language-service navigation owns identity
and collection. MCP reads saved captures; LSP returns the same reference set
for equivalent saved sources with declaration inclusion disabled. Existing
definition and rename behavior remains unchanged.

## Acceptance Model

This table is the planned acceptance authority. The evidence below must be
added during implementation; these rows do not claim passing coverage.
Exact sets must identify source leaves per response, not merely count them.

| Input or state | Required observation | Planned evidence |
| --- | --- | --- |
| Select either a `decode` or an `encode` leaf for one eligible alias in a project with several owned sources. | Both return the same exact ordered operation-leaf set across those sources. | Shared navigation tests and an MCP JSONL case. |
| Use a full imported module path and its valid implicit leaf alias. | Both select the same alias; bare imported names remain empty. | Import-resolution tests and exact MCP ranges. |
| Two aliases target one schema; two dependencies export the same alias spelling; a workspace alias has that spelling. | Each identity has its own reference set. Selecting the target schema excludes alias uses. | Exact-set identity tests and paired alias/target MCP selections. |
| The alias or target is duplicated, invalid-casing, unresolved, private, wrong-kind, cyclic, or syntax-recovered. | Successful empty references without lexical fallback to a same-spelled valid declaration. | Shared negative table and MCP boundary cases. |
| The alias's target is qualified, belongs to another module or package, or is another alias. | Successful empty references for this bounded slice. | Target-boundary fixtures, including a valid cross-module target and an alias chain. |
| Select an alias from a non-exported module, a mismatched import, a transitive dependency, or the standard library. | Successful empty references with existing scope metadata. | Graph-aware MCP boundary cases; inject a standard-library alias in a focused server test if needed. |
| Select a composition leaf, alias-target token, module qualifier, import token, or package-source URI. | Preserve the existing unsupported-selection or invalid-path result; do not reinterpret it as an operation leaf. | MCP negative cases and shared selection tests. |
| Package sources, composition fields, comments, strings, or unrelated symbol classes contain the alias spelling. | None enter the alias operation-reference set. | Exact-set shared and adapter cases. |
| A sibling project or unselected descendant package contains similar operations. | Include only selected-project owned-source references and retain project scope metadata. | MCP project-isolation cases. |
| An anonymous single-file source is selected. | Preserve single-file scope and existing package eligibility; never borrow a neighboring project's dependency context. | MCP anonymous-scope regression. |
| Saved capture exhausts stabilization retries. | Return `snapshot_changed` without locations or success-only scope fields; preserve project selection and retained resources. | MCP capture-failure test for an eligible alias selection. |
| MCP and LSP read identical saved sources, including non-BMP text before a selected leaf. | Normalized URI and range sets match with LSP declaration inclusion disabled. | Paired executable MCP and LSP cases. |
| The direct dependency uses path, vendor, mirror, or locally available git capture. | Source kind does not change eligibility, alias identity, or workspace-only results. | Shared saved-capture source-kind cases. |

## Evidence And Completion

Add focused protocol cases under `examples/specification/mcp/` and
`examples/specification/lsp/`. Extend shared navigation tests in
`crates/veln-language-service` and saved-scope and capture-failure tests in
`crates/veln-mcp`. Run the affected cases with the repository toolchain harness
and focused Rust tests through the bounded runners described by
[Toolchain Test Harness](../reference/toolchain-test-harness.md).

The existing dependency-schema operation boundary fixtures contain a direct
same-module alias and an alias chain. Change the former's empty expectation
only after adding exact positive evidence. Retain the latter's empty result
and coverage for excluded package schema classes. Completion requires every
acceptance row, updated current MCP and editor specifications, and retirement
of this proposal to the implemented records.

## Deferred Boundary

Cross-module alias targets, alias chains, package schema composition,
standard-library schema aliases, transitive dependencies, package-source
results, MCP declaration inclusion, pagination, recovery and casing-neutral
selection, and new definition or rename support remain separate umbrella
work. Reconsider this boundary when a later slice defines their resolution and
identity acceptance cases.
