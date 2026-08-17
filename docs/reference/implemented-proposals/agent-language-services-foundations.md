---
role: implementation-record
authority: supporting
update-when: The implemented MCP workspace inventory, saved diagnostics, saved definitions, package snapshot foundations, or named executable evidence is superseded or invalidated.
---

# Agent Language Service Foundations

This record preserves the completion boundary for foundations removed from the
active agent-language-services proposal. Current behavior is defined by the
linked specifications and executable evidence, not by this record.

## Completed Foundations

- `veln mcp` initialization, workspace inventory, atomic refresh, saved project
  diagnostics, and bounded saved workspace definitions are specified in
  [MCP Workspace Projects, Diagnostics, And Definitions](../../specification/mcp.md).
- Shared saved navigation and dependency virtual-source behavior used by LSP
  is specified in [Editor Support](../../specification/editor-support.md).
- Package capture, distribution, portable identity, and digest behavior is
  specified in [Package Snapshot Digests](../../specification/package-snapshots.md).
- Virtual source URI spelling and transport-independent resolution is
  specified in [Package Virtual Sources](../../specification/package-virtual-sources.md).
- Exported documentation catalog generation is specified in
  [Package Documentation Catalogs](../../specification/package-documentation.md).
- The response-local MCP JSONL assertion surface is specified in
  [Toolchain Test Harness](../toolchain-test-harness.md).

## Evidence Routes

The executable MCP cases under `examples/specification/mcp/` cover workspace
lifecycle, anonymous single-file isolation, saved project diagnostics, and
saved workspace definitions. The `definition-workspace` case binds a dynamic
canonical workspace URI, location range, result array cardinality, indexed
content, failure state, and absent fields to individual JSON-RPC response IDs.

Focused tests in `veln-mcp`, `veln-language-service`, and `veln-project` cover
schema boundaries, capture stability, symbol identity, virtual URI spelling,
distribution membership, digest behavior, and atomic failure. The checked
toolchain semantic baseline records executable-case assertion selectors and
operands.

## Remaining Route

Unimplemented references, MCP resources, published reference generation,
conformance, and plugin work remains in
[Agent Language Services](../../proposals/agent-language-services.md).
