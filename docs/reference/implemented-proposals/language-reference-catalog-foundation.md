---
role: implementation-record
authority: supporting
update-when: The completed language-reference catalog foundation scope, checked artifact, digest, freshness route, executable evidence, or current specification route is invalidated or superseded.
---

# Language Reference Catalog Foundation

This record preserves the completed proposal that introduced the checked
transport-independent language-reference catalog. Current behavior is
specified by
[../../specification/language-reference-catalog.md](../../specification/language-reference-catalog.md).

## Completed Scope

- Added `veln-repo-language-reference` as a repository-maintenance package
  under `tools/`.
- Generated the closed schema-v1 catalog for the ten language-reference
  topics from implemented authorities.
- Sourced complete grammar output and selected grammar productions from the
  executable source-surface grammar.
- Sourced displayed example source from files selected by existing
  specification case manifest commands.
- Sourced public keyword and punctuation projections from compiler-owned token
  records, with keyword records used by the lexer and every public token record
  validated against lexer recognition.
- Produced canonical JSON and the domain-separated SHA-256 digest as checked
  artifacts.
- Kept ordinary Cargo build paths on checked artifact bytes and kept ordinary
  package tests independent of SWI-Prolog.
- Added an explicit freshness route that executes the source grammar and
  rejects artifact or digest drift.

## Completion Evidence

| Claim | Checked evidence |
| --- | --- |
| The catalog contains the closed schema-v1 topic set and generator contract version. | `cargo test -p veln-repo-language-reference` and the language-reference contract fixture under `examples/specification` |
| Descriptor metadata, relations, grammar selections, and example selections are validated before output replacement. | `cargo test -p veln-repo-language-reference` |
| Public token tables are compiler-owned; keyword records drive lexer recognition, and every public token record is checked against lexer recognition. | `cargo test -p veln-repo-language-reference` |
| Canonical JSON, normalization, digest transcript vectors, equivalent input cases, and bundle exclusions are checked. | `cargo test -p veln-repo-language-reference` |
| The source grammar freshness route rejects artifact or digest drift after executing SWI-Prolog. | `cargo run -p veln-repo-language-reference -- . check-fresh` |

## Preserved Non-Goals

- MCP resources, documentation search, rendering, search indexes, pagination,
  plugin packaging, and package documentation remain outside this foundation.
- The catalog does not publish development specifications, proposal text,
  implementation records, repository paths, maintenance commands, unfinished
  behavior, timestamps, build paths, or compiler binary versions.
