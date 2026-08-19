---
role: implementation-record
authority: supporting
update-when: The MCP JSONL assertion contract, executable definition fixture, shared navigation capture evidence, or current MCP specification route changes.
---

# Agent Language Services Slice Closure

## Current Behavior Route

Use [../toolchain-test-harness.md](../toolchain-test-harness.md) for the
current `[[mcp_assert]]` harness contract. Use
[../../specification/mcp.md](../../specification/mcp.md) for the current MCP
tool behavior.

This record is historical completion evidence. It is not the source for
current harness or MCP behavior.

## Completed Scope

The slice added response-local MCP JSONL assertions to the toolchain case
harness. The assertion surface selects one response by string or integer
JSON-RPC `id`, applies an RFC 6901 JSON Pointer, and checks complete JSON
equality, exact array length, missing values, or a canonical workspace-file
URI.

The `examples/specification/mcp/definition-workspace/` case now uses
response-local assertions for definition responses 3 through 11. Raw stdout
fragments remain only for initialization and tool discovery text.

The shared navigation capture evidence is compositional:

- `stable_capture_retries_manifest_source_and_path_set_changes_only_three_times`
  and `navigation_capture_retries_descendant_boundary_changes_as_one_attempt`
  check deterministic shared changed-snapshot rejection.
- `definition` calls `capture_navigation_source`, which routes through the
  shared stable navigation capture boundary.
- `definition_rejects_paths_and_changed_workspace_identity` checks
  `snapshot_changed`, `isError: true`, and no success-only definition payload
  for the definition adapter.

## Completion Evidence

The implemented harness contract is covered by the `decoded_mcp_*` and
`manifest_mcp_*` tests in `toolchain_harness.rs`.

The migrated executable example is:

- `examples/specification/mcp/definition-workspace/`

The checked semantic baseline records the migrated `mcp_assert` selector,
path, operation, and operand entries.

## Closure

The active proposal page was removed from `docs/proposals/` after the
response-local harness contract, migrated executable definition fixture,
shared-capture evidence, current harness reference, and checked semantic
baseline were updated.
