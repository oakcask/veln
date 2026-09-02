---
role: specification
authority: normative
update-when: The `veln mcp` stdio lifecycle, JSON-RPC request validation, workspace project selection, refresh transition, saved project diagnostics, saved navigation tools, tool schemas, or executable MCP cases change.
---

# MCP Workspace Projects And Navigation

`veln mcp` runs a Model Context Protocol (MCP) server over standard input and
standard output. Standard output contains only newline-delimited JSON-RPC
messages. End-of-file ends the session successfully.

The current MCP surface contains `workspace_projects`, `refresh_workspace`,
`check_project`, `definition`, and `references`. The checked declarations under
`../../crates/veln-mcp/schemas/mcp/v1/` define the advertised input and result
schemas. The `check_project` result schema closes diagnostics, summary counts,
and the two analysis metadata shapes. Schema failures, unknown input fields,
`null` in non-nullable fields, and non-object inputs produce a JSON-RPC
invalid-params error. The `definition` input requires one source plus positive
JSON integer line and column coordinates. The `references` input uses the same
coordinate contract.
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
including replacement with another regular directory at the same path, the
operation reports `snapshot_changed` instead of reclassifying the root. An
anonymous workspace base replacement also reports `snapshot_changed` instead
of consuming bytes from the replacement directory.

## Project Diagnostics

`check_project` analyzes one immutable saved snapshot. It retries capture when
the selected manifest bytes, owned source path set, owned source bytes, or
dependency manifest and source bytes that analysis can read from path, vendor,
mirror, or locally materialized git inputs change during the operation.
Selected manifest-project capture excludes project-local file and directory
symbolic links from the owned source path set and does not read source bytes
through them. A descendant directory that contains a regular `veln.toml` is a
nested package boundary even when that manifest file is not valid UTF-8.
Captured snapshots include those descendant boundary marker bytes. A
descendant `veln.toml` symbolic link is not a nested package boundary.
Successful analysis uses the captured dependency inputs and does not fall back
to reading uncaptured dependency files. If no stable capture is available, the
tool returns a domain failure with code `snapshot_changed` and no partial
diagnostics. Platforms without handle-relative no-follow saved snapshot
capture fail closed with `snapshot_changed`.

Project selection follows the current workspace selection. An explicit
manifest project must name one selected root and must omit `source`.
If exactly one manifest project is selected, omitting `project` selects that
project. If multiple manifest projects are selected, omitting `project` returns
`project_ambiguous` with the sorted relative roots. An anonymous workspace
requires `project: "."` and exactly one accepted regular `.veln` `source`;
only that file is analyzed. Manifest files added after the last successful
discovery and companion-shaped source names do not expand an anonymous
analysis beyond the requested file. Anonymous requests that omit either the
explicit project or the source return `source_required`.

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
single-file analysis. MCP diagnostic conversion preserves the common
diagnostic contract routed by [diagnostics-json.md](diagnostics-json.md),
including span-less `toolchain.invalid_symbol_case` entries with diagnostic
kind `toolchain` and details for `provider`, `name`, `name_class`, and
`required_initial`.

## Saved Workspace Navigation

`definition` and `references` read one saved workspace-relative regular
`.veln` source and a one-based line and Unicode-scalar column. The line and
column are positive JSON integer values; decimal and exponent spellings that
denote an integer address the same source position as the equivalent plain
integer. If the source is in a selected manifest project's captured
owned-source set, the tool resolves symbols over that project. Any other
accepted source uses anonymous single-file scope. A source below an unselected
descendant manifest is therefore not analyzed with the outer project.

The implemented symbol set is the shared language-service definition selection
set for captured saved workspace sources, including functions, type
constructors, handler context parameters, handler operation clause parameters,
exact test-companion access to target-private functions, and unique
class-compatible invalid source declaration or binding recovery records. MCP
only exposes the recovery record source range through `definition`;
prepare-rename, rename edits, dependencies, and the standard library do not
produce definition locations through this MCP slice.
A supported workspace declaration returns one canonical `file:` URI based on
the resolved workspace-base identity and a half-open range. A valid position
without a supported symbol succeeds with `definition: null`.

`references` exposes only the shared language-service workspace function
reference result. It does not expose type, constructor, local binding, recovery,
package, dependency, standard-library, or virtual reference locations. A
selected workspace function returns sorted canonical `file:` locations for the
function's project-owned reference sites and scope metadata. A valid position
without a supported workspace function succeeds with an empty `references`
array. Selected manifest sources report project scope metadata with
`project_wide: true`. Sources outside the selected project-owned source set
report single-file scope metadata with `project_wide: false`.

LF and CRLF each end one logical line, and neither CRLF terminator scalar is an
addressable position. A line containing `N` Unicode scalars accepts columns 1
through `N + 1`. A terminal newline creates a final empty line at column 1;
an empty file accepts only `(1, 1)`. A token's end is excluded from its
selection. A positive integer line or column that does not address one of these
source positions, including a value larger than the implementation's native
coordinate range, returns `invalid_position`.
Definition and references capture use the same no-follow path checks,
selected-root and workspace-base identity checks, stable double capture,
bounded retry, and
`snapshot_changed` failure as saved project diagnostics. When definition
lookup falls back from a selected outer project to anonymous single-file scope
for a source below a descendant manifest, the ownership decision and the
anonymous source bytes belong to the same stable capture attempt.
`snapshot_changed` definition failures publish no success-only `definition`
member. `snapshot_changed` references failures publish no success-only
`references` locations.

## Executable Evidence

The `../../examples/specification/mcp/workspace-lifecycle/` case checks
initialization, exact tool declarations, accepted request metadata, numeric
request ID preservation, both tool calls, invalid tool input, initialization
phase errors, invalid initialize parameters, invalid request IDs, malformed
ID-less requests, protocol-only standard output, and clean end-of-file
termination. The `check-project-diagnostics` MCP specification case checks the
advertised `check_project` schema and a diagnostic result with a spanless
compiler-owned related note over stdio. The `anonymous-single-file-isolation`
case checks anonymous `check_project` analysis over only the requested source
when another saved source in the same workspace contains a language error.
The `definition-workspace` MCP specification case checks the advertised
`definition` declaration plus representative definition, no-definition,
decimal and exponent integer coordinate spellings, and invalid-position
results plus non-integer decimal and negative-exponent coordinate schema
rejection over stdio. Its response-local assertions bind response IDs 3
through 11 to the expected JSON-RPC result, error, location, cardinality, and
absence observations. Object member order inside those expected result objects
is harness equality evidence, not an MCP output ordering contract.
File-backed expected text and JSON sidecars in that case are harness
reviewability evidence and do not add a distinct MCP response field contract.
Response-local string containment checks in that case are harness evidence
over selected JSON strings and do not add a distinct MCP response field
contract.
The `references-workspace` MCP specification case checks the advertised
`references` declaration plus declaration-position lookup, recursive calls,
ordinary calls, unsupported constructor success, invalid positions, and
schema-invalid coordinates over stdio.
The `definition-recovery-navigation` MCP specification case checks
`definition` over a unique invalid source declaration recovery record, an
ambiguous invalid source declaration boundary, and valid-symbol precedence.
The shared language-service selector supplies the same recovery boundary for
retained invalid binding records.
Table-driven tests in `veln-mcp` check discovery boundaries,
client-root invariance, refresh transitions, failure state preservation,
project/source decision rows, schema failures, path boundaries, anonymous
isolation before refresh, companion-shaped anonymous source names, dependency
snapshots for direct path and locally materialized git inputs, clean analysis,
selected-root symlink and regular-directory replacement, and structured
language diagnostics with spanless related notes and closed related-note
schemas. They also check definition schema rejection, project inference,
anonymous and descendant-manifest isolation, every implemented ordinary symbol
kind, canonical URI spelling, path rejection, stable-capture failure,
no-symbol success, invalid positions including oversized positive integers,
half-open ranges, LF, CRLF, terminal-newline, empty-file, non-BMP scalar
coordinates, extreme positive and negative exponent coordinates, and
non-integer numeric coordinate schema rejection. They also check MCP
definition conversion for unique invalid-name recovery records and unsupported
ambiguous recovery selection.
`veln-mcp` tests check references schema rejection, selected-project
inference, single-file isolation outside selected projects, deterministic
canonical locations, unsupported-symbol success, invalid positions, path
failures, stable-capture failure without partial reference locations, and
accepted success and domain-failure result schemas.
Unix-only `veln-mcp` tests also
check canonical resolved-base URI spelling, definition path symlink rejection,
anonymous workspace-base symlink replacement, and that selected
manifest-project analysis does not consume source bytes through project-local
file or directory symbolic links. Linux-only
`veln-mcp` coverage checks that a symlinked descendant `veln.toml` is ignored
as a nested package marker. A `veln-mcp` test also checks that a non-UTF-8
descendant regular manifest still forms a nested package boundary. Non-Linux
`veln-mcp` coverage checks the fail-closed saved snapshot boundary.
