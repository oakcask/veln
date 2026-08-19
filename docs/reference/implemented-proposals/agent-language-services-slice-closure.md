---
role: implementation-record
authority: supporting
update-when: The MCP JSONL assertion contract, definition-workspace executable case, shared-capture evidence boundary, or saved-reference slice status is superseded.
---

# Agent Language Services Slice Closure

## Completed Scope

This record preserves the completed proposal that closed the executable
evidence gap before saved workspace function references continue. Current
behavior is specified by [../../specification/mcp.md](../../specification/mcp.md)
and [../toolchain-test-harness.md](../toolchain-test-harness.md), with primary
checked evidence in `../../../examples/specification/mcp/definition-workspace/`
and the harness tests.

The completed change added response-local MCP JSONL assertions to the
toolchain case harness. A `[[mcp_assert]]` selects one string or integer
JSON-RPC response ID, applies an RFC 6901 JSON Pointer, and checks complete
JSON equality, exact array length, missing values, or a canonical workspace
`file:` URI for a regular workspace-relative file.

The `definition-workspace` case now uses those response-local assertions for
IDs 3 through 11. Raw stdout checks remain only for incidental initialization
and tool-discovery text.

The shared capture evidence boundary is compositional. Shared stable-capture
tests prove that source identity or bytes changing during capture cannot
produce a successful snapshot. The definition adapter test proves that the
adapter routes through that boundary and returns `snapshot_changed` with
`isError: true` and no success-only `definition` payload.

## Non-Goals Left Open

This closure did not implement saved workspace references, broaden navigation
symbol support, change Veln name resolution, add dependency or standard-library
reference search, add pagination or retained resources, add documentation
tools, or add client plugins.
