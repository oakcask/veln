---
role: specification
authority: normative
specification-coverage: usage=#workspace-selection; behavior=#resources; limits=#selection-state
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
JSON integer line and column coordinates. An initial `references` input uses
the same coordinate contract; a continuation uses only its cursor.
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

## Documentation Tools

`search_docs` searches checked language-reference topics and retained
successful package-documentation catalogs. The input requires `query`, accepts
optional `scope`, and accepts optional integer `limit` from 1 through 50. The
accepted scopes are `language`, `stdlib`, `package`, and `all`. The default
scope is `language`, and the default limit is 10. The query must contain 1
through 256 Unicode scalars before normalization and must contain at least one
non-whitespace scalar after normalization. Unknown fields, `null`, non-object
input, unsupported scopes, non-integer limits, and out-of-range limits fail
with invalid params.

The scope selects the candidate set:

| Scope | Candidate set |
| --- | --- |
| `language` | Checked language-reference topic resources. |
| `stdlib` | Retained successful embedded `std` package-documentation index, module, and declaration resources. |
| `package` | Retained successful non-standard direct-dependency package-documentation index, module, and declaration resources. |
| `all` | The union of `language`, `stdlib`, and `package`. |

Search normalizes searched text and query text to NFC, applies the pinned full
default Unicode case fold used by the portable project contract, trims Unicode
whitespace, and splits query text on Unicode whitespace. The index resource is
not a language search candidate. Status-only package-documentation results,
package source resources, workspace-package documentation, unpublished package
documentation, grammar blocks, example source blocks, doctest code, expected
output, diagnostic text, physical paths, and rendered Markdown decoration are
not search candidates or searched fields.

Search ranks language-topic results by the first matching tier:

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

Package-documentation candidates rank by the first matching tier:

| Rank | Match |
| --- | --- |
| 1 | The complete normalized query equals the package identity, module ID, declaration ID, or rendered title. |
| 2 | The package identity, module ID, declaration ID, or rendered title starts with the complete normalized query. |
| 3 | Every token occurs in the rendered title, package keywords, or the candidate-specific name field. For an index candidate, the name field is the manifest package name. For a module candidate, it is the module name. For a declaration candidate, it is the declaration name. |
| 4 | Every token occurs in the candidate summary or declaration signature. The summary is the manifest package description for an index candidate and the first documentation line for a module or declaration candidate. |
| 5 | Every token occurs in catalog-owned documentation text, constructor documentation, or contract text. |

Package-documentation candidates use the same first matching tier, result
shape, excerpt, truncation-flag, no-match, and no-cursor behavior as language
topic results. For package-documentation results, one exact resource URI
appears at most once. Different retained snapshot or documentation digests
remain distinct. Equal-rank results sort by exact resource URI UTF-8 bytes
across the complete selected scope.

`read_doc` accepts one exact `uri` for the checked language index, checked
language topic resources, or retained package-documentation index, status,
module, and declaration resources. Success returns `uri`, `name`, `title`,
optional `description`, `mimeType`, and the same complete Markdown `text` as
`resources/read`. Hidden package-documentation module and declaration
resources are readable through exact URIs. Missing, nullable, non-string,
non-object, or unknown-field parameters fail with invalid params.
Syntactically valid but unknown, noncanonical, wrong-snapshot,
wrong-documentation-digest, source-resource, unpublished package
documentation, or unknown-topic URIs return an MCP tool result with
`isError: true`, structured code `resource_not_found`, and no partial document
text.

The documentation tool candidate set, result URIs, read metadata, and read
bytes are retained server state. Embedded standard-library candidates exist
after initialization. Successful saved-project dependency admission publishes
dependency package search candidates and reads atomically. Repeated admission
of the same retained package key adds no candidates. A later digest for the
same identity coexists with older candidates and reads until shutdown.
Status-only package documentation adds only its exact read route. Capacity
failures and stable-capture failures add no package candidates or read routes
and preserve earlier tool results and resources. Workspace refresh and later
filesystem changes do not remove or mutate retained package search candidates
or reads. Invalid input and `resource_not_found` failures do not change saved
workspace state, language-reference resource state, or package resource state.

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

`definition` and `references` read a saved workspace-relative regular
`.veln` source at positive JSON integer `line` and `column` coordinates.
Decimal or exponent JSON spellings that denote an integer address the same
position as the plain integer. A selected manifest project's captured owned
source uses project scope; another accepted source uses anonymous single-file
scope. A source below an unselected descendant manifest is not analyzed as
part of the outer project.

A definition result is either `{"definition": location|null}` or a failure.
A location has `uri` and a half-open `range` with one-based line and
Unicode-scalar column positions. Workspace locations use canonical `file:`
URIs. Eligible package locations use same-snapshot `veln-pkg:` URIs and may
include `packageDocumentationUri` for the retained declaration Markdown.
Invalid paths return `invalid_path`; invalid coordinates return
`invalid_position`; stable-capture exhaustion returns `snapshot_changed`;
capacity exhaustion returns `resource_capacity`. These failures include
`code`, `message`, and object `details`. Unsupported or valid-but-empty
selections succeed with `definition: null`.

The supported definition set includes workspace functions, types,
constructors, handler context and operation-clause parameters, exact
test-companion private-function access, and unique class-compatible invalid
source recovery records. Eligible package selections include public functions,
types, constructors, schemas, and public function aliases in exported direct
dependencies and embedded `std`; exported direct-dependency public type
aliases are eligible when their target resolves to a type in that retained
dependency. Invalid-cased package records, private or non-exported package
declarations, mismatched imports, unsupported symbols, and package module
segments return an empty definition. A public constructor selected through a
visible type alias returns the constructor declaration, not the alias
declaration. Definition exposes a recovery record's source range only;
`references` excludes recovery records, and MCP provides no rename tool.

An initial `references` request requires `source`, `line`, and `column`.
It may set boolean `include_declaration` (default `false`) and `page_size`
(integer default `100`, minimum `1`, maximum `1000`). Scope is derived from
source capture; it is not an input field. A continuation request contains only
a non-empty `cursor`.
The result is either `references`, a `scope` object, and optional
`next_cursor`, or the same failure object as definition. The scope is
`{mode:"project", generation, project, project_wide:true}` for selected
projects, or `{mode:"single_file", generation, project, source,
project_wide:false}` for anonymous source scope. The reference locations have
`uri` and half-open `range`; pages sort by URI UTF-8 bytes and numeric
start line, start column, end line, and end column. A nonfinal page contains
exactly `page_size` locations and `next_cursor`; the final page omits it.

Supported reference identities are schemas, eligible workspace,
direct-dependency, and standard-library schema aliases, functions, types,
constructors, value bindings, handler context parameters, and handler
operation-clause parameters. Schema
references include direct fields, `decode`, `encode`, `Repeat`, array
payloads, and resolved composition leaves. Workspace aliases have a separate
identity from their target and are eligible only when the direct target is a
public workspace schema; alias chains and package targets are ineligible.
Package schema aliases are eligible only in exported retained direct-dependency
or standard-library modules when every finite acyclic hop resolves through a
public alias and the terminal hop is an exported public schema in the same
retained package. The alias identity remains separate from its target and from
same-spelled aliases in other package origins. An eligible alias in the
standard-library `prelude` module is also selectable by its bare name in
composition, `decode`, and `encode` leaves, or by an explicit `prelude::`
qualifier. For a bare name, a same-named local schema, schema alias, type, or
type alias takes precedence and blocks implicit prelude fallback. A local
declaration does not block the explicit qualifier.

Bare schema names resolve in their declaring module. A full written import path
takes precedence over a colliding implicit leaf alias; an implicit leaf alias
resolves only when exactly one workspace or package import provides it.
Duplicate or syntax-recovered dependency imports resolve no dependency schema.
Consumer imports do not participate in package-alias target resolution.
Invalid-cased, unresolved, wrong-kind, transitive, non-exported, and
ambiguous aliases return successful empty reference results. These rules also
exclude module qualifiers, alias-target expressions, package implementation
sources, comments, strings, fields, and unrelated declarations.

When `include_declaration` is true, an eligible workspace declaration is
added as a `file:` location. In project scope, an eligible direct-dependency
or standard-library declaration is added as a `veln-pkg:` location; single-file
scope never adds a package declaration. The declaration is inserted before
sorting and paging. Package implementation locations remain excluded.

The cursor authenticates the process, generation, project, source selection,
scope, declaration policy, and page size. It is single-use. The server retains
at most 64 unfinished results in FIFO admission order. Refresh invalidates live
cursors as `stale_snapshot`; eviction does the same. Malformed, tampered,
foreign, post-restart, or consumed cursors return `invalid_cursor`, with
`details: {}`. Invalid request shapes and failed initial captures do not
consume a cursor. Continuation consumes its cursor before issuing a later one;
the final page releases its retained result. There is no time-based expiry.
A later reuse of an admission slot can classify an old authenticated cursor as
`invalid_cursor`; it never revives.

A successful refresh replaces roots, increments generation, and invalidates
cursors. A failed refresh preserves roots, generation, diagnostics, and
navigation. Stable capture retries are bounded; exhaustion returns
`snapshot_changed` without success-only fields. If dependency admission
would exceed the retained package capacity, definition and references return
`resource_capacity` without partial locations, scope, or new resources.

## References

Closed input and result schemas are in `crates/veln-mcp/schemas/mcp/v1/`.
Navigation serialization is implemented by `crates/veln-mcp/src/definition.rs`
and `crates/veln-mcp/src/references.rs`; cursor retention is implemented by
`crates/veln-mcp/src/reference_pagination.rs`. Protocol regression tests are in
`crates/veln-mcp/src/server/tests/`.
