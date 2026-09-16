---
role: implementation-record
update-when: Direct-dependency schema-alias operation selection, alias identity, saved navigation scope, or its MCP and LSP reference evidence changes.
---

# MCP Saved Direct-Dependency Schema-Alias Operation References

## Completed Outcome

Agents can find saved workspace `decode` and `encode` uses of a public schema
alias exported by a direct dependency. The alias remains distinct from its
target schema and other aliases. This completed slice was extracted from
[Agent Language Services](../../proposals/agent-language-services.md).

The required foundations are implemented:

- Direct-dependency schema operation references, package visibility, saved
  capture, project scope, and reference result schemas are specified by
  [MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).
- Workspace schema-alias identity and shared saved-source navigation are
  specified by [Editor Support](../../specification/editor-support.md).
- Public schema aliases are existing syntax in
  [Source Surface](../../specification/source-surface.md).
- Exact response-local location assertions are available through
  [Toolchain Test Harness](../toolchain-test-harness.md).

The implemented capability is alias-specific consumer reference search, not a
new source syntax or a relaxation of identifier casing. Package composition
resolution, alias chains, pagination, plugins, and MCP rename remain outside
this record.

## Bounded Contract

An eligible alias is a valid `pub schema` alias in an exported module of a
retained direct dependency. Its target is a bare name resolving uniquely to a
public schema declaration in the alias's own module. Alias and target names
must satisfy current casing and declaration rules. A schema alias with the
target name makes the target ambiguous, while a same-spelled declaration in an
unrelated namespace does not affect schema-target lookup. Qualified targets,
targets in another module or package, and targets that are aliases are outside
this slice, even if another language capability can resolve them.

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

This table records the acceptance observations covered by the executable
evidence below.
Exact sets must identify source leaves per response, not merely count them.

| Input or state | Required observation | Evidence |
| --- | --- | --- |
| Select either a `decode` or an `encode` leaf for one eligible alias in a project with several owned sources. | Both return the same exact ordered operation-leaf set across those sources. | Shared navigation tests and an MCP JSONL case. |
| Use a full imported module path and its valid implicit leaf alias. | Both select the same alias; bare imported names remain empty. | Import-resolution tests and exact MCP ranges. |
| Two aliases target one schema; two dependencies export the same alias spelling; a workspace alias has that spelling. | Each identity has its own reference set. Selecting the target schema excludes alias uses. | Exact-set identity tests and paired alias/target MCP selections. |
| A schema alias or an unrelated-namespace declaration shares the target name. | The schema-alias collision is successful empty; the unrelated-namespace declaration does not change the eligible alias reference set. | Shared namespace tests, the MCP boundary case, and paired MCP/LSP cases. |
| The alias or target is duplicated, invalid-casing, unresolved, private, wrong-kind, cyclic, or syntax-recovered. | Successful empty references without lexical fallback to a same-spelled valid declaration. | Shared negative table and MCP boundary cases. |
| The alias's target is qualified, belongs to another module or package, or is another alias. | Successful empty references for this bounded slice. | Target-boundary fixtures, including a valid cross-module target and an alias chain. |
| The selected leaf is visible only through a duplicate or syntax-recovered import. | Successful empty references without granting dependency alias visibility. | Shared navigation tests, focused MCP tests, and MCP boundary cases. |
| Select an alias from a non-exported module, a mismatched import, a transitive dependency, or the standard library. | Successful empty references with existing scope metadata. | The graph-aware MCP boundary case and `references_keep_standard_library_schema_aliases_empty_with_project_scope`. |
| Select a composition leaf, alias-target token, module qualifier, import token, or package-source URI. | Preserve the existing unsupported-selection or invalid-path result; do not reinterpret it as an operation leaf. | MCP negative cases, including `references_reject_dependency_schema_alias_target_package_source_selection`. |
| Package sources, composition fields, comments, strings, or unrelated symbol classes contain the alias spelling. | None enter the alias operation-reference set. | Exact-set shared tests and the paired MCP/LSP case. |
| A sibling project or unselected descendant package contains similar operations. | Include only selected-project owned-source references and retain project scope metadata. | MCP project-isolation cases. |
| An anonymous single-file source is selected. | Preserve single-file scope and existing package eligibility; never borrow a neighboring project's dependency context. | MCP anonymous-scope regression. |
| Saved capture exhausts stabilization retries. | Return `snapshot_changed` without locations or success-only scope fields; preserve project selection and retained resources. | MCP capture-failure test for an eligible alias selection. |
| MCP and LSP read identical saved sources, including non-BMP text before a selected leaf. | Normalized URI and range sets match with LSP declaration inclusion disabled. Enabling declaration inclusion does not add the package declaration. | Paired executable MCP and LSP cases. |
| The direct dependency uses path, vendor, mirror, or locally available git capture. | Source kind does not change eligibility, alias identity, or workspace-only results. | Shared saved-capture source-kind cases. |

## Evidence And Completion

Focused protocol cases live under `examples/specification/mcp/` and
`examples/specification/lsp/`. Shared navigation tests live in
`crates/veln-language-service`, with saved-scope and capture-failure tests in
`crates/veln-mcp`. The repository toolchain harness and focused Rust tests run
through the bounded runners described by
[Toolchain Test Harness](../toolchain-test-harness.md).

The dependency-schema operation boundary fixtures contain a direct same-module
alias, an alias chain, a valid qualified cross-module target, a target-name
schema-alias collision, recovered duplicate aliases, invalid imports, and
graph-ineligible alias selections. They preserve the required positive or
successful-empty result for each boundary. The checked
`codec-schema-references` case independently verifies that qualified
cross-module schema-alias targets are valid source language. Focused shared and
MCP tests cover recovered alias declarations that collide with valid
same-module schemas or aliases. They also cover recovered schema declarations
and hidden same-module aliases that block alias eligibility without becoming
navigation targets. The paired dependency-schema-alias cases give exact
written dependency imports precedence over colliding implicit leaf aliases and
keep clean non-exported aliases as blockers against same-spelled exported
schemas. They preserve positive
resolution when an unrelated type shares the target name and separate alias
operation sets from direct target-schema operation sets. Current MCP and editor
specifications own the implemented behavior.
The focused package-source test selects the alias target in the retained
dependency resource and fixes the existing `invalid_path` outcome. The paired
cases select import, comment, and string tokens as successful empty results and
bind every positive decode and encode range to its source URI.

## Deferred Boundary

Cross-module alias targets, alias chains, package schema composition,
standard-library schema aliases, transitive dependencies, package-source
results, MCP declaration inclusion, pagination, recovery and casing-neutral
selection, and new definition or rename support remain separate umbrella
work. Reconsider this boundary when a later slice defines their resolution and
identity acceptance cases.
