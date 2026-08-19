---
role: implementation-record
authority: supporting
update-when: The MCP JSONL assertion contract, executable definition-workspace response evidence, shared-capture evidence boundary, or current harness reference route changes.
---

# Agent Language Services Slice Closure

## Current Behavior Route

Use [../toolchain-test-harness.md](../toolchain-test-harness.md) for the
current `[[mcp_assert]]` harness contract. Use
[../../specification/mcp.md](../../specification/mcp.md) for current MCP
workspace project, diagnostics, refresh, and definition behavior.

This record is historical completion evidence. It is not the source for
current harness or MCP behavior.

## Completed Scope

The completed slice added response-local MCP JSONL assertions to the
toolchain case harness. The assertion surface selects exactly one response by
string or integer id, evaluates RFC 6901 JSON Pointers, compares complete JSON
values, checks array length, checks missing values, and compares dynamic
workspace file URIs without recording temporary absolute paths in checked
fixtures.

The `definition-workspace` executable MCP case now uses response-local
assertions for response ids 3 through 11. Raw stdout fragments remain only for
incidental initialization and tool declaration text.

The saved-reference adapter was not implemented by this slice. It remains
planned work only when a future proposal is moved into the Ready catalog.

## Completion Evidence

- `manifest_mcp_assertions_validate_id_operation_and_pointer_contracts`
  covers manifest validation for selectors, operations, and JSON Pointer
  syntax.
- `decoded_mcp_jsonl_assertions_cover_ids_pointers_equality_lengths_and_missing_values`
  covers string and integer ids, escaped pointer segments, object-member-order
  independence, ordered arrays, exact array length, and missing values.
- `decoded_mcp_jsonl_rejection_matrix_reports_stream_and_selector_failures`
  covers malformed JSONL, non-object JSONL, missing selected ids, and duplicate
  selected ids.
- `decoded_mcp_workspace_uri_assertions_use_safe_regular_workspace_files`
  covers canonical workspace file URI comparison and unsafe relative path
  rejection.
- `examples/specification/mcp/definition-workspace/` covers the migrated MCP
  definition fixture and its semantic baseline fields.
- `stable_capture_retries_manifest_source_and_path_set_changes_only_three_times`,
  `navigation_capture_retries_descendant_boundary_changes_as_one_attempt`, and
  `definition_rejects_paths_and_changed_workspace_identity` compose the shared
  capture and adapter-level `snapshot_changed` evidence.

## Closure

The active proposal page was removed from `docs/proposals/` after the
assertion contract moved into the normative harness reference, executable
tests, the MCP definition specification case, and the checked semantic
baseline.
