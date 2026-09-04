---
role: specification
authority: normative
update-when: The `veln mcp` stdio lifecycle, JSON-RPC request validation, workspace project selection, refresh transition, saved project diagnostics, saved navigation tools, MCP resources, tool schemas, or executable MCP cases change.
---

# MCP Workspace Projects, Resources, And Navigation

`veln mcp` runs a Model Context Protocol (MCP) server over standard input and
standard output. Standard output contains only newline-delimited JSON-RPC
messages. End-of-file ends the session successfully.

The current MCP surface contains language-reference, standard-library source,
standard-library package-documentation, admitted direct-dependency source,
and admitted direct-dependency package-documentation resources plus the
`workspace_projects`, `refresh_workspace`, `check_project`, `definition`,
`references`, `search_docs`, and `read_doc` tools.
Initialization advertises
`resources` with
`listChanged: false` and `subscribe: false`, and `tools` with
`listChanged: false`.
The checked declarations under
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

## Resources

`resources/list` returns the complete listed resource set in one response and
omits `nextCursor`. The list is sorted by URI UTF-8 bytes and contains no
duplicate URI. It includes the checked language-reference digest index, one
topic URI for each checked language-reference catalog topic, one
standard-library package-documentation index URI when embedded `std`
package-documentation generation succeeds, one standard-library
package-documentation status URI when that generation fails, one
standard-library source URI for each distribution source retained by the
embedded `std` package snapshot, and one dependency source URI for each
distribution source retained from an admitted direct-dependency package
snapshot. For each admitted direct-dependency snapshot, it also includes one
package-documentation index URI when generation succeeds or one
package-documentation status URI when generation fails. Module and declaration
package-documentation resources are readable only through exact URIs linked
from the package-documentation index or module Markdown; they are not eagerly
enumerated in `resources/list`.
`resources/list` accepts omitted parameters or request metadata. It
rejects a cursor, unknown field, `null`, or non-object parameters with
JSON-RPC invalid params.

The index resource has name `language-index`, title `Veln Language Reference`,
and media type `text/markdown; charset=utf-8`. Topic resources use their
topic identifier as `name`, catalog title as `title`, catalog summary as
`description`, and the same media type.

Standard-library source resources use the canonical `veln-pkg:` URI from the
embedded `std` snapshot virtual-source catalog. Their `name` is the package
relative source path. Their `title` is `Veln standard library source: {path}`.
Their media type is `text/x-veln; charset=utf-8`. They have no description.
Distribution membership controls publication, so private and non-exported
standard-library sources are readable. Test-shaped sources and paths absent
from the embedded distribution source set are not listed.

The embedded standard-library package-documentation resources come from a
checked bundle generated from the embedded `std` snapshot. MCP startup rejects
the bundle when its recorded digest is invalid or its snapshot digest does not
match the retained `std` snapshot. A successful result lists only the
index resource with name `std-documentation-index`, title `Veln
package documentation: std`, and media type `text/markdown; charset=utf-8`.
The index Markdown preserves the package-documentation catalog metadata and
ordered module links. Exact linked module resources preserve module
documentation, source path, references, and ordered declaration links. Exact
linked declaration resources preserve kind, signature, documentation,
contracts, constructors, doctests, expected outputs, aliases, and references
when those fields exist in the catalog. A failed result lists only the status
resource with name `std-documentation-status`; its Markdown preserves the
ordered gate, code, message, and optional source span for each diagnostic.
The Markdown projection does not expose raw manifests, physical paths,
dependency selectors, environment values, or other data excluded from the
package-documentation catalog. Successful standard-library
package-documentation does not publish a separate status resource.

`resources/templates/list` accepts omitted parameters or request metadata and
rejects cursor, unknown field, `null`, or non-object parameters with JSON-RPC
invalid params. It advertises the canonical package-documentation module and
declaration URI forms with Markdown media type. Clients obtain exact readable
module and declaration URIs from index and module Markdown; template variables
are not a URI normalization or discovery surface.

Successful `check_project`, `definition`, and `references` calls on a selected
manifest project atomically admit every valid direct-dependency package
snapshot captured by the same stable saved-project operation. A valid snapshot
has a manifest package name that equals the dependency table key and a captured
package distribution snapshot. The server generates the existing
transport-independent package-documentation result from that retained snapshot
and the parsed manifest used for admission. If generation succeeds, the
dependency documentation index is listed, and its linked module and
declaration resources are available only through exact `resources/read`
requests. If generation fails, only the dependency documentation status
resource is listed and readable. Source resources are retained in either
case. Repeating the same package identity and digest adds no state. A later
digest for the same identity coexists with the earlier snapshot. The server
retains at most 256 package snapshots, including the embedded standard-library
snapshot. If one operation would exceed that limit, the tool returns
`resource_capacity`, admits none of that operation's new snapshots, publishes
no source or documentation resources for the rejected snapshots, and preserves
the previous resource state. Capture failure, validation failure, tool domain
failure, and invalid tool parameters admit no new dependency resources.

Dependency source resources use the canonical `veln-pkg:` URI from the
admitted dependency snapshot virtual-source catalog. Their `name` is the
package-relative source path. Their `title` is
`Veln package source: {identity}: {path}`. Their media type is
`text/x-veln; charset=utf-8`. They have no description. Distribution
membership controls publication, so private and non-exported dependency
sources are readable while test-shaped sources are not listed.

Dependency package-documentation resources use the same `veln-doc:` URI forms,
Markdown media type, renderer-provided names, titles, descriptions, and
allowlisted metadata as the embedded standard-library documentation
projection. A successful dependency result lists only its index resource. A
failed dependency result lists only its status resource. Neither result lists
module or declaration resources eagerly.

`resources/read` accepts one exact `uri` plus optional request metadata. A
successful read returns one complete text content entry with the requested
URI, media type, and deterministic text. Language-reference resource text is
Markdown rendered from the checked catalog artifact. Standard-library
package-documentation resource text is Markdown rendered from the retained
embedded `std` package-documentation result. Dependency
package-documentation resource text is Markdown rendered from the
documentation result retained with the admitted dependency snapshot.
Standard-library source resource text is the exact UTF-8 source text captured
from the embedded package snapshot at server startup. Dependency source
resource text is the exact UTF-8 source text retained from the admitted
saved-project capture. The server does not truncate, paginate, normalize,
regenerate resource content, or fall back to dependency filesystem paths
during a session.

Lookup uses exact URI spelling. Unknown, noncanonical, wrong-digest, and
unknown-topic language-reference URIs fail with the MCP resource-not-found
protocol error and structured domain code `resource_not_found`. Unknown
identity, wrong-snapshot, wrong-documentation-digest, unpublished status,
missing module, missing declaration, malformed, and noncanonical
standard-library package-documentation URIs fail with the same protocol error
and structured domain code. Unknown identity, wrong-snapshot,
wrong-documentation-digest, unpublished status, unpublished index, missing
module, missing declaration, malformed, and noncanonical direct-dependency
package-documentation URIs fail with the same protocol error and structured
domain code. Unknown identity, wrong-digest, malformed, noncanonical,
absent-path, and test-shaped `veln-pkg:` URIs fail with the same protocol
error and structured domain code. Rejected `veln-doc:` and `veln-pkg:` URIs
are not normalized and do not fall back to the filesystem. Missing, nullable,
non-string, non-object, or unknown-field read parameters fail with invalid
params.

Admitted dependency resource snapshots remain available until server shutdown.
Workspace refresh, project removal, dependency removal, dependency relocation,
dependency source edits, and a later digest for the same identity do not remove
or mutate an existing admitted snapshot. The resource set is independent of
language-reference tool calls and failed resource requests. Existing resource
URIs, metadata, ordering, and bytes remain stable until server shutdown. If the
embedded standard-library bundle, manifest validation, or virtual-source
catalog construction fails, `veln mcp` startup fails instead of publishing a
partial resource set.

## Language Reference Tools

`search_docs` searches only the checked language-reference topic resources.
The input requires `query`, accepts optional `scope: "language"`, and accepts
optional integer `limit` from 1 through 50. The default scope is `language`,
and the default limit is 10. The query must contain 1 through 256 Unicode
scalars before normalization and must contain at least one non-whitespace
scalar after normalization. Unknown fields, `null`, non-object input,
unsupported scopes, non-integer limits, and out-of-range limits fail with
invalid params.

Search normalizes searched text and query text to NFC, applies the pinned full
default Unicode case fold used by the portable project contract, trims Unicode
whitespace, and splits query text on Unicode whitespace. The index resource is
not a search candidate. Grammar and example source blocks are not searched.

Search ranks topic results by the first matching tier:

| Rank | Match |
| --- | --- |
| 1 | The complete normalized query equals the identifier or title. |
| 2 | The identifier or title starts with the complete normalized query. |
| 3 | Every token occurs in the title or keywords. |
| 4 | Every token occurs in the summary. |
| 5 | Every token occurs in the body. |

A topic matches the first tier whose field set satisfies that tier. Tokens do
not match across different ranks. Equal-rank results sort by resource URI
UTF-8 bytes. One URI appears at most once. A successful search returns the
effective scope and at most the effective limit of results. Each result
contains `uri`, `title`, `summary`, `excerpt`, `prefix_truncated`, and
`suffix_truncated`. The excerpt comes from the first match in the first
matching field for the winning rank, using field order identifier, title,
keywords in catalog order, summary, and body. It preserves original field
text, contains at most 160 Unicode scalars, and keeps the complete
matched-token source span when that span is not longer than 160 scalars. The
truncation flags report whether original field content was omitted before or
after the excerpt. A search with no match succeeds with an empty `results`
array and no cursor.

`read_doc` accepts one exact `uri` for the checked language index or checked
language topic resources. Success returns `uri`, `name`, `title`, optional
`description`, `mimeType`, and the same complete Markdown `text` as
`resources/read`. Missing, nullable, non-string, non-object, or unknown-field
parameters fail with invalid params. Syntactically valid but unknown,
noncanonical, wrong-digest, non-language, or unknown-topic URIs return an MCP
tool result with `isError: true`, structured code `resource_not_found`, and no
partial document text.

The language tool candidate set, result URIs, read metadata, and read bytes
are independent of workspace project discovery, refresh, and project
analysis. Invalid input and `resource_not_found` failures do not change saved
workspace state, language-reference resource state, or standard-library source
resource state.

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
If dependency resource admission exceeds retained package capacity after
analysis succeeds, `check_project` returns `resource_capacity` and publishes no
partial diagnostics, summary, or analysis metadata.

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
set for captured saved workspace sources. Workspace selections include
functions, type constructors, handler context parameters, handler operation
clause parameters, exact test-companion access to target-private functions, and
unique class-compatible invalid source declaration or binding recovery records.
Eligible package selections include public functions, types, constructors,
schemas, and public function aliases in exported direct-dependency modules and
the embedded standard library. The source must select the exact visible import
or implicit standard-library prelude path required by name resolution. Invalid
casing records, private declarations, non-exported sources, mismatched package
imports, unsupported symbol classes, and package module-segment selections
succeed with `definition: null`.
MCP only exposes the recovery record source range through `definition`.
Prepare-rename, rename edits, and package reference locations are outside the
MCP definition result.
A supported workspace declaration returns one canonical `file:` URI based on
the resolved workspace-base identity and a half-open range. A supported package
declaration returns the canonical retained `veln-pkg:` URI from the package
virtual-source catalog. The returned range is the one-based Unicode-scalar
half-open declaration-token range in that retained source. A valid position
without a supported symbol succeeds with `definition: null`.
If the package declaration resolves through the successful package-documentation
result retained for the same admitted package snapshot, the same location
object includes `packageDocumentationUri`. The value is the exact published
`veln-doc:` declaration URI. A selected constructor uses the owning type
declaration documentation URI. The field is omitted for workspace definitions,
status-only package-documentation results, unpublished declarations,
unsupported symbol classes, and any package location that does not match a
retained package-documentation location for that snapshot.

`references` exposes the shared language-service workspace reference result
for non-recovery workspace functions, types, constructors, value bindings,
handler context parameters, and handler operation clause parameters. It does
not expose recovery, package, dependency, standard-library, virtual, schema,
effect, handler, or effect-operation reference locations. A selected supported
workspace symbol returns sorted canonical `file:` locations for reference sites
only, excluding the selected declaration, plus scope metadata. A valid
position without a supported workspace symbol succeeds with an empty
`references` array. Selected manifest sources report project scope metadata
with `project_wide: true`. Sources outside the selected project-owned source
set report single-file scope metadata with `project_wide: false`.

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
member. After bounded retry exhaustion, `snapshot_changed` references failures
publish no success-only `references` locations or scope member.
If dependency resource admission exceeds retained package capacity after
navigation succeeds, `definition` and `references` return `resource_capacity`
and publish no success-only `definition`, `references`, or scope member.
When `definition` returns a direct-dependency package URI, the same successful
operation has admitted the dependency snapshot. `resources/read` for the exact
returned URI returns the captured UTF-8 source text for that immutable package
snapshot. A capacity failure or `snapshot_changed` failure does not publish a
partial package definition or new package resource state.
When `definition` returns `packageDocumentationUri`, `resources/read` for that
exact URI returns the retained declaration Markdown for the same package
snapshot and package-documentation digest. Later dependency changes can admit a
new source URI and documentation URI for the same package identity. Earlier
returned package source and documentation URIs remain immutable resources.

## Executable Evidence

The `../../examples/specification/mcp/workspace-lifecycle/` case checks
initialization, resource capability advertisement, exact tool declarations,
accepted request metadata, numeric request ID preservation, both tool calls,
invalid tool input, initialization phase errors, invalid initialize
parameters, invalid request IDs, malformed ID-less requests, protocol-only
standard output, and clean end-of-file termination. The
`language-reference-resources` MCP specification case checks resource list and
read success, index and topic Markdown fragments, representative embedded
standard-library source reads, malformed list and read parameters, and
structured `resource_not_found` failures over stdio. It also checks standard
library source metadata shape, private source readability, test-source
exclusion, wrong-digest rejection, and noncanonical `veln-pkg:` rejection. It
checks that every listed resource can be read and that every emitted
language-reference topic URI resolves through `resources/read`. The same case
checks `search_docs` and `read_doc` tool schema advertisement, a normalized
bounded search, compatibility-folded query input, exponent-spelled integer
limits, empty search results, search invalid params, exact `read_doc` index
and topic text, and `read_doc`
`resource_not_found` failures for wrong-digest, noncanonical, non-language,
unknown-topic, and unknown URI classes over stdio. The
`standard-library-package-documentation-resources` MCP specification case
checks listed embedded `std` package-documentation index metadata,
package-documentation resource templates, exact index read, exact
index-linked module read, exact module-linked declaration read, hidden
module and declaration exclusion from `resources/list`, and
wrong-documentation-digest `resource_not_found` over stdio. The
`check-project-diagnostics` MCP specification case checks the
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
The `definition-package-navigation` MCP specification case checks that
`definition` returns a canonical direct-dependency `veln-pkg:` URI and
declaration range, that `definition` returns package-documentation declaration
URIs for an ordinary package function and a constructor-to-type mapping, that
`definition` omits `packageDocumentationUri` while retaining the package
source location for a status-only package-documentation result, that
the returned snapshot source is listed as an MCP resource in the same session,
and that `resources/read` follows the returned package source and
documentation URIs. It also preserves CRLF and non-ASCII UTF-8 text for the
exact returned package URI.
The `references-workspace` MCP specification case checks the advertised
`references` declaration plus declaration-position lookup, recursive calls,
ordinary calls, workspace type references, workspace constructor references,
workspace value-binding references, handler operation clause parameter
references, unsupported schema success, function-shaped recovery exclusion,
invalid positions, and schema-invalid coordinates over stdio.
The `definition-recovery-navigation` MCP specification case checks
`definition` over a unique invalid source declaration recovery record, an
ambiguous invalid source declaration boundary, and valid-symbol precedence.
The shared language-service selector supplies the same recovery boundary for
retained invalid binding records.
The `dependency-source-resources` MCP specification case checks successful
saved-project admission, dependency metadata listing, exported and private
source exact-byte reads, test-source rejection, omitted `nextCursor`, and
structured `resource_not_found` failures over stdio. The
`dependency-package-documentation-resources` MCP specification case checks
successful and status-only direct-dependency documentation publication, listed
index and status metadata, exact index read, exact index-linked module read,
exact module-linked declaration read, package-documentation templates, omitted
`nextCursor`, hidden module and declaration resources, and
`resource_not_found` failures for wrong-documentation-digest and unpublished
direct-dependency documentation URIs over stdio.
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
canonical locations, workspace type, constructor, value-binding, and handler
parameter reference admission, unsupported-symbol success, recovery and package
exclusion, function-shaped recovery exclusion, invalid positions, path
failures, bounded stable-capture retry exhaustion without partial reference
locations or scope metadata, and accepted success and domain-failure result
schemas.
`veln-mcp` unit tests check embedded standard-library startup validation,
checked package-documentation bundle loading, catalog construction failure
propagation, bidirectional completeness between the embedded bundle and MCP
source resources, exact-byte reads for every listed standard-library source,
combined URI-byte ordering, duplicate prevention, lifecycle state preservation
across refresh and analysis, private and non-exported source publication,
absent test-source rejection, direct
dependency admission, identity-and-digest deduplication, same-identity digest
coexistence, retained-byte reads after refresh and dependency replacement,
state preservation after invalid saved navigation, package snapshot capacity
failure atomicity, and mapped `resource_not_found` behavior for unknown,
wrong-digest, malformed, and noncanonical `veln-pkg:` URIs. They also check
the embedded standard-library package-documentation Markdown renderer,
status-only documentation publication, listed index metadata, template
metadata, exact index-linked reads, module and declaration omission from
`resources/list`, byte-for-byte read preservation, and `resource_not_found`
mapping for unknown, noncanonical, wrong-snapshot,
wrong-documentation-digest, and unpublished package-documentation URIs. They
also check direct-dependency package-documentation resource generation from
the admitted snapshot, renderer-equal bytes, listed success indexes,
status-only failure publication, exact linked reads, module and declaration
omission from `resources/list`, rejection of unknown, noncanonical,
wrong-snapshot, wrong-documentation-digest, and unpublished documentation
URIs, deduplication, same-identity snapshot coexistence, capacity atomicity,
and retained reads across refresh and dependency replacement.
The `veln-repo-mcp-standard-library-docs` freshness check regenerates the
bundle from compiler, renderer, and standard-library inputs and rejects any
byte or digest difference from the checked artifact.
Unix-only `veln-mcp` tests also
check canonical resolved-base URI spelling, definition path symlink rejection,
anonymous workspace-base symlink replacement, and that selected
manifest-project analysis does not consume source bytes through project-local
file or directory symbolic links. Linux-only
`veln-mcp` coverage checks that a symlinked descendant `veln.toml` is ignored
as a nested package marker. A `veln-mcp` test also checks that a non-UTF-8
descendant regular manifest still forms a nested package boundary. Non-Linux
`veln-mcp` coverage checks the fail-closed saved snapshot boundary.
