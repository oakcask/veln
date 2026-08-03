---
review-when: The documented editor-facing behavior or its LSP server evidence changes.
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
- LSP `textDocument/definition`, `textDocument/prepareRename`, and
  `textDocument/rename` handle the implemented companion private-function
  identity case in `veln-lsp`.
- The stdio LSP server starts through `veln lsp`.
- TextMate fallback highlighting is contributed by
  `editors/vscode/syntaxes/veln.tmLanguage.json`.
- VSCode starts the language server when a `.veln` document opens and requests
  full-document semantic tokens. It enables workspace diagnostics for Veln
  project folders.

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
`definitionProvider`, `renameProvider.prepareProvider`, and
`semanticTokensProvider` with full-document semantic token support. It handles
`initialize`, `initialized`, `shutdown`, `exit`, `textDocument/didOpen`,
`textDocument/didChange`, `textDocument/didClose`,
`textDocument/semanticTokens/full`, `textDocument/definition`,
`textDocument/prepareRename`, and `textDocument/rename`.

The full response uses LSP relative integer encoding in groups of five:

1. delta line
2. delta start character
3. token length
4. token type index
5. token modifier bitset

Tokens are sorted before encoding. Overlapping ranges are skipped so the encoded
stream remains valid for LSP clients.

## LSP Diagnostics

The stdio server resolves workspace roots from `initialize.workspaceFolders`.
When a resolved folder has no `veln.toml`, nested manifest directories become
workspace roots; if no nested manifests are found, the original folder remains
the workspace root. When no workspace folders are present, it falls back to
`initialize.rootUri`. When the client sends no workspace identity, the server
leaves workspace roots empty and publishes document-scoped diagnostics for open
documents only.

For files inside a resolved workspace root, the server discovers project
`.veln` files the same way `check` and `run` do, excludes doctest-generated
sources, overlays open unsaved editor text onto the discovered source set, and
includes open new `.veln` buffers that do not exist on disk yet. It publishes
`textDocument/publishDiagnostics` for every discovered or open workspace source
file, including unopened files. It also publishes empty diagnostic lists for
previously reported files that become clean or leave discovery.

For documents outside resolved workspace roots, diagnostics remain
document-scoped and are computed from the in-editor document text. Parse
diagnostics are reported first. When parsing succeeds, the server lowers the
document into the surface module model and publishes semantic diagnostics from
the checked surface module.

Published diagnostics use standard LSP severity numbers and zero-based ranges.
The diagnostic `code` is the Veln diagnostic id, and the diagnostic `source` is
`veln`.

## LSP Definition And Rename

For a private target function reference written as `target::name` from the
exact `.test.veln` companion, `textDocument/definition` returns the private
function declaration location in the target `.veln` source when the companion
writes an explicit `use` for that target. `textDocument/prepareRename` returns
the selected function-name range for the same accepted identity.

`textDocument/rename` for that identity returns workspace edits for the target
function declaration, valid call or function-value references in the target
source, and valid qualified call references in the exact matching companion. A
same-named companion-local declaration, bare companion reference, or target
source reference shadowed by a callable parameter is a different symbol and is
not edited. Calls through another qualifier, comments, and string literals are
not edited. Definition, prepare-rename, and rename requests whose selected text
is inside a comment or string literal do not identify the private target
function. Wrong companions, `_test.veln` integration modules, and references
through a target dependency do not receive private-function definition or rename
results.

Definition and rename use the same open-document overlays as workspace
diagnostics. Unsaved target or companion text can provide the declaration and
reference locations used in the response. The routed executable evidence is
`../../examples/specification/lsp/companion-private-function-identity/`. The
`veln-lsp` server tests also cover companion private-function definition,
prepare rename, rename edits, source-scope isolation, target-source function
values and callable shadowing, rejected boundaries, request-origin filtering,
and open-document overlays.

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
package manifest and no nested manifest roots, the extension searches that
folder in name order, ignores `.git` and `target`, and uses the parent
directory of the first discovered `.veln` source as an anonymous package root.
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
- Stdio JSON-RPC lifecycle for semantic highlighting requests.
- Stdio definition, prepare-rename, and rename responses for exact companion
  qualified private-function references.
- Stdio diagnostic publication for discovered workspace Veln files across
  resolved workspace roots, including unopened files, with unsaved open
  document overlays.
- Document-scoped diagnostic publication for Veln documents outside resolved
  workspaces or when no workspace identity is initialized.
- VSCode startup for `.veln` files using the configured language-server
  command.
- VSCode Problems pane integration for Veln diagnostics.
- Rust tests for collector classification, LSP relative encoding, ordering, and
  overlap handling, and server initialize/full-token/diagnostic/navigation
  responses.

Not implemented:

- LSP range and delta semantic token requests.
- Completion and hover.
- General rename and go-to-definition support outside the implemented
  companion private-function identity case.
