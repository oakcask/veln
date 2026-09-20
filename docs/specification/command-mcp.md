---
role: specification
authority: normative
update-when: The veln mcp command startup or stdio boundary changes.
specification-coverage: usage=#mcp-command; behavior=#mcp-command; limits=#limits-and-errors
---

# MCP Command

`veln mcp` starts the agent-facing MCP server on stdin and stdout using
JSON-RPC messages. It accepts no source path arguments and does not run the
shared package-root analysis used by source commands. Standard output is
reserved for protocol messages.

End-of-file on stdin ends the session successfully. Startup failures are
command failures reported by the CLI wrapper. Tool schemas, workspace
selection, saved diagnostics, saved navigation, and refresh state transitions
are specified by [mcp.md](mcp.md).

## Limits and errors

The command is a protocol endpoint. Clients must send MCP messages on stdin and
must not mix human output with stdout.

## References

MCP transport coverage is in
`crates/veln-cli/tests/toolchain_harness/lsp_transport.rs`; protocol behavior
is specified by [mcp.md](mcp.md).
