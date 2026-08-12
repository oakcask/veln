---
role: specification
authority: normative
update-when: The `veln mcp` stdio lifecycle, JSON-RPC request validation, workspace project selection, refresh transition, tool schemas, or executable MCP cases change.
---

# MCP Workspace Projects And Diagnostics

`veln mcp` runs a Model Context Protocol (MCP) server over standard input and
standard output. Standard output contains only newline-delimited JSON-RPC
messages. End-of-file ends the session successfully.

The current MCP surface contains `workspace_projects`, `refresh_workspace`, and
`check_project`. The checked declarations under
`../../crates/veln-mcp/schemas/mcp/v1/` define the advertised input and result
schemas. The `check_project` result schema closes diagnostics, summary counts,
and the two analysis metadata shapes. Schema failures, unknown input fields,
`null` in non-nullable fields, and non-object inputs produce a JSON-RPC
invalid-params error.
`refresh_workspace` reports the stable `generation_failed` domain failure as an
MCP tool result with `isError: true`.

Request IDs are strings or JSON numbers. A request with a `null` ID is an
invalid JSON-RPC request. Numeric request IDs are returned unchanged in the
response, including fractional, exponent-form, and implementation-large
numbers. A malformed ID-less request object returns `Invalid Request` with a
`null` ID. A structurally valid notification has no response. `initialize`
requires the declared protocol version, client capabilities object, and client
name/version fields. Requests other than `initialize` fail before a successful
`initialize`. A second valid `initialize` in the same session fails. `ping`,
`tools/list`, and `tools/call` accept request metadata as `_meta.progressToken`
when the token is a string or JSON number. `tools/list` also accepts a string
`cursor` parameter; the current server still returns the complete tool list in
one response.

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
separators. If a selected root cannot be represented as UTF-8, discovery fails
instead of returning a lossy root spelling.

## Selection State

The initial generation is zero. `workspace_projects` observes the current
generation and roots without changing them.

| Event | Result | Stored state |
| --- | --- | --- |
| `refresh_workspace` discovery succeeds | Return the replacement roots and next generation. | Replace all roots and advance the generation by one. |
| `refresh_workspace` discovery fails, including an unrepresentable root spelling | Return an MCP tool result with `isError: true` and structured code `generation_failed`. | Preserve both roots and generation. |

Adding, removing, or renaming a manifest has no observable effect until a
successful refresh. `check_project` uses the project kind selected at the last
successful discovery. If a selected manifest root is replaced before analysis,
the operation reports `snapshot_changed` instead of reclassifying the root.

## Project Diagnostics

`check_project` analyzes one immutable saved snapshot. It retries capture when
the selected manifest bytes, owned source path set, owned source bytes, or
direct local dependency manifest and source bytes change during the operation.
Successful analysis uses the captured direct local dependency inputs. If no
stable capture is available, the tool returns a domain failure with code
`snapshot_changed` and no partial diagnostics.

Project selection follows the current workspace selection. An explicit
manifest project must name one selected root and must omit `source`.
If exactly one manifest project is selected, omitting `project` selects that
project. If multiple manifest projects are selected, omitting `project` returns
`project_ambiguous` with the sorted relative roots. An anonymous workspace
requires `project: "."` and exactly one accepted regular `.veln` `source`;
only that file is analyzed. Anonymous requests that omit either the explicit
project or the source return `source_required`.

Tool paths are workspace-relative `/` paths. Absolute paths, paths that escape
the workspace, missing paths, non-regular source paths, non-`.veln` sources,
and source paths that traverse symbolic links return `invalid_path`.
An explicit project outside the selected roots returns `project_not_selected`.
A selected manifest project combined with `source` returns `invalid_query`.

Successful `check_project` results set `isError: false`, even when language
diagnostics have severity `error`. The result includes `schema_version`,
diagnostics using compiler-owned diagnostic identifiers, severities, one-based
Unicode-scalar ranges, related notes, and structured details, plus summary
counts and analysis metadata. The metadata uses `mode: "project"` with
`project_wide: true` for selected manifest-project analysis, and
`mode: "single_file"` with `project_wide: false` and `source` for anonymous
single-file analysis.

## Executable Evidence

The `../../examples/specification/mcp/workspace-lifecycle/` case checks
initialization, exact tool declarations, accepted request metadata, numeric
request ID preservation, both tool calls, invalid tool input, initialization
phase errors, invalid initialize parameters, invalid request IDs, malformed
ID-less requests, protocol-only standard output, and clean end-of-file
termination. The `check-project-diagnostics` MCP specification case checks the
advertised `check_project` schema and a diagnostic result over stdio.
Table-driven tests in `veln-mcp` check discovery boundaries,
client-root invariance, refresh transitions, failure state preservation,
project/source decision rows, schema failures, path boundaries, anonymous
isolation, direct local dependency snapshots, clean analysis, and structured
language diagnostics.
