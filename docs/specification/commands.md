---
role: routing
update-when: A CLI command specification page is added, moved, reclassified, or removed.
---

# Commands

This page routes command-specific behavior. Open the smallest page that
matches the command or shared command surface being changed.

## Read First

- Shared package-root selection, source discovery, dependency loading, and
  analysis gates: [command-analysis.md](command-analysis.md).
- Top-level and subcommand help: [command-help.md](command-help.md).
- Machine-readable output routing: [json-output.md](json-output.md).
- Human diagnostics and diagnostic JSON fields: [diagnostics-json.md](diagnostics-json.md).

## Command Pages

- `veln check`: [command-check.md](command-check.md).
- `veln fmt`: [command-fmt.md](command-fmt.md).
- `veln doc`: [command-doc.md](command-doc.md).
- `veln metrics`: [command-metrics.md](command-metrics.md), then
  [metrics-json.md](metrics-json.md) for metrics JSON fields.
- `veln run`: [command-run.md](command-run.md), then [run-json.md](run-json.md) for run JSON fields.
- `veln test`: [command-test.md](command-test.md), then [test-json.md](test-json.md) for test JSON fields.
- `veln repair`: [command-repair.md](command-repair.md), then [repair-json.md](repair-json.md) for repair JSON fields.
- `veln explain`: [command-explain.md](command-explain.md).
- `veln package lock`: [command-package-lock.md](command-package-lock.md).
- `veln lsp`: [command-lsp.md](command-lsp.md).
- `veln mcp`: [command-mcp.md](command-mcp.md), then [mcp.md](mcp.md) for MCP
  protocol behavior.

## Stop Rule

- Use this page only to choose the focused command page.
- Keep command-specific contracts in the focused page, not in this route.
