# Editor Semantic Highlighting

Status: promoted
Implementation: promoted to reference: compiler token classification, TextMate
fallback grammar, LSP full-token data encoding, stdio JSON-RPC server
lifecycle, and VSCode startup.

This proposal originally defined editor support for semantic highlighting
through an LSP server while keeping the classification logic editor-neutral.
Implemented behavior is specified in
[../reference/language/editor-support.md](../reference/language/editor-support.md).

## Read First

- Current source syntax and token behavior:
  [../reference/language/source-surface.md](../reference/language/source-surface.md).
- Current name, prelude, and effect behavior:
  [../reference/language/names-effects.md](../reference/language/names-effects.md).
- Current implementation boundary:
  [implementation boundaries][implementation-boundaries].

## Goal

Provide useful syntax highlighting for Veln editors in two layers:

- TextMate grammar for immediate lexical highlighting.
- LSP semantic tokens for meaning-aware highlighting after parsing and semantic
  analysis.

The VSCode extension should stay thin. It registers the language, loads the
TextMate grammar, starts the LSP client, and maps the server's semantic token
legend. Compiler crates own token classification.

## Design

Add an editor-neutral semantic token collector to the Rust toolchain. The
collector returns Veln-owned records rather than LSP's encoded integer stream:

```rust
pub struct SemanticToken {
    pub span: SourceSpan,
    pub kind: SemanticTokenKind,
    pub modifiers: SemanticTokenModifiers,
}
```

The LSP server converts these records into `textDocument/semanticTokens/full`
responses. Initial support should use full refresh only. Range and delta
requests can wait until the editor integration has stable behavior.

The collector should combine:

- `veln-syntax` lossless tokens for lexical fallback.
- parsed surface AST facts for declarations and structural spans.
- semantic analysis facts for resolved names, prelude symbols, binding kind,
  and error-tolerant classification.

Parse or semantic diagnostics must not make semantic token requests fail. When
semantic facts are unavailable, the server should return lexical tokens and any
safe partial semantic classifications.

## Token Classes

Prefer standard LSP token types so existing themes work without Veln-specific
theme support.

| Veln source element | LSP token type | Modifiers |
| --- | --- | --- |
| module name | `namespace` | `declaration` |
| use alias | `namespace` | `declaration` |
| function declaration name | `function` | `declaration` |
| function call | `function` | none |
| test declaration name | `function` | `declaration`, `test` |
| parameter declaration | `parameter` | `declaration`, `readonly` |
| parameter reference | `parameter` | `readonly` |
| let binding declaration | `variable` | `declaration`, `readonly` |
| local binding reference | `variable` | `readonly` |
| result binding | `variable` | `declaration`, `readonly`, `result` |
| type name | `type` | none |
| effect label | `enumMember` | none |
| record field | `property` | none |
| hole | `variable` | `hole` |
| prelude function | `function` | `defaultLibrary` |

Custom modifiers should be minimal. Start with `test`, `result`, and `hole`.
Do not add custom token types unless standard LSP token types cannot represent
the source element well enough for common themes.

## Implementation Path

1. Add a TextMate grammar for comments, strings, numbers, keywords, operators,
   punctuation, holes, and identifiers.
2. Add a semantic token collector over lossless syntax tokens that can return
   lexical fallback tokens without semantic analysis.
3. Extend surface AST data where needed so declaration names, type names,
   effect labels, fields, and binding names have precise spans.
4. Extend semantic analysis or shared side tables so resolved references can be
   classified as function, prelude function, parameter, local binding, field,
   or unresolved name.
5. Add a `veln-lsp` crate that owns open document snapshots, project discovery,
   diagnostics publishing, semantic token legend construction, and LSP position
   encoding conversion.
6. Add a VSCode extension that contributes the `veln` language, TextMate
   grammar, language configuration, and LSP client startup.

## Boundaries

In scope:

- TextMate fallback highlighting.
- LSP `textDocument/semanticTokens/full`.
- editor-neutral semantic token records.
- declaration and reference distinction.
- prelude/default-library classification.
- graceful degradation when parse or semantic errors exist.

Out of scope:

- completion, hover, rename, and go to definition.
- semantic token range and delta requests.
- incremental parsing.
- broad multi-workspace indexing.
- theme-specific color choices.

## Risks

The main implementation risk is span precision. Current AST spans often cover
larger syntactic forms than a single identifier. Semantic highlighting needs
identifier-level spans for declaration names, type names, effect labels, record
fields, and references.

The second risk is coupling the collector to LSP details too early. Keep LSP
position encoding and semantic token integer encoding inside the LSP crate so
the compiler can test semantic classifications without a VSCode runtime.

## Acceptance Criteria

- Opening a `.veln` file in VSCode shows immediate lexical highlighting before
  the LSP server finishes analysis.
- After LSP analysis, function declarations, function calls, parameters, local
  bindings, type names, holes, and prelude functions receive semantic token
  classifications.
- Files with parse or semantic diagnostics still return semantic token results.
- The semantic token collector has Rust tests that do not start VSCode.
- The LSP token encoding has tests for line and column conversion, token
  ordering, and non-overlapping token ranges.

## Skip Unless Needed

- Do not treat this proposal as implemented behavior unless the relevant
  reference page also states it.
- Do not use this proposal to change language syntax, name resolution, or
  diagnostic rules.

[implementation-boundaries]: ../reference/source-decisions/implementation-boundaries.md
