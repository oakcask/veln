---
role: specification
authority: normative
specification-coverage: usage=#lsp-encoding; behavior=#semantic-token-records; limits=#boundaries
update-when: The `veln lsp` semantic-token, publish-diagnostic, navigation, formatting, rename, virtual-document, VSCode integration, executable LSP evidence, or shared LSP/MCP navigation declaration-policy contract changes.
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
for open documents only.

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
When a workspace source has an invalid source-path-derived module identity,
the language service does not expose that source or its declarations as normal
workspace navigation candidates. Definition, references, prepare-rename, and
rename return no selected symbol or edits for positions in that source and for
qualified selections that would resolve through that invalid module identity.
References and rename edits for valid symbols selected from other sources do
not include occurrences inside that invalid source.
Navigation for unrelated valid workspace sources remains available.
Invalid source declarations, function parameters, result bindings, local and
pattern bindings, satisfy candidate bindings, handler context parameters, and
handler operation-clause parameters do not enter the normal LSP navigation
symbol set. Definition, references, and prepare-rename expose only the recovery
navigation records specified below. Rename requests for a recovery record use
the selected recovery record's retained declaration and linked in-scope
references. Invalid casing in an unselected package root does not produce a
workspace diagnostic for the selected project.

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
representation.

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
Definition and references use the shared selected symbol and reference set.
Prepare-rename and rename use the same selected-symbol model only for
rename-supported symbol classes.
Navigation requests convert zero-based UTF-16 LSP characters to the shared
one-based Unicode-scalar positions. Navigation responses convert shared ranges
back to zero-based UTF-16 LSP ranges using the retained source snapshot.
For a selected valid-cased, unrecovered workspace effect declaration,
references include every structurally complete bare effect-row occurrence,
handler `handles` target, and `perform
Effect::operation(...)` qualifier in saved workspace sources that declare the
same module. The declaration and each supported occurrence select the same
effect identity. The operation leaf remains a separate effect-operation
identity and has no effect reference set.
An unrelated parse error in the same saved source does not remove structurally
complete effect occurrences from that shared set.

Effect reference lookup requires one valid-cased workspace effect declaration
for that name and module. It excludes qualified imported or package effects,
generic effect-row parameters, duplicate declarations, invalid-cased or
unresolved names, syntax-recovered occurrences, and equal spelling in another
module or symbol class. Comments and strings do not contribute references.
Effects, handlers, and effect operations remain unsupported for rename.

For a selected workspace schema declaration, `textDocument/references` returns
the declaration when requested plus `decode`, `encode`, and directly resolved
schema-composition path leaves that resolve to that schema in workspace
sources. Composition references include direct fields and both supported
repeated-payload spellings. An eligible public schema alias has its own
declaration identity and returns the `decode`, `encode`, direct-composition,
`Repeat`, and array-payload leaves that resolve to that alias, without merging
them into its direct public workspace schema target. Workspace alias chains are
not eligible. An eligible public package alias in an exported retained
direct-dependency or standard-library module can resolve through a finite,
acyclic chain of public aliases to an exported public schema in the same
package. An eligible alias in the standard-library `prelude` module is also
selectable by its bare name or an explicit `prelude::` qualifier in
composition, `decode`, and `encode` leaves when `prelude` does not resolve as a
written import. An exact workspace or package import named `prelude` instead
selects that import's schema-alias identity across direct, `Repeat`, array,
`decode`, and `encode` leaves. A collision between exact workspace and package
imports selects neither identity and does not fall back to the standard
library, regardless of import order. For a bare name, a same-named local
schema or schema alias blocks implicit prelude fallback in every leaf. Because
composition also admits the type namespace, a same-named local type or type
alias additionally blocks the fallback there, but does not block it in
`decode` or `encode`. A syntax-recovered local schema alias with no target also
blocks bare fallback in every leaf. A local declaration does not block the
explicit qualifier.
Although a written import named `prelude` reports `name.reserved`, a
parse-clean reserved import still participates in this navigation precedence.
Syntax-recovered imports remain excluded.
When a selected standard-library alias is ineligible, references return the
existing successful empty result. That declaration remains distinct from a
same-spelled standard-library schema and blocks fallback to the schema instead
of becoming or merging with its identity.
Package aliases do not enter the workspace alias identity. Alias
declarations are not included when declaration inclusion is false. Definition
and rename support do not expand to schema aliases.

For both adapters, an eligible workspace declaration is included when the
request enables declaration inclusion and is excluded when it does not. LSP
uses `includeDeclaration` and always excludes package declarations. MCP uses
`include_declaration` over the same shared navigation result. In project-wide
scope, it additionally includes the canonical `veln-pkg:` declaration for an
eligible direct-dependency or standard-library selection. In single-file
scope, it can include only a workspace declaration in the captured source.
This adapter difference does not change the shared symbol identity or
workspace reference set.

Same-module bare schema and alias paths can resolve to their selected identity.
Imported-module uses must be named by an accepted qualified path. Schema
composition and operation paths also accept a unique implicit leaf import
alias. For those paths, an exact full written import path takes
precedence over a same-spelled implicit leaf alias. Otherwise, an implicit
leaf alias selects a schema identity only when it is unique across
workspace and package imports. Conflicting exact imports select no identity.
Duplicate and syntax-recovered dependency imports do not grant dependency
schema visibility. A written import does not make the
imported schema or alias available as a bare schema path. It does not add
module-qualifier, recovery, or rename behavior for
schemas.

A rename request that would create a same-namespace duplicate or a provable
ambiguity in an affected module or lexical scope returns JSON-RPC invalid
params with code `-32602`. The error payload preserves the shared
`rename.conflict` code and includes the selected symbol class, requested name,
conflicting declaration location, and affected scope. The request returns no
workspace edits in that failure response.

Recovery rename uses the same identifier-class validation for selected
recovery declarations and bindings. Type and constructor recovery replacements
must start with an ASCII uppercase letter. Function and value-binding recovery
replacements must start with an ASCII lowercase letter. A selected recovery
rename edits the retained invalid declaration and every linked in-scope
reference returned by the shared recovery selector. The same conflict failure
shape applies when a recovery rename would create a predictable same-namespace
or lexical duplicate. A class failure, conflict failure, unsupported
selection, ambiguous recovery selection, incompatible role, shadowed
occurrence, qualified occurrence, local-binding initializer occurrence, or
out-of-scope occurrence returns no workspace edits.

Lexical conflict prediction rejects same-scope declaration duplicates
independently of edited references. Lexical shadowing checks preserve an
unedited occurrence when the complete edit would leave that occurrence bound
to the same local binding or clause parameter. A handler operation clause
parameter can be renamed to an enclosing handler context parameter's name when
the edited declaration and references remain bound to the clause parameter. A
handler context parameter can be renamed to a clause parameter's name only when
the edited references stay outside the clause parameter's lexical scope.
Otherwise, the existing clause parameter is the reported conflict. A
lexical affected scope has `kind: "lexical"` and includes the file plus source
start and end offsets.
When a local binding conflicts with a function rename, the reported conflict
location is the binding declaration. Function declarations and test
declarations share the function rename namespace, so either declaration can
block the other. When a function rename is captured by a function parameter or
result binding in an affected lexical scope, the reported conflict location is
that parameter or result-binding declaration. Handler context parameters and
operation-clause parameters are lexical bindings for this function-rename
capture check. When an edited bare function call or bare function-value
reference would bind to one of those handler parameters after the complete
edit, the reported conflict location is the handler parameter declaration.
When an unused handler operation clause parameter is renamed to another
parameter in the same clause, the reported conflict location is the existing
clause parameter declaration.

Module conflict prediction checks workspace modules where the renamed symbol
would be visible after the complete edit, including requested-name occurrences
that were not references to the selected symbol before the rename. A module
affected scope has `kind: "module"` and the module name. Type conflict
prediction uses the current top-level type namespace, so a selected source type
cannot be renamed to the name of a type declaration or visible type alias in an
affected module.
Constructor conflict prediction uses the current constructor namespace for the
selected ADT and bare constructor expression and pattern uses in modules where
the renamed constructor would be visible after the complete edit, including
visibility through an imported public type alias that re-exports the selected
ADT. Equal-spelled effect operation declarations and handler operation clause
headings do not participate in constructor conflict prediction and are not
constructor rename references.
Function conflict prediction checks bare call targets and bare function-value
occurrences in modules where the renamed function would be visible after the
complete edit.

For a workspace public function alias, prepare-rename and rename select the
alias identity at its declaration and at calls that resolve through the alias.
Rename edits the alias declaration and those alias calls. It does not edit the
target function declaration or direct calls to that target. Workspace type
aliases remain unsupported by LSP prepare-rename and rename, even though the
MCP rename tool supports their separate alias identity as specified in
[mcp.md](mcp.md#rename).

A rename request without a selected supported workspace symbol returns an empty
workspace-edit `changes` object, and prepare-rename for the same position
returns `null`.

The retained dependency input contains the package identity, captured package
snapshot, manifest export paths, and canonical virtual-source catalog derived
from the same identity and snapshot. A qualified call through
`use module from "package"` can resolve to a function in that dependency only
when the dependency identity matches, the function's source is listed in
`[lib].exports`, and the function is public.
An invalid-cased exported dependency source path is excluded from the retained
public export set, so it cannot produce a dependency definition result. A
valid sibling export in the same dependency remains visible.
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
public functions and supported public function aliases from the exported
standard prelude. A function parameter or local binding with the same name
shadows the bare prelude fallback at call sites in its scope; the same
standard function or supported function alias remains reachable through an
explicit `prelude::` call. A qualified call through `use module from "std"`
resolves a public function or supported public function alias only from an
exported standard source. Private declarations and declarations in
non-exported standard sources do not produce definition results.

`textDocument/definition` returns a dependency or standard declaration with
the exact canonical `veln-pkg:` URI from the retained catalog. It does not
convert the location to a `file:` URI. It exposes neither a dependency
materialization path nor a standard-library build path. Workspace definitions
continue to use `file:` URIs. Private functions and functions in non-exported
package sources have no package definition result. Package declarations are
immutable locations:
`textDocument/prepareRename` returns no range for them, and
`textDocument/rename` returns no workspace edits for them. `textDocument/references`
returns no package locations for dependency or standard-library declarations
in this slice. Supported direct-dependency and standard-library public
function aliases return selected-project workspace `file:` locations for LSP
references. For a project-wide request, MCP returns those same workspace
locations and, when `include_declaration` is true, also includes the eligible
canonical `veln-pkg:` declaration. A single-file MCP request does not include
that package declaration. Supported direct-dependency and standard-library
public type aliases follow the same LSP and MCP declaration policy.
Unsupported schema-alias origins or scopes, public function aliases with unresolved,
non-function, or invalid-cased targets, and public type aliases with
transitive, unresolved, non-type, or invalid-cased targets do not produce
definition or reference locations.

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
reference locations used in the response.

For a handler operation clause binding, `textDocument/definition` returns the
binding location from the operation clause parameter list.
`textDocument/references` returns the binding and ordinary expression
references inside the clause body. `textDocument/prepareRename` returns the
binding range, and `textDocument/rename` edits the binding and references in the
clause body. Record field labels and field accesses that use the same text are
not binding references.

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
binding.

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

Implemented support includes:

- TextMate fallback highlighting and editor-neutral semantic token records.
- Full-document semantic token legend and relative integer encoding.
- Stdio lifecycle, diagnostics, definition, references, formatting,
  prepare-rename, and rename responses described above.
- Workspace and document-scoped diagnostics, unsaved overlays, source-casing
  diagnostics, and selected-project isolation.
- Navigation for workspace declarations, exact companions, handler bindings,
  embedded standard-library exports, and admitted direct dependencies.
- Canonical `veln-pkg:` virtual-document reads for retained package sources,
  including private and non-exported distribution sources when publication
  permits them.
- VSCode activation, semantic tokens, Problems-pane diagnostics, and the
  `veln-pkg:` content provider.

The server does not implement range or delta semantic-token requests,
completion, or hover. General dependency search, definition, and rename remain
limited to the supported public and snapshot-bound declaration classes above.
Unsupported symbols, invalid source modules, wrong package snapshots, comments,
strings, field labels, and out-of-scope bindings return no selected symbol,
range, or edits. Rename rejects identifier-class changes and predictable
namespace or lexical conflicts with a structured failure and no workspace edit.
Formatting returns one whole-document `TextEdit` in the response; it does not
return a multi-file workspace edit.

## References

The authoritative implementations are `crates/veln-editor`, `crates/veln-lsp`,
and `crates/veln-language-service`. Their unit and protocol checks verify
the token legend, LSP encoding, workspace diagnostics, navigation, formatting,
rename, and virtual-document boundaries.
The checked `examples/specification/lsp/references-workspace-effect/` transcript
demonstrates declaration policy and UTF-16 conversion for effect references.
