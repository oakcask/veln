---
role: implementation-record
update-when: The MCP references tool schema, saved navigation capture, shared workspace function-reference result, or executable MCP references cases change.
---

# MCP Saved Workspace Function References

The completed slice exposes the shared language-service workspace function
reference result through `veln mcp` as the `references` tool. Current behavior
is specified by [MCP Workspace Projects And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-workspace` executable MCP specification case checks tool
  advertisement, declaration-position lookup, recursive and ordinary calls,
  unsupported constructor success, function-shaped recovery exclusion, invalid
  positions, and schema-invalid coordinates.
- `veln-mcp` server tests check selected-project inference, single-file
  isolation outside selected projects, deterministic canonical locations,
  unsupported-symbol success, function-shaped recovery exclusion, path and
  coordinate failures, result schema acceptance, and stable-capture failure
  without partial reference locations.

Out-of-scope agent language-service work remains planned only when a separate
Ready proposal selects it from [../../proposals/README.md](../../proposals/README.md).
