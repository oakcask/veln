---
role: routing
update-when: The user-facing application examples, executable specification case contract, or their subject routes change.
---

# Executable Examples

For a runnable application, start with the
CSV order analyzer at `examples/csv_analyzer/README.md`: a Veln HTTP/2
backend and a TypeScript browser UI with integration checks in CI.

Checked cases under `examples/specification/`
are executable evidence for source behavior, command output, runtime
boundaries, and protocol interactions. Start with that directory's README and
the focused case manifest.

Use examples by subject:

- source inference and diagnostics: `check/`;
- runtime and command output: `run/`;
- package documentation: `doc/`;
- LSP and MCP wire behavior: `lsp/` and `mcp/`;
- metrics and policy output: `metrics/`;
- formatting, diagnostic explanations, test selection, repairs, and package
  locking: `fmt/`, `explain/`, `test/`, `repair/`, and `package/`.

Choose representative cases rather than treating the directory as an inventory.
Rust unit tests own transport-independent APIs such as snapshots, virtual
sources, package documentation, and editor token records; their specification
pages identify those tests. Exact-byte sidecars and workspace-URI directives
exist for protocol fixtures that cannot safely embed machine paths.
