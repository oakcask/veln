---
role: specification
authority: normative
update-when: The veln mcp command startup or stdio boundary changes.
---

# MCP Command

`mcp` starts the agent-facing MCP server over standard input and standard
output using JSON-RPC messages. It does not take source path arguments, and it
does not run the shared package-root analysis used by `check`, `doc`, `fmt`,
`metrics`, `repair`, `run`, `test`, or `package lock`.

Standard output is reserved for MCP protocol messages. End-of-file on standard
input ends the session successfully. Startup failures are command failures
reported by the CLI command wrapper.

The MCP workspace-project selection rules, saved diagnostics, saved
definitions, implemented tools, checked tool schemas, and refresh state
transitions are specified in [mcp.md](mcp.md).
