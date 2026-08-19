---
role: implementation-record
authority: supporting
update-when: The MCP JSONL assertion contract, definition-workspace executable evidence, shared-capture evidence boundary, or saved-reference slice status changes.
---

# Agent Language Services Slice Closure

## Current Behavior Route

Use [../toolchain-test-harness.md](../toolchain-test-harness.md) for the
current MCP JSONL assertion contract. Use
[../../specification/mcp.md](../../specification/mcp.md) for the current MCP
tool behavior. Use
`../../../examples/specification/mcp/definition-workspace/` for executable
definition evidence.

This record is historical completion evidence. It is not the source for
current harness or MCP behavior.

## Completed Scope

The slice added response-local assertions for newline-delimited `veln mcp`
stdout. `[[mcp_assert]]` selects one response by string or integer ID, applies
an RFC 6901 JSON Pointer, and checks one decoded operation: complete JSON
equality, exact array length, missing value, or a canonical workspace-file
URI.

The `definition-workspace` executable case now uses response-local assertions
for IDs 3 through 11. Raw stream checks remain only for initialization and tool
discovery text.

The shared capture evidence boundary is compositional. The shared navigation
capture retry test proves that identity or byte changes cannot produce a
stable snapshot. The definition adapter routes through that boundary, returns
`snapshot_changed` with `isError: true`, and omits the success-only definition
payload.

## Completion Evidence

- `toolchain_harness.rs`: `manifest_mcp_*`, `decoded_mcp_*`, and
  `mcp_workspace_uri_*` tests cover the assertion syntax, JSONL decoder,
  response selector, JSON Pointer matrix, equality, array length, missing
  values, duplicate selected IDs, workspace URI rejection matrix, and direct
  rejection for workspace-relative paths that traverse a symlinked component.
- `examples/specification/mcp/definition-workspace/`: checked executable MCP
  case for response IDs 3 through 11.
- `toolchain-case-semantics.baseline`: checked semantic inventory for the
  migrated MCP assertion fields.
- `crates/veln-mcp/src/check_project.rs`: shared stable navigation capture
  retry evidence.
- `crates/veln-mcp/src/server/tests.rs`: definition adapter
  `snapshot_changed` result mapping with no partial success payload.

## Closure

The active proposal page was removed from `docs/proposals/` after the
assertion contract moved into the normative harness reference and the
definition fixture moved into executable response-local evidence.
