---
role: specification
authority: normative
update-when: The language-reference catalog schema, checked artifact, digest transcript, source authorities, freshness route, or executable catalog evidence changes.
---

# Language Reference Catalog

The language-reference catalog is a checked, transport-independent schema-v1
JSON artifact generated from implemented Veln authorities. It is not exposed
through MCP, documentation search, rendered Markdown, pagination, or plugin
packaging in the current behavior.

## Current Contract

The repository-maintenance package stores the checked schema-v1 JSON artifact
and its checked digest under its generated-output directory.
Ordinary Cargo builds consume those checked files. Ordinary package tests
validate the checked files, schema fixture, descriptor and example rejection
rules, bidirectional token projection, canonicalization, digest transcript,
bundle exclusions, and selected example inputs without executing the
source-surface grammar.

The artifact has `schema_version` `1` and `generator_contract_version` `1`.
It contains exactly the topic identifiers listed by the executable
language-reference contract fixture under `examples/specification`.
Each topic has validated descriptor text, normalized set-valued fields,
validated related-topic identifiers, selected executable grammar productions,
and selected displayed source files from specification case command inputs.

The lexical topic includes the normalized complete output of
`source-surface-executable.pl --grammar`. Selected grammar blocks come from
named productions in that same output. Keyword and punctuation tables come from
compiler-owned public token records. The lexer uses the public keyword records
for recognition and the public punctuation records for fixed punctuation
recognition. Package tests validate every public fixed-spelling token record
against lexer recognition and validate that every compiler-owned public
fixed-spelling token appears in the catalog projection.

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

## Verification

Run `cargo test -p veln-repo-language-reference` to check the ordinary
consumer path, schema closure, descriptor rejection cases, selected example
inputs, token projection, canonicalization, digest vectors, and bundle
exclusions.

Run `cargo run -p veln-repo-language-reference -- . check-fresh` to execute
the source grammar and reject artifact or digest drift.
