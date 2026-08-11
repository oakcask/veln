---
role: specification
authority: normative
update-when: The `veln mcp` stdio lifecycle, workspace project selection, refresh transition, tool schemas, or executable MCP cases change.
---

# MCP Workspace Projects

`veln mcp` runs a Model Context Protocol (MCP) server over standard input and
standard output. Standard output contains only newline-delimited JSON-RPC
messages. End-of-file ends the session successfully.

The current MCP surface is intentionally limited to `workspace_projects` and
`refresh_workspace`. The checked declarations under
`../../crates/veln-mcp/schemas/mcp/v1/` define both tools' empty-object inputs and
structured results. `refresh_workspace` uses the same checked result schema for
successful refreshes and for the stable `generation_failed` domain failure.
Unknown input fields and non-object inputs produce a JSON-RPC invalid-params
error.

## Workspace Selection

The server resolves its process working directory once as the workspace base.
Client root fields do not change the selection.

| Workspace state | Selected relative roots |
| --- | --- |
| The base contains a regular `veln.toml`. | `.` only. Descendants are not searched. |
| The base has manifests below separate directory branches. | The first manifest directory on each branch, sorted and deduplicated. |
| The base has no manifest below it. | `.` as one anonymous project. |

Implicit discovery does not traverse `.git` or directory symbolic links. An
ordinary `target` directory remains discoverable. Relative roots use `/`
separators.

## Selection State

The initial generation is zero. `workspace_projects` observes the current
generation and roots without changing them.

| Event | Result | Stored state |
| --- | --- | --- |
| `refresh_workspace` discovery succeeds | Return the replacement roots and next generation. | Replace all roots and advance the generation by one. |
| `refresh_workspace` discovery fails | Return a tool error with code `generation_failed`. | Preserve both roots and generation. |

Adding, removing, or renaming a manifest has no observable effect until a
successful refresh.

## Executable Evidence

The `../../examples/specification/mcp/workspace-lifecycle/case.toml` case checks
initialization, exact tool declarations, both tool calls, invalid input,
protocol-only standard output, and clean end-of-file termination. Table-driven
tests in `veln-mcp` check discovery boundaries, client-root invariance, refresh
transitions, and failure state preservation.
