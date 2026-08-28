---
role: specification
authority: normative
update-when: The `veln lsp` semantic-token, publish-diagnostic, navigation, formatting, rename, virtual-document, VSCode integration, or executable LSP evidence contract changes.
---

# Editor Support

This page specifies implemented editor-facing classification. It covers the
compiler-owned records, LSP semantic-token transport, and VSCode integration
used by editor integrations.

## Read First

- Source lexical tokens come from `veln-syntax`.
- Editor-neutral semantic records come from `veln-editor`.
- LSP `textDocument/semanticTokens/full` integer data comes from `veln-lsp`.
- LSP `textDocument/publishDiagnostics` messages come from `veln-lsp`.
- Definition and reference identity comes from `veln-language-service`.
- LSP `textDocument/definition`, `textDocument/references`,
  `textDocument/prepareRename`, and `textDocument/rename` convert shared
  navigation results to LSP responses in `veln-lsp`.
- LSP `veln/virtualDocument` reads immutable direct path, vendor, mirror,
  locally available direct git, and embedded standard-library source from
  retained package snapshots in `veln-lsp`.
- LSP `textDocument/formatting` is implemented in `veln-lsp`.
- The stdio LSP server starts through `veln lsp`.
- TextMate fallback highlighting is contributed by
  `editors/vscode/syntaxes/veln.tmLanguage.json`.
- VSCode starts the language server when a `.veln` document opens and requests
  full-document semantic tokens. It enables workspace diagnostics for Veln
  project folders. It also registers definition and `veln-pkg` virtual-document
  providers.

## Semantic Token Records

`veln-editor` returns Veln-owned records with a source span, token type, and
modifier bitset. These records are independent of LSP integer encoding so tests
can assert classifications without starting an editor.

The collector is error tolerant because it works from the lossless lexical token
stream. Parse or semantic diagnostics do not prevent lexical fallback tokens or
safe semantic classifications from being returned.

## Token Classes

The implemented semantic token types are standard LSP token types:

| Veln source element | LSP token type | Modifiers |
| --- | --- | --- |
| module name segment | `namespace` | `declaration` |
| use alias segment | `namespace` | `declaration` |
| function declaration name | `function` | `declaration` |
| function call or known function reference | `function` | none |
| test declaration name | `function` | `declaration`, `test` |
| schema declaration name | `type` | `declaration` |
| parameter declaration | `parameter` | `declaration`, `readonly` |
| parameter reference | `parameter` | `readonly` |
| let binding declaration | `variable` | `declaration`, `readonly` |
| local binding reference | `variable` | `readonly` |
| result binding | `variable` | `declaration`, `readonly`, `result` |
| handler operation clause binding | `parameter` | `declaration`, `readonly` |
| handler operation clause binding reference | `parameter` | `readonly` |
| type name | `type` | none |
| effect label | `enumMember` | none |
| schema format name | `enumMember` | none |
| record or field-access field | `property` | none |
| unnamed or named hole | `variable` | `hole` |
| prelude function | `function` | `defaultLibrary` |

Lexical fallback also classifies `#` comments, strings, numbers, keywords, and
operators with the matching standard LSP token types. Decimal, lowercase `0b`
binary, and lowercase `0x` hexadecimal integer literals are each one `number`
token. The contextual `satisfy` marker and boolean literals are highlighted as
keywords.

The only Veln-specific semantic token modifiers are `test`, `result`, and
`hole`.

## LSP Encoding

`veln-lsp` exposes the semantic-token legend, full-token response data, and a
stdio JSON-RPC server. The server advertises `textDocumentSync`,
`definitionProvider`, `referencesProvider`, `documentFormattingProvider`,
`renameProvider.prepareProvider`, and `semanticTokensProvider` with
full-document semantic token support. It handles `initialize`, `initialized`,
`shutdown`, `exit`, `textDocument/didOpen`, `textDocument/didChange`,
`textDocument/didClose`, `textDocument/semanticTokens/full`,
`textDocument/definition`, `textDocument/references`,
`textDocument/formatting`, `textDocument/prepareRename`, and
`textDocument/rename`.

The full response uses LSP relative integer encoding in groups of five:

1. delta line
2. delta start character
3. token length
4. token type index
5. token modifier bitset

Tokens are sorted before encoding. Overlapping ranges are skipped so the encoded
stream remains valid for LSP clients.

## LSP Diagnostics

The stdio server resolves each folder in `initialize.workspaceFolders` to its
filesystem identity before it selects package roots. A folder with a regular
`veln.toml` becomes one workspace project, and selection does not continue
below it. Otherwise, the first manifest package on each directory branch
becomes a workspace project. If no branch contains a manifest, the resolved
folder becomes one anonymous workspace project. The selected filesystem
identities are sorted and deduplicated.
When a workspace folder is supplied through a directory symbolic link, the
deduplicated project uses the resolved filesystem identity for project
selection while document requests and published locations remain valid for the
client-supplied path.

Nested manifest discovery does not follow directory symbolic links. It skips
`.git` and treats `target` as an ordinary directory. Explicit outer and nested
workspace folders remain separate workspace projects. Analysis can load a
source dependency without adding that dependency as a workspace project; the
client must supply the dependency as a workspace folder to initialize it as a
workspace project. Package-root selection finishes before source discovery or
analysis starts.

When no workspace folders are present, the server applies the same selection
rules to `initialize.rootUri`. When the client sends no workspace identity, the
server leaves workspace roots empty and publishes document-scoped diagnostics
for open documents only. The executable LSP example is
`../../examples/specification/lsp/workspace-package-root-selection/`. Direct
`veln-lsp` tests cover manifest roots, branch selection, explicit nested roots,
dependency isolation, filesystem-identity deduplication, directory symbolic
links, symlink workspace document requests, `.git`, `target`, and anonymous
fallback.

For files inside a resolved workspace root, the server discovers project
`.veln` files the same way `check` and `run` do. For each resolved root, a
descendant regular `veln.toml` excludes that nested package's saved sources,
while an ordinary `target` directory remains discoverable. The server excludes
doctest-generated sources, overlays open unsaved editor text onto the
discovered source set, and includes open new `.veln` buffers that do not exist
on disk yet. It publishes
`textDocument/publishDiagnostics` for every discovered or open workspace source
file, including unopened files. It also publishes empty diagnostic lists for
previously reported files that become clean or leave discovery.

For documents outside resolved workspace roots, diagnostics remain
document-scoped and are computed from the in-editor document text. Parse
diagnostics are reported first. When parsing succeeds, the server lowers the
document into the surface module model and publishes semantic diagnostics from
the checked surface module. Parse-clean source invalid-name records are passed
to that checked surface model, so document-scoped diagnostics include the
implemented source identifier casing failures specified by
[names-effects.md](names-effects.md).
For workspace sources, saved snapshots and open-document overlays publish
source identifier casing diagnostics for the selected workspace project only,
including source-path-derived module segment diagnostics at the zero-width
source-start range specified by [name-resolution.md](name-resolution.md).
An invalid declaration or handler binding name in the selected snapshot or
overlay does not enter the LSP navigation symbol set. Definition, references,
prepare-rename, and rename requests for that invalid name return the same
empty result shape as an unsupported symbol. Invalid casing in an unselected
package root does not produce a workspace diagnostic for the selected project.

Published diagnostics use standard LSP severity numbers and zero-based ranges.
The diagnostic `code` is the Veln diagnostic id, and the diagnostic `source` is
`veln`. Span-less diagnostics are published at a zero-width start range while
preserving the compiler-owned diagnostic id, including
`toolchain.invalid_symbol_case`. Source-path-derived module segment
diagnostics include LSP `data` with the observable source-path origin
projection: `origin`, `occurrence`, `source_path`, `source_kind`, `segment`,
and `segment_index`. The remaining diagnostic detail contract is the shared
compiler diagnostic contract routed by [diagnostics-json.md](diagnostics-json.md).

## LSP Navigation, Formatting, And Rename

`veln-language-service` accepts an effective project snapshot and a one-based
Unicode-scalar source position. It returns the selected symbol, its definition,
and deterministic reference locations as Veln source identities and ranges.
The result contains no URI serialization, JSON, JSON-RPC, or LSP coordinate
representation. Its direct tests cover project functions, exact companion
visibility, handler bindings, deterministic ordering, shadowing, field
isolation, and positions without a supported symbol.

`veln-lsp` captures the workspace manifest, saved workspace sources, valid
direct path, vendor, mirror, and locally available direct git dependency
snapshots, and the embedded standard-package snapshot together for each
selected workspace project. A git `subdir` selects the package root below the
available repository tree. The LSP server does not clone, fetch, or check out a
git dependency. It constructs the standard snapshot directly from the
embedded manifest and distribution sources without materializing a filesystem
tree. Navigation starts from that retained project snapshot. The server
applies open-document overlays to workspace sources before calling the shared
language service. It converts shared locations to LSP URIs and zero-based
ranges.
Definition, references, prepare-rename, and rename use the same shared selected
symbol and reference set.
For selected workspace type, constructor, function, and value-binding symbols,
`textDocument/rename` first validates that the requested replacement stays in
the selected symbol's existing identifier class. Type rename selection covers
type declarations and syntax-retained type-role references. It does not select
same-spelled effect names or effect operation names as type symbols. A bare
type-role reference with multiple visible same-spelled imported type candidates
has no selected symbol. A qualified type-role reference selects only the
visible type identity named by its qualifier. Constructor rename edits selected
constructor declarations, constructor calls, and source-declared bare nullary
constructor expression and pattern uses in workspace sources. Type and
constructor replacement names start with an ASCII uppercase letter. Function
and value-binding replacement names start with an ASCII lowercase letter. A
class-changing replacement returns JSON-RPC invalid params with code `-32602`.
The error payload preserves the shared `rename.invalid_case` code and includes
the selected symbol class, requested name, and required initial class. The
request returns no workspace edits in that failure response. A rename request
without a selected supported workspace symbol returns an empty workspace-edit
`changes` object, and prepare-rename for the same position returns `null`.
The executable
`identifier-casing-rename-boundary` LSP example covers same-class edits and
class-changing failures for the four supported rename classes, plus
source-declared nullary constructor uses, same-spelled non-type namespace
exclusion, ambiguous imported type rejection, and qualified type identity
preservation for type rename.
The executable `identifier-casing-snapshot-boundary` and
`identifier-casing-overlay-boundary` LSP examples cover selected-unit casing
diagnostics, invalid declaration exclusion from navigation results, overlay
replacement of saved source text, and unselected nested package isolation.
The executable `identifier-casing-source-path-boundary` LSP example covers
workspace source-path-derived module segment diagnostics at the zero-width
source-start range.
The executable `identifier-casing-handler-binding-navigation` LSP example
covers invalid handler context and operation-clause binding exclusion across
definition, references, prepare-rename, and rename for declaration positions
and in-scope uses.
For a workspace symbol, references and rename edits include only workspace
source locations. Sources loaded only as dependency package snapshots do not
produce `file:` locations for workspace references or workspace edits, even
when their module path and symbol spelling match a workspace declaration.

The retained dependency input contains the package identity, captured package
snapshot, manifest export paths, and canonical virtual-source catalog derived
from the same identity and snapshot. A qualified call through
`use module from "package"` can resolve to a function in that dependency only
when the dependency identity matches, the function's source is listed in
`[lib].exports`, and the function is public.
If a retained dependency declaration has an invalid source identifier casing
record, it is not eligible for dependency definition results.
The dependency source field can be `path`, `vendor`, or `mirror` when it names
an already available package root. A `git` field can name an already available
repository tree through the same local path and local `file:` URL spellings
accepted by package locking. Remote git URLs are retained only when their
selected repository tree is already materialized by another operation; the LSP
server does not materialize them. A git dependency is retained only when it
declares exactly one selector: `rev`, `tag`, or `branch`. When `subdir` is
present, it must be a non-empty repository-relative path with no root or
parent-directory component, and it selects the package root below the available
repository tree. The source kind and physical root are not part of the
retained package location. Equal package identity, dependency manifest bytes,
and distribution source bytes produce the same dependency `veln-pkg:` URI
across those source fields and physical roots. A manifest or included-source
byte change produces a different snapshot URI.

The retained standard input has the reserved `std` identity and the same
snapshot, export, and catalog boundaries. Bare and `prelude::` calls resolve
public functions from the exported standard prelude. A function parameter or
local binding with the same name shadows the bare prelude fallback at call
sites in its scope; the same standard function remains reachable through an
explicit `prelude::` call. A qualified call through `use module from "std"`
resolves a public function only from an exported standard source. Private
declarations and declarations in non-exported standard sources do not produce
definition results.

`textDocument/definition` returns a dependency or standard declaration with
the exact canonical `veln-pkg:` URI from the retained catalog. It does not
convert the location to a `file:` URI. It exposes neither a dependency
materialization path nor a standard-library build path. Workspace definitions
continue to use `file:` URIs. Private functions and functions in non-exported
package sources have no package definition result. Package declarations are
immutable locations:
`textDocument/prepareRename` returns no range for them, and
`textDocument/rename` returns no workspace edits for them. `textDocument/references`
returns no package locations for dependency declarations in this slice.

`veln/virtualDocument` accepts an exact `veln-pkg:` URI retained by the server
and returns its UTF-8 source text. The returned text preserves the captured
source bytes, including line endings. An unknown or noncanonical URI produces
a JSON-RPC invalid-params error. The request does not normalize the URI or read
a filesystem fallback. A later physical dependency edit does not change the
text returned for an already retained URI.

The VSCode extension registers a definition provider for Veln filesystem
documents and a `TextDocumentContentProvider` for `veln-pkg`. Following a
dependency definition therefore requests the exact returned URI through
`veln/virtualDocument` and opens the result as provider-backed content. If
VSCode's URI object displays a different string for the same provider-backed
document, the request still uses the canonical URI returned by the server.

The `veln-language-service` tests are the executable evidence for dependency
and standard visibility and transport-neutral package locations. The static
LSP example
`../../examples/specification/lsp/direct-dependency-virtual-document-boundary/`
covers dependency definition boundaries without reading the dynamic digest.
The `veln-lsp` dependency virtual-document test is the executable JSON-RPC
evidence for the complete definition-to-read path, retained CRLF text, retained
workspace and dependency sources, URI identity and digest, private declaration
rejection, prepare-rename and rename rejection, exact import-path visibility,
and unknown or noncanonical URI rejection. The VSCode extension tests cover the
corresponding definition request, exact-text read, location conversion,
canonical URI lookup after VSCode URI parsing, and content-provider
registration.
The `veln-lsp` path, vendor, and mirror dependency virtual-URI test is the
executable evidence that the returned URI omits physical placement and source
kind while still reading the exact retained source text. The executable LSP
example
`../../examples/specification/lsp/direct-git-dependency-virtual-document/`
checks a remote git source backed by an existing package-lock materialization,
including direct git `subdir` definition-to-read round trip against a fixed
snapshot URI and exact source text. The focused `veln-lsp` git dependency test
checks physical-location independence, local `file:` URL source spelling,
remote materialization, manifest-byte and source-byte URI changes, retained
exact bytes after a physical edit, and private declaration rejection. The
`veln-project` direct analysis source tests cover unique git selector
rejection and escaping git `subdir` rejection.

The executable LSP example
`../../examples/specification/lsp/standard-library-virtual-document/` checks
bare and qualified prelude definitions, an explicitly imported exported
standard module, bare prelude shadowing by parameter and local bindings, a
private prelude boundary, the exact standard snapshot URI, the complete
embedded prelude read, and noncanonical URI rejection. The `veln-lsp`
standard-package test additionally compares the returned virtual document with
the exact embedded source value and checks package rename rejection.

LSP executable examples use `stdin_jsonrpc_file` when the requested behavior is
an ordered sequence of decoded JSON-RPC requests and notifications. Those
fixtures can place document text in case-text sidecars and reference it with
`$case_text`, or reference copied workspace source URIs with
`$workspace_file_uri`. These directives keep source text and workspace URI
evidence in fixture files without making manual `Content-Length` framing or
temporary workspace paths part of the behavior under test. The
`publish-diagnostics`, `semantic-tokens`, and
`semantic-tokens-unsaved-change` examples use decoded `[[lsp_assert]]`
selectors for initialize capabilities, diagnostic notifications,
semantic-token data, and shutdown responses. When an assertion compares a
complete JSON-RPC response object, member order belongs to the harness JSON
equality model rather than to the LSP server contract.
When decoded assertions use file-backed JSON operands, the sidecar placement
is harness reviewability evidence and does not change the LSP message field
contract.
When a decoded assertion checks string containment, the containment operation
is harness evidence over the selected JSON string and does not change the LSP
message field contract.
When a decoded assertion checks array length or a workspace file URI, the
operation is harness evidence over the selected notification or response field.
It does not add an LSP extension field or change URI serialization behavior.

LSP executable examples still use ordered stdout fragments when JSON-RPC
responses are interleaved with file-backed virtual-document text. Those
fixture-manifest fragment boundaries are evidence placement, not a separate
LSP response contract. Examples use `stdin_file` with `.raw` case-text
sidecars only when the observable behavior depends on exact protocol bytes,
such as CRLF header separators or invalid JSON-RPC framing.

`textDocument/formatting` returns a single whole-document text edit containing
the same canonical formatting produced by the formatter. Handler operation
clauses are formatted as `operation(binding, ...) => expression`, with
operation-clause bodies formatted as ordinary expressions.

For a private target function reference written as `target::name` from the
exact `.test.veln` companion, `textDocument/definition` returns the private
function declaration location in the target `.veln` source when the companion
writes an explicit `use` for that target. `textDocument/prepareRename` returns
the selected function-name range for the same accepted identity.

`textDocument/references` for a private target function identity returns the
same declaration and reference set that rename edits. Handler operation clause
calls to the target function are references when the call resolves inside the
target source.

`textDocument/rename` for that identity returns workspace edits for the target
function declaration, valid call or function-value references in the target
source, same-module public function-alias targets in the target source, and
valid qualified call references in the exact matching companion. A same-named
companion-local declaration, bare companion reference, target callable
parameter, target local `let` binding, or target pattern binding is a different
symbol and is not edited inside the binding's scope. Record field labels and
field accesses that use the same text are not function references. In
`let name = name`, the initializer reference remains part of the production
function identity when it resolves before the local binding starts. Valid target
references after nested blocks, including `else if` branches, remain part of the
production function identity. Calls through another qualifier, companion
function-value references, companion public-alias targets, comments, and string
literals are not edited.
Definition, prepare-rename, and rename requests whose selected text is inside a
comment or string literal do not identify the private target function. Wrong
companions, `_test.veln` integration modules, and references through a target
dependency do not receive private-function definition or rename results.

Definition and rename use the same open-document overlays as workspace
diagnostics. Unsaved target or companion text can provide the declaration and
reference locations used in the response. The routed executable evidence is
`../../examples/specification/lsp/companion-private-function-identity/`. The
`veln-lsp` server tests also cover companion private-function definition,
prepare rename, rename edits, source-scope isolation, target function-value
references, target function-alias targets, companion function-value and alias
rejection, callable shadowing, record field isolation, match-arm binding
isolation, local-binding initializer references, rejected boundaries,
request-origin filtering, and open-document overlays.

For a handler operation clause binding, `textDocument/definition` returns the
binding location from the operation clause parameter list.
`textDocument/references` returns the binding and ordinary expression
references inside the clause body. `textDocument/prepareRename` returns the
binding range, and `textDocument/rename` edits the binding and references in the
clause body. Record field labels and field accesses that use the same text are
not binding references. The routed executable evidence is
`../../examples/specification/lsp/handler-operation-editor/`. The `veln-lsp`
server tests also cover handler operation clause function-call references and
record field isolation for binding rename.

For a handler context parameter, `textDocument/definition` returns the binding
location from the handler parameter list when an ordinary clause-body
expression selects that parameter. `textDocument/references` returns the
binding and ordinary expression references inside operation clause bodies.
`textDocument/prepareRename` returns the binding range, and
`textDocument/rename` edits the handler parameter binding and matching
clause-body references. A same-named top-level function is not selected by a
clause-body reference that resolves to the handler context parameter. A
same-named operation clause heading is not a handler context parameter
reference and receives no context-parameter definition, references, or rename
edits. A same-named operation clause parameter shadows the handler context
parameter inside that operation clause and is renamed as a separate local
binding. The routed executable evidence is
`../../examples/specification/lsp/handler-context-callable-binding/` and
`../../examples/specification/lsp/handler-context-operation-heading-isolation/`.
The `veln-lsp` server tests also cover callable handler context parameter
definition, references, rename, top-level function isolation, and operation
clause parameter shadowing.

## VSCode Integration

The VSCode extension contributes the `veln` language, the TextMate grammar,
semantic token types and modifiers, and activation for Veln files. On
activation, it starts the command configured by `veln.server.path` with the
`lsp` argument. The default command is `veln`.

The extension registers a document semantic token provider. Before requesting
tokens, it sends the current document text to the server, so highlighting follows
unsaved editor content.

The extension also registers a `veln` diagnostic collection. It starts
workspace diagnostics for VSCode workspace folders that contain `veln.toml`, or
for nested manifest directories when the VSCode workspace folder is a larger
repository. Manifest roots stop nested discovery so vendored dependencies are
not initialized as separate workspace roots. If a workspace folder has no
package manifest and no nested manifest roots, the extension keeps that folder
as an anonymous package root. Nested manifest discovery ignores `.git` and
treats `target` as an ordinary directory.
Open Veln documents outside resolved roots still receive document-scoped
diagnostics. The extension listens for `textDocument/publishDiagnostics`
messages from the language server and mirrors them into VSCode diagnostics so
syntax and checker diagnostics appear in the Problems pane.

The `veln.server.trace` setting controls protocol tracing in the Veln output
channel. `messages` logs compact request, notification, and response summaries.
`verbose` logs JSON messages with large document text redacted.

## Boundaries

Implemented:

- TextMate fallback grammar for comments, strings, numbers, keywords,
  operators, punctuation, unnamed and named holes, type-like identifiers, and
  identifiers.
- Editor-neutral semantic token records.
- Full semantic-token legend and integer data generation for LSP clients.
- Stdio JSON-RPC lifecycle for semantic highlighting and whole-document
  formatting requests.
- Stdio definition, prepare-rename, and rename responses for exact companion
  qualified private-function references.
- Stdio references responses for exact companion qualified private-function
  references.
- Stdio definition, references, prepare-rename, and rename responses for
  handler operation clause bindings.
- Stdio definition, references, prepare-rename, and rename responses for
  handler context parameters selected from operation clause bodies.
- Stdio diagnostic publication for discovered workspace Veln files across
  resolved workspace roots, including unopened files, with unsaved open
  document overlays.
- Document-scoped diagnostic publication for Veln documents outside resolved
  workspaces or when no workspace identity is initialized, including
  parse-clean source identifier casing failures.
- Stdio definition responses for public functions in exported direct `path`,
  `vendor`, `mirror`, and locally available direct git dependency sources, and
  `veln/virtualDocument` reads for the returned exact `veln-pkg:` URI.
- Stdio definition responses for implicit prelude functions and public
  functions in explicitly imported exported `std` sources, with exact
  `veln/virtualDocument` reads from the embedded standard snapshot.
- VSCode startup for `.veln` files using the configured language-server
  command.
- VSCode Problems pane integration for Veln diagnostics.
- VSCode `veln-pkg` virtual-document content provider backed by
  `veln/virtualDocument`.
- Rust tests for collector classification, LSP relative encoding, ordering, and
  overlap handling, and server initialize/full-token/diagnostic/navigation
  responses.

Not implemented:

- LSP range and delta semantic token requests.
- Completion and hover.
- Dependency reference search.
- General rename and go-to-definition support outside the implemented
  companion private-function identity, handler binding, direct path, vendor,
  mirror, locally available direct git dependency, and embedded
  standard-function definition cases.
