---
role: implementation-record
authority: supporting
update-when: The MCP JSONL assertion contract, definition-workspace executable MCP case, shared navigation capture evidence, or saved workspace references slice boundary is superseded.
---

# Agent Language Services Slice Closure

This record preserves the completed slice that closed the executable evidence
gap before saved workspace function references. Current behavior is specified
by [../toolchain-test-harness.md](../toolchain-test-harness.md),
[../../specification/mcp.md](../../specification/mcp.md), and the checked MCP
examples.

## Completed Scope

- The toolchain harness accepts `[[mcp_assert]]` for newline-delimited MCP
  JSON-RPC stdout. It selects one response by string or integer ID and checks
  RFC 6901 JSON Pointer paths with complete JSON equality, string containment,
  array length, missing-path assertions, and canonical workspace-file URI
  assertions.
- The `../../../examples/specification/mcp/definition-workspace/` case uses
  response-local assertions for response IDs 3 through 11. Raw stdout fragments
  remain only for initialization and tool discovery text.
- The shared navigation capture evidence composes with adapter evidence for
  `snapshot_changed`: the stable capture tests check deterministic capture
  changes, `definition::definition` routes through `capture_navigation_source`,
  and
  `definition_rejects_paths_and_changed_workspace_identity` checks the
  adapter-visible `snapshot_changed` failure, `isError: true`, and absence of
  the success-only definition payload.

## Evidence Map

| Claim | Checked evidence |
| --- | --- |
| MCP JSONL assertions reject malformed and non-object lines, missing and duplicate IDs, invalid pointers, non-string containment targets, non-array length targets, and unsafe workspace URI operands. | `decoded_mcp_jsonl_*` and `manifest_mcp_assertions_*` tests in `toolchain_harness.rs` |
| MCP JSONL assertions compare reordered objects as equal and ordered arrays as ordered values. | `decoded_mcp_jsonl_assertions_cover_success_matrix` |
| The definition executable case binds each required response observation to its response ID. | `examples/specification/mcp/definition-workspace/case.toml` |
| Stable capture retries deterministic source, manifest, identity, and descendant-boundary changes without accepting an unstable snapshot. | `stable_capture_retries_manifest_source_and_path_set_changes_only_three_times` and `navigation_capture_retries_descendant_boundary_changes_as_one_attempt` |
| Definition capture uses the shared stable navigation capture boundary. | `definition::definition` calls `capture_navigation_source`, which routes through `capture_stable_navigation_source_with` |
| A changed definition capture publishes `snapshot_changed` without partial success content. | `definition_rejects_paths_and_changed_workspace_identity` |

## Remaining Boundary

Saved workspace references remain outside this completed slice. A later target
can select that work only after the proposal catalog moves it into Ready.
