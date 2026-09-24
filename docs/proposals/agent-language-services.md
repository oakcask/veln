---
role: proposal
update-when: The remaining package-navigation, workspace-symbol-reference, client-plugin, agent-skill, or conformance-gate scope changes.
---

# Agent Language Services

This umbrella records only the unimplemented follow-up work for language
services. The implemented workspace, direct-dependency, and standard-library
navigation behavior is specified by [Editor Support](../specification/editor-support.md)
and [MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md).

The ready
[MCP Recovery References](mcp-recovery-references.md) slice owns MCP reference
results for the recovery identities already selected by the shared language
service and exposed by LSP. It is independent of new symbol resolution,
package graphs, client packaging, and agent integration.

## Remaining scope

The following work remains planned:

- transitive-dependency navigation;
- remaining package definition and reference symbol classes;
- imported effects, imported handlers, imported effect operations, and other
  workspace symbol reference classes outside the implemented same-module
  effect, effect-operation, and handler boundaries;
- Codex and Claude Code plugin packaging and client-native installation flows;
- shared agent-skill routing and the conformance gate.

Each follow-up must define its own observable acceptance cases and executable
evidence before implementation. It must update the matching current
specification page and remove its completed scope from this inventory.

The ready MCP recovery-reference slice does not define new recovery records or
change recovery selection, reference linking, or LSP behavior. Those contracts
already belong to the shared language service and current editor
specification.

## Remaining acceptance model

The following rows are the umbrella's remaining planning contract. They are
not claims about the current implementation; each row remains incomplete until
its evidence is added to the named current specification and checked route.

| Requirement | Observable acceptance | Required evidence |
| --- | --- | --- |
| Client plugins | Codex and Claude Code plugin manifests bind the active workspace, start the supported MCP contract, and isolate unknown files; the Claude route also completes its LSP lifecycle. Invalid manifests, unavailable servers, failed startup, and unknown-file inputs produce bounded client-visible failures without MCP stdout corruption. | Pinned native-client smoke cases, manifest/schema validation, startup-failure, MCP-failure, Claude-LSP-failure, and unknown-file isolation cases. |
| Skill routing | The shared agent skill searches and reads the published reference for language questions, routes implementation work to the repository authority, and reports unavailable or stale reference artifacts without inventing behavior. | Skill routing cases for known topics, unknown topics, stale catalogs, unavailable resources, and client-specific configuration. |
| Conformance gate | Every requirement has one stable identifier and passing evidence, every evidence route maps to a requirement, and stale or undeclared capabilities fail the gate. | Versioned conformance manifest tests for missing rows, duplicate IDs, orphaned evidence, missing matrix cells, stale artifacts, undeclared capabilities, malformed requests, and plugin mismatches. |

The remainder also includes broader definition navigation, unsupported package
symbol classes, transitive dependencies, recovery and casing-neutral lookup,
and any package-reference boundary not listed as implemented above. These rows
must retain explicit negative and
state-preservation cases: rejected requests do not mutate saved snapshots,
published resources, cursors, or prior successful results.

An implementation slice may be removed from this inventory only after its
current specification page names the executable authority, the evidence
passes independently of proposal prose, and the conformance manifest (when
present) has no missing or orphaned mapping. This preserves the umbrella as a
partial proposal rather than a second current-behavior specification.

## Non-goals

This proposal does not redefine the current `veln lsp` or `veln mcp` protocol,
the saved-workspace capture boundary, standard-library source resources, or
the implemented public standard-library schema composition and operation
references. Those contracts are owned by the current specification pages and
their checked adapter and language-service tests.
