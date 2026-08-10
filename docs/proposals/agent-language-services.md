---
role: proposal
update-when: The MCP acceptance model, language-service scope, virtual-location contract, reference inputs, plugin boundary, or implementation status changes.
---

# Agent Language Services

## Summary

Add a local MCP server to the Veln toolchain as `veln mcp`. The server gives
coding agents version-matched language knowledge and read-only code
intelligence without requiring them to drive the editor-oriented LSP protocol.

The first complete capability includes:

- project diagnostics;
- definition and reference lookup;
- language-reference search and retrieval;
- exported package and standard-library documentation;
- virtual source locations for dependencies and the standard library; and
- plugin packaging for Codex and Claude Code.

Language semantics belong to an editor- and agent-neutral language service.
`veln lsp` and `veln mcp` adapt that service to different session and transport
models. The MCP server does not start an LSP subprocess or proxy LSP messages.

## Motivation

Agents can run Veln commands through a shell, but they do not have a stable,
structured route to language rules, package APIs, definitions, or references.
The current LSP server supplies some of that information to editors, but its
wire contract assumes an editor that maintains open-document state and speaks
LSP.

The current pages under `../specification/` are development specifications.
They include evidence routes, implementation boundaries, maintenance
instructions, and internal organization that are unsuitable as the published
language reference. The public reference must instead be a deterministic
projection of executable grammar, checked examples, compiler-owned public
tables, and narrowly scoped supporting prose.

Dependencies and the standard library also need stable source locations.
Returning their materialization paths would expose cache layout, make results
client-specific, and fail for embedded standard-library sources. A
content-addressed package snapshot provides one location model for path, vendor,
mirror, git, and standard packages.

## Goals

- Make `veln mcp` a stdio MCP server distributed with the `veln` executable.
- Keep MCP operations read-only in the first capability.
- Give MCP and LSP the same semantic answers for the same saved project
  snapshot.
- Generalize definition and reference lookup beyond the narrow cases currently
  implemented by `veln lsp`.
- Preserve LSP support for unsaved editor buffers and pushed diagnostics.
- Represent dependency and standard-library sources without exposing physical
  storage paths.
- Bind source and generated package documentation to the same immutable package
  snapshot.
- Publish a human- and agent-readable language reference from checked primary
  artifacts instead of exporting development documentation.
- Keep ordinary one-package repositories free from required MCP configuration.
- Support repositories containing more than one Veln package.
- Package client configuration and usage guidance as an agent plugin.

## Non-Goals

- A remotely hosted MCP service or HTTP transport.
- Completion or hover.
- MCP-managed unsaved document overlays.
- MCP formatting, rename, or file mutation.
- Replacing `veln lsp` for editor integrations.
- Publishing active proposals, implementation records, or source-decision
  history as language reference material.
- Treating MCP roots as an access-control mechanism or a required project
  discovery input.
- Changing the meaning of the existing lockfile source-tree checksum.
- Requiring SWI-Prolog during an ordinary Cargo build or installed-toolchain
  startup.
- Defining a registry or package-version scheme.
- Having `veln mcp` edit Codex or Claude Code user configuration in the first
  capability.

## Terminology

- **Workspace base**: the current directory of the `veln mcp` process.
- **Workspace project**: a Veln package selected below the workspace base, or an
  anonymous source root when no manifest exists.
- **Saved snapshot**: files read from disk for one MCP operation.
- **Open-document overlay**: editor text retained by the LSP server and applied
  over saved files.
- **Package identity**: the dependency table key, manifest package name, or
  reserved `std` identity.
- **Package snapshot**: a package manifest and its sorted owned source files.
- **Language-reference snapshot**: the checked inputs used to build one public
  language-reference catalog.
- **Virtual source**: source addressed by a `veln-pkg:` URI instead of a
  physical `file:` URI.
- **Published reference**: the human- and agent-readable projection exposed by
  MCP and other documentation renderers.

## Ownership Boundary

Language operations are classified by their observable requirements.

| Question | Required owner or exposure |
| --- | --- |
| Does the operation compute Veln language meaning? | Implement it in the shared language service. |
| Does it require continuing open-document state? | Expose it through LSP. |
| Is it useful as an explicit, bounded agent request over saved files? | Expose it through MCP. |
| Does it provide language or package knowledge? | Expose it as an MCP resource and, when model discovery matters, a bounded tool. |
| Is its representation specific to an editor or agent client? | Convert it in the corresponding adapter. |
| Does it require direct file mutation? | Keep it outside the first MCP capability. |

The resulting boundary is:

| Capability | Language service | LSP | MCP |
| --- | --- | --- | --- |
| Saved project analysis | yes | yes | yes |
| Definition and references | yes | yes | yes |
| Open-document overlays | snapshot input only | yes | no |
| Pushed diagnostics | no | yes | no |
| Semantic tokens | editor records | yes | no |
| Language and package reference | reference catalog | no | yes |
| Formatting and rename edits | shared computation where practical | yes | deferred |
| Direct file writes | no | no | no |

The shared language service returns Veln-owned diagnostics, symbols, ranges,
locations, and reference sets. It does not return JSON-RPC, LSP, or MCP wire
types.

## MCP Server Contract

### Command And Transport

`veln mcp` starts one MCP server over standard input and standard output. It
takes no source path arguments in the first capability. Protocol logs and human
diagnostics must not be written to standard output because that stream carries
MCP framing.

The server declares resource and tool capabilities. It may use client root
information as a compatibility hint, but the availability or contents of
`roots/list` do not change the normative project-selection rules below.

### Project Selection

The server selects workspace projects from its workspace base with the same
package-boundary principles used by the LSP workspace-folder selection.

| Workspace state | Selected projects |
| --- | --- |
| The workspace base has a regular `veln.toml`. | Select the workspace base and do not search below it for implicit projects. |
| The workspace base has no manifest and a directory branch contains manifests. | Select the first manifest directory on each branch. |
| The workspace base has no manifest below it. | Select the workspace base as one anonymous project. |

Implicit discovery does not follow directory symbolic links and skips `.git`.
An ordinary `target` directory is discoverable, matching current project
discovery. Selected filesystem identities are sorted and deduplicated.

Tool paths are relative to the workspace base and use `/` separators. Absolute
paths and paths that resolve outside the workspace base are rejected.

At startup, the server resolves the workspace base once to an existing
canonical directory identity. The startup spelling may contain symbolic links.
Discovery does not traverse directory symbolic links. Every component below
the resolved base in a tool path is opened without following links; directory
links, file links, nonexistent leaves, and non-regular source files are
rejected. Returned `file:` URIs use the resolved base identity and normalized
relative path. An accepted startup alias therefore produces one URI spelling.
If an opened identity or path set changes during capture, the operation fails
with `snapshot_changed` instead of reading through the replacement.

Project choice follows this table:

| Request | Choice rule |
| --- | --- |
| `check_project` with an explicit manifest project | The project must match a selected workspace-project root, and `source` must be omitted. |
| `check_project` for an anonymous project | `project` is `.` and `source` must name exactly one accepted regular `.veln` file below the base. Only that file is analyzed. |
| `check_project` without a project and exactly one manifest project selected | Use that project. |
| `check_project` without a project and multiple projects selected | Return an ambiguity error listing relative project roots. |
| `definition` or `references` for a path in one selected project's captured owned-source set | Infer that project. |
| `definition` or `references` for any other accepted source path | Perform anonymous single-file analysis and do not return project-wide references. This includes a source owned by an unselected descendant manifest. |

An explicit outer and nested project may both be selected only when the client
starts separate MCP servers with the corresponding workspace bases. Implicit
selection stops at the first manifest on a branch.

Selection is fixed at server startup. The `refresh_workspace` tool is the only
operation that rediscovers projects. A successful refresh atomically replaces
the complete selection and increments its generation. A failed refresh keeps
the previous selection. Manifest addition, removal, and rename do not affect
selection before a successful refresh. A refresh makes all earlier reference
cursors stale, but it does not remove package snapshots already published by
the server.

### Coordinates

MCP inputs and results use one-based line and one-based Unicode scalar column
coordinates. LF and CRLF each end one logical line; CRLF is one terminator and
neither terminator byte is addressable. A line containing `N` Unicode scalars
accepts columns 1 through `N + 1`. The last column is its end insertion point.
A terminal line ending creates a final empty line whose only valid column is 1.
An empty file therefore has the single valid position `(1, 1)`.

A range is half-open. A position at a token's end does not select the token.
An end-of-line or end-of-file insertion point is valid but normally selects no
symbol. Adapters convert between this form and LSP positions. LSP advertises
UTF-8, UTF-16, and UTF-32 support, selects the client's first supported
`general.positionEncodings` entry, and uses the LSP default UTF-16 when the
client does not negotiate an encoding. A position inside a multi-unit encoding
of one scalar is invalid.

An input position outside the selected source returns an `invalid_position`
error. A position that is valid but does not select a supported symbol returns
a successful result with no definition or references.

### Tools

The authoritative MCP v1 input, result, resource-metadata, and domain-error
schemas are checked JSON Schemas in the planned `mcp/v1` schema bundle. The
server exposes that bundle as a built-in resource and derives its tool
declarations from the same files. Schema objects reject unknown fields and
reject `null` unless a field explicitly permits it. Schema or JSON-RPC shape
failures map to protocol invalid-params errors. A decoded domain failure is an
MCP tool error with `{code, message, details}`.

The stable v1 domain codes are `invalid_path`, `invalid_position`,
`invalid_query`, `source_required`, `project_not_selected`,
`project_ambiguous`, `snapshot_changed`, `invalid_cursor`, `stale_snapshot`,
`resource_not_found`, `generation_failed`, `resource_capacity`, and
`incompatible_version`. The request spelling of the
workspace base is `.`. Numeric values outside their documented range are
rejected and are never clamped.

The first capability exposes these model-controlled tools:

| Tool | Purpose |
| --- | --- |
| `check_project` | Analyze one saved workspace project and return structured diagnostics. |
| `definition` | Resolve the symbol at a saved source position and return its declaration location. |
| `references` | Return a deterministic, bounded page of locations for the symbol at a saved source position. |
| `search_docs` | Search the published language and package reference catalogs. |
| `read_doc` | Read one published documentation resource through a model-controlled route. |
| `workspace_projects` | Return the current selection generation and sorted project roots. |
| `refresh_workspace` | Atomically rediscover workspace projects and return the new selection generation. |

`check_project` accepts an optional workspace-relative project root and an
optional source path. The source path is required only for an anonymous
project and selects one file; the tool rejects all other project/source
combinations. Its
diagnostic facts use the same Veln diagnostic identifiers, severities, source
ranges, related notes, and structured details as the compiler-owned diagnostic
model. Transport failure is distinct from a successful analysis containing
language diagnostics.

`definition` accepts a workspace-relative source path and position. It returns
zero or one semantic location. When the declaration has published
documentation, the result also contains its documentation resource URI and
declaration identifier.

The first `references` request accepts a workspace-relative source path,
position, `include_declaration`, and a bounded page size. A continuation
request contains only `cursor`. `include_declaration` defaults to true and the
page size defaults to 100 with a maximum of 1,000. Values outside that range
are rejected rather than clamped. Results are sorted by URI, start line, start
column, end line, and end column. Each result states whether its scope is a
selected project or one file and whether it is project-wide.

`search_docs` accepts a query of at most 256 Unicode scalars, a scope of
`language`, `package`, `stdlib`, or `all`, and a result count that defaults to
10 and has a maximum of 50. Empty queries and out-of-range counts are rejected
rather than clamped. Each result contains a resource URI, title, summary, and
short matching excerpt. It does not return an entire reference page.

`read_doc` accepts one `veln-doc:` URI and returns the same content and metadata
as standard MCP resource reading. This duplicate route is intentional:
resources remain available to application-controlled clients while agents can
explicitly retrieve a selected result as a model-controlled tool.

### Resources

The server lists and reads:

- a language-reference index and individual language topics;
- package-documentation indexes, modules, and public declarations for loaded
  dependency snapshots;
- standard-library documentation from the embedded standard package snapshot;
- virtual source files for loaded dependencies and the standard library.

Large catalogs use resource templates and bounded indexes rather than listing
every declaration eagerly. Documentation resources use Markdown text. Virtual
source resources use Veln source text.

The first successful operation that publishes a dependency snapshot pins it
until server shutdown. Snapshots are deduplicated by package identity and
digest. Different digests for one identity can coexist. The server admits at
most 256 package snapshots, including the embedded standard package. An
operation that would exceed the limit fails before publishing any new URI and
does not evict an existing snapshot. The embedded standard package and
language reference are always available.

## Definition And Reference Coverage

The initial capability generalizes navigation beyond the current LSP cases.
The closed v1 navigation matrix contains:

- project-owned public and private functions;
- source types and their constructors;
- schemas;
- public member aliases;
- function and handler parameters;
- local `let` and pattern bindings;
- handler operation clause bindings;
- handler context parameters;
- exact test-companion access to target-private declarations;
- visible declarations in direct dependency exports; and
- visible standard-library declarations.

Definition and reference results follow existing visibility and shadowing
rules. A same-spelled field, local binding, declaration, comment, or string is
not a reference unless it resolves to the selected symbol. Private dependency
declarations do not become navigable merely because their source snapshot is
loaded. A dependency must be initialized as a separate workspace project to
provide editor operations over its private declarations.

MCP definition and references use saved files. LSP definition and references
apply the current open-document overlays before calling the same semantic
operations. With no overlays, both adapters return the same semantic locations
after coordinate conversion.

A project-scoped reference request searches the captured owned sources of the
inferred selected project. It does not search other selected projects or
dependency implementation bodies. For a visible dependency or standard symbol,
the result can contain uses in the selected project and, when requested, the
exported declaration location. A dependency initialized as its own workspace
project has its own independent project-wide search. Anonymous analysis searches
one file. Every result reports `scope` as `project` or `single_file`, its
relative scope root when present, and whether the result is project-wide. An
empty single-file result is therefore not presented as a complete project-wide
answer.

The server retains at most 64 reference continuation states. A cursor is an
opaque authenticated token bound to one server process, selection generation,
captured result, page size, declaration policy, and next offset. A continuation
contains only that cursor. Tampered, cross-server, post-restart, reused, and
terminal cursors return `invalid_cursor`. A cursor whose retained state was
evicted returns `stale_snapshot`. There is no time-based expiry. Unrelated file
changes do not affect a retained result, but a successful workspace refresh
stales every earlier cursor. Byte-identical restoration does not revive an
evicted cursor.

## Semantic Locations

The language service distinguishes workspace files from package snapshot
files.

| Source origin | Location URI |
| --- | --- |
| A source owned by the selected workspace project | `file:` |
| A dependency explicitly initialized as its own workspace project | `file:` |
| A source loaded only as a dependency | `veln-pkg:` |
| An embedded standard-library source | `veln-pkg:` |
| A compiler-known symbol with no source declaration | No definition location; return a related language-reference URI when one exists. |

### Package Source URI

The canonical virtual-source form is:

```text
veln-pkg:///<package-segment>/snapshot/<digest>/<source-path>
```

For example:

```text
veln-pkg:///github.com%2Foakcask%2Flib/snapshot/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/math.veln
veln-pkg:///std/snapshot/456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123/prelude.veln
```

`package-segment` is the UTF-8 package identity encoded as one URI path segment.
`/` in the identity is encoded and does not delimit the segment. `digest` is
the lowercase hexadecimal SHA-256 package-snapshot digest without a prefix.
`source-path` is the normalized package-relative source path. Its separators
are `/`; empty, `.`, and `..` segments are invalid. Each path segment is
percent-encoded separately. URI percent encoding uses uppercase hexadecimal
digits and UTF-8 bytes.

Source kind and materialization location are not part of the URI. Path, vendor,
mirror, and git dependencies with the same package identity and package
snapshot therefore use the same virtual location.

### Package Documentation URI

Generated package documentation uses the same package segment and snapshot
digest and adds the digest of its canonical documentation result:

```text
veln-doc:///package/<package-segment>/snapshot/<digest>/documentation/<doc-digest>/index
veln-doc:///package/<package-segment>/snapshot/<digest>/documentation/<doc-digest>/module/<module-id>
veln-doc:///package/<package-segment>/snapshot/<digest>/documentation/<doc-digest>/declaration/<declaration-id>
veln-doc:///package/<package-segment>/snapshot/<digest>/documentation/<doc-digest>/status
```

Module and declaration identifiers are deterministic within the package
snapshot. They are opaque to clients. A definition result returns the exact
documentation URI instead of asking clients to construct it.

### Resolution And Failure

The server resolves only virtual URIs whose package identity and digest match a
currently loaded snapshot or the embedded standard package. An unknown digest,
invalid path, mismatched package identity, or source path absent from the
snapshot returns `resource_not_found`. The server never falls back from a
virtual URI to a physical cache path.

The resolver accepts canonical URI spellings only and never normalizes before
lookup. The scheme is lowercase, the authority is empty, and query, fragment,
userinfo, host, and port are absent. ASCII unreserved bytes are literal. Every
other UTF-8 byte is percent-encoded with uppercase hexadecimal digits.
Percent-encoded unreserved bytes, decoded separators, empty or dot segments,
lowercase escape digits, malformed UTF-8, and a digest not consisting of
exactly 64 lowercase hexadecimal digits are rejected as `resource_not_found`
without filesystem access.

The virtual source remains readable when generated documentation fails. In
that case definition results omit the documentation link. Only the package
status resource remains from the documentation projection; it reports ordered
generation diagnostics.

### Saved Snapshot Capture

One operation uses one immutable capture containing project selection, the
complete owned path sets, manifest bytes, source bytes, and dependency
snapshots. Discovery, hashing, analysis, documentation generation, and the
operation's responses consume only that capture.

Each capture attempt enumerates and reads all inputs and then independently
enumerates and reads them again. The attempt succeeds only when both path sets
and all bytes match. The server retries the complete capture at most three
times. Continued change returns `snapshot_changed` and publishes no partial
state. Concurrent operations use independent captures. A successful operation
only interns immutable package and documentation snapshots by identity and
digest; it does not replace a different digest or cause a slower operation to
answer from newer bytes.

## Package Snapshots

The transport-independent Q11 digest transcript foundation is implemented in
`veln-project`. Its current contract and fixed-vector evidence are specified in
[Package Snapshot Digests](../specification/package-snapshots.md). The same
specification defines the implemented filesystem capture, Q12 distribution
set foundation, portable package identity and capture validation, and Q13
evidence. The canonical transport-independent virtual-source catalog and exact
resolver are also implemented and specified in
[Package Virtual Sources](../specification/package-virtual-sources.md).
The direct path, vendor, mirror, and locally available direct git dependency
LSP definition and virtual-document slices, embedded standard-library LSP
definition and virtual-document slices, plus the VSCode content provider, are
implemented and specified in
[Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename).
The transport-independent package documentation catalog, status-only failure
result, documentation digest, canonical `veln-doc:` URI foundation, same-capture
manifest binding, path-derived module identity, exported module and public
constructor documentation projection, stream-aware expected-output publication,
effect-row-binder signatures, and generated doctest static gate for expression,
declaration, and mixed declaration-statement doctests are implemented and
specified in
[Package Documentation Catalogs](../specification/package-documentation.md).
MCP resources, MCP documentation publication, and snapshot lifetime rules
remain planned below.

The owned distribution set contains every captured regular file whose name
ends in `.veln`, including private, non-exported, on-disk generated, and
ordinary `target` sources. It excludes exact `.test.veln` companions,
`*_test.veln` integration-test sources, `.git`, descendant directories with a
regular `veln.toml`, and all symbolic links. Digesting, dependency and standard
package analysis, virtual-source listing, and virtual-source reads share this
captured set. Workspace diagnostics use the broader workspace source set and
identify that scope in their result metadata.

Every virtualized path and package identity is valid UTF-8 and Unicode NFC.
Path segments are nonempty, relative, `/`-separated, and cannot be `.`, `..`,
control-bearing, NUL-bearing, contain `\` or `:`, end in a space or `.`, or use
a platform-reserved device spelling. A name that the host cannot represent is
rejected instead of lossily converted. A package identity is the
dependency-table key, or the manifest package name for a root package. It has
1 through 255 Unicode scalars, has no empty or whitespace-bearing segment, and
cannot be `std`; only the embedded package owns that identity. A package is
rejected when two paths collide after Unicode default case folding. The v1
schema pins the Unicode data version. Source text must also be valid UTF-8,
while accepted bytes and line endings are hashed without normalization.

The existing lockfile checksum remains available as separate metadata. A path
dependency changed after lockfile generation uses the digest of the files that
were actually analyzed, not the stale lockfile checksum.

The same package snapshot produces byte-identical source resources. The same
package snapshot and documentation-result digest produce byte-identical
documentation resources. Any manifest or included-source byte change produces
a different package snapshot digest.

## Generated Package Documentation

Package documentation is a deterministic public-API projection, not an
arbitrary invocation of `veln doc [path ...]`.

Its `doc-digest` is SHA-256 over the ASCII domain
`veln-package-doc-catalog/v1\0`, the canonical catalog byte length as u64-BE,
and the canonical documentation-result bytes. A result contains either the
complete successful catalog or the complete ordered failure diagnostics. It
also contains its schema version and generator-contract version. It does not
contain a toolchain marketing version.
A renderer-only implementation change preserves the URI only when it preserves
the exposed result bytes. A semantic, schema, or generator-contract change
changes the documentation-result digest.

Module identifiers are full lowercase SHA-256 digests of a versioned canonical
module identity. Declaration identifiers use a separate domain over declaration
kind, fully qualified semantic name, and canonical disambiguating signature.
They do not depend on traversal order or source ranges. A duplicate semantic
identity or detected identifier collision fails the complete generation.

For an external or standard package, it includes:

- package identity and public metadata;
- modules listed by `[lib].exports`;
- public types and public constructors in those modules;
- public schemas;
- public member aliases;
- public functions;
- attached documentation comments;
- public function contracts;
- visible doctest and expected-output fences; and
- resolved documentation references.

It excludes:

- non-exported modules;
- private declarations;
- exact test companions and integration-test modules;
- hidden doctest setup;
- ADR-lite records; and
- toolchain maintenance metadata.

All distribution sources in a loaded snapshot are listable through bounded
source indexes and readable, including private and non-exported modules.
Semantic visibility still controls whether consumer navigation can select a
declaration. Published package metadata is limited to package identity,
manifest package name, version, description, license, authors, keywords, and
exported module names. It excludes raw manifests, local paths, dependency
declarations and selectors, repository and homepage URLs, tool metadata,
environment-derived values, and unknown fields.

Documentation generation has a parse, manifest, export, documentation-reference,
and doctest gate. The manifest gate rejects unsupported manifest sections,
invalid export paths, test companion exports, duplicate exported module
identities, and invalid direct git selector cardinality. Generation is
package-atomic. On failure, the status resource is listable and contains
diagnostics sorted by source URI, start range, diagnostic code, and message.
Module and declaration resources are not listed, searched, or readable. The
loaded-package index returns the exact immutable status URI. The source
snapshot remains readable.

A passing positive doctest and a `veln fail` doctest whose visible source
produces a parse diagnostic can be published. Ignored doctests and hidden setup
are not published. Doctest metadata is validated by the shared doctest
extractor, and metadata errors fail the complete documentation generation.
Runtime doctest execution and expected-output comparison remain part of the
planned MCP publication slice.

## Published Language Reference

### Authority And Inputs

The MCP server does not publish files from `../specification/` directly. Those
files remain development specifications and evidence routes.

The published reference is generated in this priority order:

1. executable grammar;
2. checked examples under `../../examples/specification/`;
3. compiler-owned public tables; and
4. concise supporting prose for observable rules that the first three media do
   not explain by themselves.

Supporting prose is not the only evidence for grammar, diagnostics, command
output, or another rule that can be expressed mechanically. Published content
does not include proposal text, implementation records, maintenance routes,
repository paths, compiler algorithms, or unfinished behavior.

### Topic Catalog

Each published topic has a stable identifier, title, summary, keywords, body,
and related topic identifiers. A topic descriptor may select named grammar
productions and checked example files. It does not copy grammar or example
source into a second hand-maintained authority.

The closed v1 topic matrix contains:

- lexical structure and the complete executable grammar;
- modules, imports, packages, exports, and visibility;
- declarations and aliases;
- expressions, operators, and patterns;
- types, inference, and constructors;
- effects and handlers;
- contracts;
- schemas;
- holes; and
- tests, documentation comments, and doctests.

The published language resource form is:

```text
veln-doc:///language/snapshot/<digest>/index
veln-doc:///language/snapshot/<digest>/topic/<topic-id>
```

The digest identifies the generated public catalog. Clients receive exact
topic URIs from indexes and search results and do not construct them.

### Executable Grammar

The executable source-surface grammar remains the only maintained grammar
definition. Its generated output supplies the public syntax appendix and any
selected topic fragments. CI checks accepted and rejected grammar fixtures and
checks that the generated reference artifact matches the executable grammar.

An ordinary Cargo build consumes an already checked generated artifact. It
does not execute SWI-Prolog.

### Checked Examples

Reference topic descriptors select existing specification cases and the files
that are suitable for display. Test harness commands and unrelated assertions
are not published. When an existing case is too broad or noisy, a small
reference-oriented case is added under `../../examples/specification/` and is
run by the same harness.

Published normative code blocks come from checked case files. A displayed
expected result is exact checked output or a mechanically selected structured
fact; it is not an unchecked transcription. Topics use success, boundary, and
failure cases when each is material to the stated rule.

### Compiler-Owned Tables

Public catalogs such as keywords, operators, built-in types, known effects,
prelude signatures, and schema primitive families are generated from their
compiler-owned records when such records exist. The reference does not keep a
second manually synchronized list.

### Reference Snapshot

The authoritative language-reference artifact is canonical schema-v1 JSON.
Object keys are lexicographically sorted, insignificant whitespace is absent,
catalog-owned text is NFC with LF line endings, and source and expected-output
blocks preserve their selected scalar content after CRLF-to-LF conversion. The
artifact contains its schema version and generator-contract version. It does
not contain build paths, timestamps, or a compiler binary version.

The language-reference digest is SHA-256 over the ASCII domain
`veln-language-reference/v1\0`, the canonical artifact byte length as u64-BE,
and the artifact bytes. The artifact covers grammar output, topic descriptors,
supporting prose, selected example source and expected results, and generated
public table fragments. Changes confined to development documentation do not
change it.

MCP and offline Markdown are pure renderings of the parsed artifact. Their
conformance comparison checks topic identifiers, titles, summaries, keywords,
relations, ordered semantic blocks, source snippets, and expected-result text;
renderer navigation and decoration are outside the comparison. Topic IDs are
unique lowercase ASCII descriptor IDs. An invalid or duplicate topic ID fails
catalog generation. The same catalog feeds MCP, offline Markdown, and a future
web renderer. Agent instructions may differ by plugin, but agents and humans
read the same language reference content.

### Documentation Search And Reads

Search normalizes a query to NFC, applies Unicode default case folding using
the v1 pinned Unicode data version, trims Unicode whitespace, and splits on one
or more Unicode whitespace characters. A candidate must contain every token in
its identifier, title, keywords, summary, signature, or body. Ranking tiers are
exact identifier or title, identifier or title prefix, all tokens in title or
keywords, all tokens in summary or signature, and body match. Equal-tier
results sort by resource URI UTF-8 bytes. One URI appears at most once.

The `language` scope contains language topics. `stdlib` contains only embedded
standard-package documentation. `package` contains loaded non-standard package
documentation. `all` is their union. An excerpt uses at most 160 Unicode
scalars around the first match in the highest-ranked field and reports
independent prefix and suffix truncation flags.

Each generated documentation resource is indivisible and limited to 262,144
UTF-8 bytes. A catalog with an oversized resource fails generation. Successful
standard resource reads and `read_doc` return identical complete Markdown bytes,
media type, and metadata. Neither route paginates or truncates content.

## LSP Integration

`veln lsp` retains its editor session model:

- workspace folders and `rootUri` select projects;
- open-document notifications maintain overlays;
- diagnostics are published; and
- editor requests use LSP positions and response types.

Definition and reference handlers delegate semantic lookup to the shared
language service. Dependency and standard-library definitions may return
`veln-pkg:` locations.

LSP clients cannot be assumed to display an unknown URI scheme. Veln defines a
read-only `veln/virtualDocument` request that accepts a `veln-pkg:` URI and
returns its Veln source text. The VSCode integration registers a content
provider for the scheme and uses that request. A client without virtual
document support may still display package identity, source path, and range,
but opening the location is client-dependent.

## Agent Plugin

One plugin source may contain client-specific manifests and shared components:

```text
plugins/veln/
├── .codex-plugin/plugin.json
├── .claude-plugin/plugin.json
├── .mcp.json
├── .lsp.json
├── compatibility.toml
└── skills/
```

The checked Codex manifest points `mcpServers` to the root `.mcp.json`, matching
the current OpenAI plugin contract. The shared MCP configuration starts the
prerequisite executable as `veln mcp --client-contract 1`. Claude Code
additionally uses the LSP configuration to start
`veln lsp --client-contract 1`. Codex obtains the initial code intelligence
through MCP. The plugin does not bundle or download Veln.

Each client starts one server per active workspace root with that root as the
process working directory. A host mode is unsupported when it cannot set the
working directory or document and demonstrate inheritance from the workspace
launch directory. Initialization verifies that the client root resolves to the
server workspace base. It also exchanges the toolchain version, MCP contract,
language-service ABI, and reference schema version before making tools and
resources available.

An explicit client executable setting takes precedence. Otherwise the first
`veln` on `PATH` is used. A missing executable tells the user to install Veln
and put the active toolchain on `PATH`. A workspace mismatch, shadowed
incompatible executable, or incompatible contract reports the resolved
executable identity, observed and required versions, and the required action
on stderr or the client log. It never writes that failure to MCP stdout.

The shared skill instructs agents to:

- search the published reference instead of inferring Veln behavior;
- inspect package or standard-library documentation before inventing APIs;
- use definition and references for symbol identity;
- run project diagnostics after edits; and
- never treat proposal resources as current language behavior.

The first capability supplies validated plugin artifacts and documents the
client-native installation and enablement flows. It does not authorize the
`veln` executable to mutate client user configuration. A later proposal may
add an installer after the supported clients expose a sufficiently stable,
non-interactive installation contract.

`compatibility.toml` is the authoritative client matrix. The v1 matrix pins one
tested Codex host build and one tested Claude Code build per supported platform,
their manifest-schema revisions, validator versions and integrity digests, and
the required Veln, MCP, LSP, language-service, and reference-schema contracts.
Widening a host range requires adding and passing both boundary builds. Shared
skill content and `.mcp.json` are common authority. Each client manifest and
Claude Code's `.lsp.json` are authoritative only for that client. Client staging
packages omit files unknown to that client and are freshness-checked against the
shared inputs.

Every matrix cell uses client-native installation, opens a fixture workspace,
checks process working directory, performs MCP initialize, tool, resource, and
template listing, resource reads, every tool, one domain failure, and shutdown.
Claude Code additionally performs the LSP initialize, initialized,
virtual-document, shutdown, and exit sequence.

## Safety And Privacy

- MCP code-intelligence tools do not write project files.
- Tool paths cannot escape the workspace base.
- Virtual URIs do not expose dependency cache paths, source URLs, credentials,
  or absolute workspace paths.
- Package source and documentation are limited to snapshots already loaded by
  selected projects, plus the embedded standard package.
- Search excerpts and reference reads are bounded.
- Tool errors distinguish invalid input, ambiguous project selection, stale
  snapshots, missing resources, analysis diagnostics, and internal transport
  failure.
- MCP server output never includes development proposal text as current
  reference material.

## Conformance Contract

The versioned `agent-language-services-v1` conformance manifest is the sole
completion gate. A repository-maintenance package named
`veln-repo-agent-language-conformance` under `tools/` validates it. The
manifest contains one requirement ID and at least one planned evidence ID for
every normative paragraph, acceptance row, schema field, domain error,
resource template, lifecycle transition, and supported client-platform cell.
It rejects missing, duplicate, skipped, orphaned, or unimplemented mappings.

The v1 manifest closes the capability matrices that this proposal previously
introduced as extensible minimums. It enumerates exactly the symbol kinds under
Definition And Reference Coverage, the language topics under Topic Catalog,
the tools and resource kinds under MCP Server Contract, the package-document
declaration kinds, the LSP encodings, and the plugin compatibility cells.
Adding a capability requires a new conformance-suite version or an explicit
backward-compatible extension entry.

The resolved-decision evidence groups are:

| Decision | Required evidence |
| --- | --- |
| Q01 anonymous diagnostics | Required single source, two unrelated files, invalid combinations, and invalid source paths. |
| Q02 descendant ownership | Outer source ownership and unselected descendant single-file analysis without outer references. |
| Q03 rediscovery | Manifest add, remove, and rename before and after refresh; atomic refresh failure; cursor invalidation; resource survival. |
| Q04 filesystem identity | Symbolic base, internal and external directory links, file links, missing leaves, alias URI equality, and link replacement. |
| Q05 stable capture | File and manifest byte changes, path-set changes, bounded retry, no partial publication, and concurrent captures. |
| Q06 schemas and errors | Schema freshness, required and nullable fields, unknown fields, every domain code, protocol mapping, project listing, and all numeric boundaries. |
| Q07 coordinates | Empty, LF, CRLF, terminal newline, non-BMP scalar, end positions, token-end exclusion, all LSP encodings, and normalized cross-adapter pages. |
| Q08 reference universe | Project, other-project exclusion, dependency consumer and declaration behavior, dependency-as-project behavior, and visibly single-file anonymous results. |
| Q09 cursors | Cursor-only continuation, page concatenation, tamper, cross-server, restart, reuse, eviction, unrelated changes, byte restoration, and refresh. |
| Q10 resource lifetime | Cross-project deduplication, coexisting digests, refresh and removal survival, capacity rejection, and shutdown. |
| Q11 package digest | All three fixed vectors, reversed discovery order, tag, byte-order, domain, and one-byte changes. |
| Q12 distribution set | Filesystem capture covers every inclusion and exclusion, private and non-exported sources, generated and `target` sources, exact-byte digest integration, ordering, and relocation. Digest-analysis-resource set equality remains planned. |
| Q13 portable domains | Non-UTF-8 names and text, NFC rejection, case collisions, separators, controls, aliases, unrepresentable names, and reserved `std`. |
| Q14 URI spelling and resolver mapping | Canonical round trip and rejection of percent aliases, encoded separators and dots, authority, query, fragment, and malformed digest forms; MCP mapping of catalog misses to `resource_not_found`. |
| Q15 disclosure | Private source access, excluded source rejection, complete metadata allowlist, and credential, path, dependency, URL, tool, and unknown-field exclusion. |
| Q16 document identity | Stable regeneration, semantic and schema change, renderer-only stability, ID order stability, duplicates, collisions, and exact definition links. |
| Q17 language catalog | Canonical byte and digest vectors, Unicode and line-ending inputs, schema changes, development-doc independence, freshness, and renderer semantic equivalence. |
| Q18 generation failure | Every generation gate, ordered diagnostics, resource and search absence, source survival, and positive, negative, ignored, invalid, and mismatched doctests. |
| Q19 search and reads | Query normalization, all scopes and rank tiers, ties, deduplication, bounds, scalar excerpts, truncation flags, size boundary, and route byte equality. |
| Q20 executable binding | Workspace paths, multi-root startup, inherited or explicit working directory, missing and shadowed executable, every incompatible contract, and matching initialization. |
| Q21 plugin matrix | Pinned validators, generated-package freshness, every supported client and platform boundary, native install, MCP smoke, Claude LSP smoke, and unknown-file isolation. |
| Q22 gate totality | Injected missing requirement, duplicate evidence, missing matrix cell, stale artifact, undeclared capability, malformed request class, and plugin mismatch. |

Q11 is implemented by the `veln-project` fixed-vector and transcript-mutation
tests. The Q12 filesystem-capture foundation is implemented by the
`veln-project` distribution matrix and digest-integration tests. Q13 is
implemented by the package-identity, portable source-path, UTF-8 source-text,
case-fold collision, and validation-exclusion tests. The Q14
transport-independent URI spelling and resolver foundation is implemented by
the `veln-language-service` virtual-source tests. Q12 analysis and
virtual-resource equality, Q14 MCP resource mapping, and Q15 through Q22
remain planned evidence for the agent language service.

The gate also covers resource-template listing and reads, every malformed
request class, zero/default/maximum/above-maximum bounds, all documentation
failure classes, digest incompatibility, stdout framing purity, generated
artifact freshness, and cross-adapter and cross-renderer equivalence. The
proposal completes only when every declared cell passes and implemented
behavior has been promoted to specification and executable-example routes.

## Acceptance Model

All rows describe planned evidence. They do not imply that the behavior is
already implemented.

### Server And Project Selection

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Start `veln mcp` in a one-package project. | The package is the default project and tools require no project argument. | CLI MCP framing case. |
| Start above two package branches. | Both first manifest roots are listed; `check_project` without a project reports ambiguity. | Multi-package MCP case. |
| Start where no manifest exists and check one of two unrelated sources. | The base is anonymous; `source` is required, only that file affects diagnostics, and project-wide references are unavailable. | Q01 anonymous isolation cases. |
| Navigate below an unselected descendant manifest. | The outer project does not own the source; navigation reports single-file scope without outer-project references. | Q02 descendant-boundary cases. |
| Add, remove, or rename a manifest. | Selection is unchanged until `refresh_workspace`; a successful refresh replaces it atomically and stales older cursors. | Q03 refresh transition table. |
| Start through a symbolic base alias. | The alias is accepted once and returned `file:` URIs use the resolved identity spelling. | Q04 symbolic-base cases. |
| Supply a path containing a directory or file symbolic link. | The path is rejected without following the link. | Q04 no-follow cases. |
| Supply an absolute path or escaping relative path. | The tool rejects the input before reading the target. | Path-boundary MCP cases. |
| Change a manifest, source, or file set during capture. | The complete capture retries at most three times, then returns `snapshot_changed` without partial publication. | Q05 stable-capture race cases. |
| List projects or send malformed tool input. | Roots use `.` or relative `/` spelling; the schema distinguishes protocol errors from stable domain errors and rejects unknown fields and out-of-range values. | Q06 schema and error cases. |
| Client roots are unavailable. | Project selection is unchanged. | MCP client without roots capability. |

### Diagnostics And Navigation

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Analyze a saved project with errors. | `check_project` returns structured Veln diagnostics without transport failure. | MCP diagnostic fixture aligned with command diagnostics. |
| Resolve a workspace declaration. | `definition` returns a `file:` location with MCP coordinates. | Shared language-service and MCP cases. |
| Resolve project references with shadowing and same-spelled fields. | Only references with the selected symbol identity are returned in deterministic order. | Table-driven symbol cases. |
| Search references to a dependency symbol from one selected project. | Consumer uses and the optional exported declaration are returned; other projects and dependency-internal uses are excluded, and the scope is explicit. | Q08 reference-universe cases. |
| Continue a paged reference result. | The request contains only its single-use cursor and concatenated pages have no gaps or duplicates. | Q09 cursor state-machine cases. |
| Use a tampered, cross-server, restarted, evicted, or pre-refresh cursor. | The server returns the specified `invalid_cursor` or `stale_snapshot` domain error without reinterpreting inputs. | Q09 cursor rejection cases. |
| Resolve an exported dependency declaration. | `definition` returns a `veln-pkg:` location and documentation link. | Path-dependency MCP case. |
| Resolve a private dependency declaration from a consumer. | No definition is returned. | Dependency visibility case. |
| Resolve a standard-library declaration. | The result points to matching `veln-pkg:` source and `veln-doc:` documentation snapshots. | Embedded standard-package case. |
| Address empty, LF, CRLF, non-BMP, end-of-line, end-of-file, and token-end positions. | Validity and half-open selection follow the scalar-coordinate contract. | Q07 coordinate matrix. |
| Run LSP and MCP on the same saved project without overlays. | Filesystem identities and Unicode-scalar locations match after declaration-policy normalization and concatenation of all MCP pages. | Q07 cross-adapter encoding matrix. |
| Apply an LSP open-document overlay. | LSP reflects the overlay; MCP continues to reflect saved files. | Existing LSP overlay cases plus MCP comparison. |

### Virtual Locations And Package Documentation

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Read a returned virtual source URI through MCP. | Resource text equals the source bytes used for analysis. The transport-independent exact-byte resolver is already implemented and specified in [Package Virtual Sources](../specification/package-virtual-sources.md). LSP `veln/virtualDocument` reads for direct path, vendor, mirror, locally available git, and embedded standard-library URIs are implemented and specified in [Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename). | MCP resource round-trip case using a source URI returned by analysis. |
| Change an included source or manifest byte in a captured distribution. | Snapshot capture passes the changed exact bytes to the implemented digest API. LSP virtual URI changes are implemented for retained direct path, vendor, mirror, and locally available git project snapshots. MCP virtual-resource changes remain planned. | Implemented Q12 capture digest-integration tests and LSP dependency virtual-URI tests; planned MCP virtual-resource cases. |
| Discover private, generated, test, descendant, symlink, non-regular, and `target` sources. | The captured distribution set includes private, non-exported, generated, and ordinary `target` sources, applies every stated exclusion, and errors on represented non-regular distribution sources. Analysis and resource consumers remain planned. | Implemented Q12 distribution-set and filesystem-boundary tests; planned analysis-resource equality cases. |
| Load nonportable names or colliding paths. | Invalid UTF-8, non-NFC, control-bearing, separator-bearing, case-colliding, or reserved identities are rejected before publication. | Q13 portable-domain matrix. |
| Read a noncanonical, unknown, or mismatched snapshot URI through MCP. | The server returns `resource_not_found` without normalization, fallback, or filesystem access. The transport-independent resolver rejection table is already implemented and specified in [Package Virtual Sources](../specification/package-virtual-sources.md). | MCP adapter case mapping a catalog miss to `resource_not_found`. |
| Read a private distribution source or inspect package metadata. | The source is readable; metadata contains only the closed public allowlist and no dependency, URL, tool, path, or credential-bearing fields. | Q15 disclosure-policy cases. |
| Keep returned dependency URIs while projects refresh or disappear. | Every published snapshot remains readable until shutdown; capacity failure never evicts an older URI. | Q10 resource-lifetime cases. |
| Generate package docs. | The transport-independent catalog contains only exported modules and their public API; attached contracts, visible doctests, stream-aware expected-output fences, resolved schema documentation references, effect-row-binder signatures, and declaration-location URI lookup are preserved. MCP publication remains planned. | Implemented package-documentation unit tests and readable `doc` examples; planned MCP resource case. |
| Change catalog semantics without changing package bytes. | The package digest stays fixed and the documentation catalog digest and URIs change. | Implemented package-documentation document-identity tests; planned MCP resource case. |
| Package documentation generation or doctest validation fails. | The transport-independent result contains ordered status diagnostics and no partial module or declaration catalog. MCP status-resource publication remains planned. | Implemented atomic-generation-failure unit tests; planned MCP resource case. |

### Published Language Reference

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Generate the syntax reference. | Its grammar is byte-equivalent to executable grammar output. | Grammar freshness check. |
| Select a reference example. | The case and display files exist and run through the specification harness. | Reference catalog validator. |
| Change a selected example or public table fragment. | The catalog freshness check fails until regenerated. | Generator freshness test. |
| Change only development documentation. | The public reference digest and catalog remain unchanged. | Determinism fixture. |
| Reorder equivalent catalog input or vary catalog-owned Unicode and line endings. | Canonical artifact bytes and the domain-separated digest follow the specified normalization. | Q17 canonical-catalog vectors. |
| Search a known language concept. | Results follow the query normalization, closed scopes, rank tiers, URI tie-break, deduplication, scalar excerpt, and truncation contract. | Q19 search matrix. |
| Read a documentation resource through both routes. | The indivisible resource is within 262,144 UTF-8 bytes and both routes return identical complete bytes and metadata. | Q19 bounded-read route equality. |
| Inspect the published catalog. | It contains no proposal text, maintenance route, or repository path. | Bundle content policy check. |
| Generate MCP and offline Markdown views. | Both views reproduce the catalog's topic metadata, ordered semantic blocks, snippets, and expected-result text. | Cross-renderer semantic-model test. |

### Plugin

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Validate the Codex plugin. | Its manifest and MCP configuration are accepted, bind the active workspace, and start a contract-compatible `veln mcp`. | Q20/Q21 pinned Codex native smoke. |
| Validate the Claude Code plugin. | Its MCP and LSP configurations bind the active workspace and complete both protocol lifecycles with the pinned client. | Q20/Q21 pinned Claude native smoke. |
| Start with a missing, shadowed, or incompatible executable. | Startup fails before capability use and names the failed version fact and required action outside MCP stdout. | Q20 executable-binding matrix. |
| Use the shared skill. | Instructions route agents to reference, package docs, navigation, and diagnostics without claiming proposals as current behavior. | Plugin content review and scenario tests. |
| Run the proposal completion gate. | Every requirement and evidence mapping, closed capability matrix, generated artifact, and supported client-platform cell passes with no orphan. | Q22 conformance-manifest self-check and gate command. |

## Implementation Slices

The shared saved-snapshot definition and reference foundation is implemented
and specified in
[Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename).
The Q11 package-snapshot digest transcript foundation is implemented and
specified in [Package Snapshot Digests](../specification/package-snapshots.md).
The Q12 filesystem capture and distribution-set foundation is implemented and
specified on the same page.
The portable package-identity and capture-validation foundation and its Q13
matrix are also implemented and specified on the same page.
The canonical package virtual-source URI and resolver foundation is implemented
and specified in
[Package Virtual Sources](../specification/package-virtual-sources.md).
The transport-independent exported package documentation catalog foundation,
same-capture manifest binding, path-derived module identity, stream-aware
expected-output publication, effect-row-binder signatures, and generated
doctest static gate are implemented and specified in
[Package Documentation Catalogs](../specification/package-documentation.md).
Direct path, vendor, mirror, and locally available direct git dependency
definition locations, embedded standard-library definition locations, the LSP
virtual-document request, and the VSCode content provider are implemented and
specified in
[Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename).
The implemented LSP direct git path includes local path, local `file:` URL,
and already materialized remote URL source spellings, package-lock-aligned
unique selector and `subdir` validation, snapshot-URI independence from
physical materialization paths, and retained exact-byte reads.
This bounded implementation retains validated workspace, direct-dependency,
and embedded standard-package captures for the definition-to-read path. It
does not implement dependency reference search or MCP resources.
The remaining slices are:

1. Define and validate language-reference topic descriptors. Generate the
   executable grammar, selected example, and compiler-owned table projections.
2. Add `veln mcp`, resources, documentation tools, project diagnostics,
   definition, and references.
3. Add cross-adapter conformance cases, bounded search, pagination, and stale
   snapshot handling.
4. Package and validate Codex and Claude Code plugins and document their
   client-native installation flows.

## Deferred Work

- Completion and hover.
- MCP formatting and rename edit calculation.
- MCP document overlays or client-supplied in-memory source.
- Remote transport and authentication.
- Persistent cross-session documentation indexes.
- Registry package versions in virtual URIs.
- Automatic client configuration mutation by `veln`.
- Opening virtual locations in LSP clients that provide no custom-document
  integration.
- Publishing internal ADR-lite records from dependency packages.
