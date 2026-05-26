# Editor Support

This page specifies implemented editor-facing classification. It covers the
compiler-owned records, LSP semantic-token transport, and VSCode integration
used by editor integrations.

## Read First

- Source lexical tokens come from `veln-syntax`.
- Editor-neutral semantic records come from `veln-editor`.
- LSP `textDocument/semanticTokens/full` integer data comes from `veln-lsp`.
- The stdio LSP server starts through `veln lsp`.
- TextMate fallback highlighting is contributed by
  `editors/vscode/syntaxes/veln.tmLanguage.json`.
- VSCode starts the language server when a `.veln` document opens and requests
  full-document semantic tokens.

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
| parameter declaration | `parameter` | `declaration`, `readonly` |
| parameter reference | `parameter` | `readonly` |
| let binding declaration | `variable` | `declaration`, `readonly` |
| local binding reference | `variable` | `readonly` |
| result binding | `variable` | `declaration`, `readonly`, `result` |
| type name | `type` | none |
| effect label | `enumMember` | none |
| record or field-access field | `property` | none |
| named hole | `variable` | `hole` |
| prelude function | `function` | `defaultLibrary` |

Lexical fallback also classifies comments, strings, numbers, keywords, and
operators with the matching standard LSP token types.

The only Veln-specific semantic token modifiers are `test`, `result`, and
`hole`.

## LSP Encoding

`veln-lsp` exposes the semantic-token legend, full-token response data, and a
stdio JSON-RPC server. The server advertises `textDocumentSync` and
`semanticTokensProvider` with full-document semantic token support. It handles
`initialize`, `initialized`, `shutdown`, `exit`, `textDocument/didOpen`,
`textDocument/didChange`, and `textDocument/semanticTokens/full`.

The full response uses LSP relative integer encoding in groups of five:

1. delta line
2. delta start character
3. token length
4. token type index
5. token modifier bitset

Tokens are sorted before encoding. Overlapping ranges are skipped so the encoded
stream remains valid for LSP clients.

## VSCode Integration

The VSCode extension contributes the `veln` language, the TextMate grammar,
semantic token types and modifiers, and activation for Veln files. On
activation, it starts the command configured by `veln.server.path` with the
`lsp` argument. The default command is `veln`.

The extension registers a document semantic token provider. Before requesting
tokens, it sends the current document text to the server, so highlighting follows
unsaved editor content.

## Boundaries

Implemented:

- TextMate fallback grammar for comments, strings, numbers, keywords,
  operators, punctuation, holes, type-like identifiers, and identifiers.
- Editor-neutral semantic token records.
- Full semantic-token legend and integer data generation for LSP clients.
- Stdio JSON-RPC lifecycle for semantic highlighting requests.
- VSCode startup for `.veln` files using the configured language-server
  command.
- Rust tests for collector classification, LSP relative encoding, ordering, and
  overlap handling, and server initialize/full-token responses.

Not implemented:

- LSP range and delta semantic token requests.
- Completion, hover, rename, and go to definition.
