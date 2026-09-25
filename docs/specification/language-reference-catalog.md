---
role: specification
authority: normative
specification-coverage: usage=#usage; behavior=#current-contract; limits=#generation-gates
update-when: The language-reference catalog schema, checked artifact, digest transcript, Markdown renderer, MCP publication, source authorities, freshness route, or executable catalog evidence changes.
---

# Language Reference Catalog

The language-reference catalog is a checked, transport-independent schema-v1
JSON artifact generated from implemented Veln authorities. The MCP server
publishes deterministic Markdown resources rendered from this checked
artifact. MCP documentation search uses the checked catalog as specified by
[MCP Workspace Projects, Resources, And Navigation](mcp.md#documentation-tools).
Search pagination and plugin packaging remain outside the current behavior.

## Usage

Consumers read the checked catalog through `veln-repo-language-reference`;
MCP clients discover its index and topic resources through `resources/list`
and retrieve a listed URI with `resources/read`.

From the repository root, run
`cargo run -p veln-repo-language-reference -- . generate` to regenerate the
catalog and digest files after changing their source authorities. `check`
validates the checked digest; `check-fresh` regenerates in memory and compares
the checked artifact and rendered resources. Failures exit with status `1`;
invalid command arguments exit with status `2`.

## Current Contract

The repository-maintenance package stores the checked schema-v1 JSON artifact,
its checked digest, and the checked Markdown resource digest under its
generated-output directory.
Ordinary Cargo builds consume those checked files without executing the
source-surface grammar.

The artifact has `schema_version` `1` and `generator_contract_version` `1`.
Its topic identifiers are `lexical-structure`, `modules-imports-packages`,
`declarations-aliases`, `expressions-patterns`, `types-inference-constructors`,
`effects-handlers`, `contracts`, `schemas`, `holes`, and `tests-docs-doctests`.
Each topic has validated descriptor text, normalized set-valued fields,
validated related-topic identifiers, selected executable grammar productions,
and selected displayed source files from specification case command inputs.

The lexical topic includes the normalized complete output of
`source-surface-executable.pl --grammar`. Selected grammar blocks come from
named productions in that same output. Keyword and punctuation tables come from
compiler-owned public token records. The lexer uses the public keyword records
for recognition and the public punctuation records for fixed punctuation
recognition. Every compiler-owned public fixed-spelling token appears in the
catalog projection.

The digest is lowercase SHA-256 over this transcript:

```text
ASCII "veln-language-reference/v1\0"
u64be(canonical artifact byte length)
canonical artifact bytes
```

Canonical artifact JSON has lexicographically ordered object keys, no
insignificant whitespace, and one terminal LF. Catalog-owned text is NFC with
LF line endings. Set-valued catalog fields are sorted after normalization.
Selected source text is copied from selected specification case source inputs
with newline normalization only.

## Generation Gates

Generation fails before replacing checked output when a descriptor contains an
invalid or duplicate topic identifier, empty required metadata, duplicate
normalized set values, a missing or self-referential topic relation, an
unknown or duplicate grammar production, a non-repository-relative example
case or file selector, an empty displayed-file set, or an example file that is
not a source input selected by the example case manifest command.

The generated bundle excludes repository provenance, proposal material,
maintenance commands, build paths, timestamps, and compiler binary versions.
Development documentation that is not a selected source authority does not
affect the artifact or digest.

## Markdown Resources

The deterministic Markdown renderer consumes only the checked catalog artifact
and digest. It renders one index resource and one topic resource for each
checked topic. The index contains the `Veln Language Reference` heading and one
link plus summary for every topic in catalog order. Each topic resource
contains the title, summary, body paragraphs, selected grammar blocks,
selected examples and displayed file source text, keywords, and related-topic
links derived from the checked catalog.

Resource URIs use the checked digest:

- `veln-doc:///language/snapshot/<digest>/index`
- `veln-doc:///language/snapshot/<digest>/topic/<topic-id>`

Each rendered resource is complete Markdown with media type
`text/markdown; charset=utf-8` and is at most `262144` UTF-8 bytes. The
freshness route rejects renderer or size-limit drift before publication.
Rendered resources do not include repository paths, proposal material,
maintenance commands, timestamps, build paths, or compiler binary versions.

## Verification

The catalog and renderer tests are in
`tools/veln-repo-language-reference/src/tests.rs`.

Run `cargo run -p veln-repo-language-reference -- . check-fresh` to execute
the source grammar and reject artifact, catalog digest, rendered-resource
digest, or size-limit drift. The
`test--language-reference-catalog` GitHub Actions workflow runs the same
freshness command for pull requests and main-branch pushes whose path filters
can affect catalog generation.
