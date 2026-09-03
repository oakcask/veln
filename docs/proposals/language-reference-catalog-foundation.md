---
role: proposal
update-when: The executable source grammar, specification-case manifest contract, compiler-owned public token tables, or planned language-reference catalog evidence changes.
---

# Language Reference Catalog Foundation

## Summary

Generate one checked, transport-independent language-reference catalog from
implemented Veln authorities. This slice establishes the catalog and its
stable digest without adding MCP resources, search, or rendered documentation.

The generator is a repository-maintenance package named
`veln-repo-language-reference` under `../../tools/`.

## Scope

| Included | Excluded |
| --- | --- |
| The closed topic catalog and validation of its descriptors. | MCP resource schemas, resource lifetime, `search_docs`, and `read_doc`. |
| Complete and selected production output from the executable source grammar. | A second hand-maintained grammar or changes to accepted Veln syntax. |
| Displayable source files selected from existing specification cases. | Harness commands, assertion metadata, diagnostics metadata, and unchecked examples. |
| Public keyword and punctuation projections from compiler-owned records. | Package documentation and tables without a compiler-owned source. |
| Canonical schema-v1 JSON, a domain-separated digest, generation, and freshness verification. | Markdown or web rendering, search indexes, pagination, and plugin packaging. |

The slice does not publish development specifications, proposal text,
implementation records, repository paths, maintenance commands, unfinished
behavior, timestamps, build paths, or a compiler binary version.

## Topic Catalog

The catalog contains exactly these v1 topics:

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

Each topic descriptor has a lowercase ASCII identifier, title, summary,
keywords, supporting body, related topic identifiers, selected grammar
productions, and selected checked examples. A descriptor selects authoritative
inputs; it does not copy their source into another maintained input.

Generation rejects an invalid or duplicate topic identifier, empty required
metadata, duplicate normalized set values, a missing or self-referential topic
relation, an unknown or duplicate grammar production, and an example file that
is not a source input of the selected specification-case command.

## Canonical Artifact

The checked artifact is canonical schema-v1 JSON. Object keys are
lexicographically ordered, insignificant whitespace is absent, and the file has
one terminal LF. Catalog-owned text is NFC with LF line endings. Set-valued
catalog fields are sorted after normalization. Selected source preserves its
scalar content after newline normalization.

The lowercase digest is SHA-256 over this transcript:

1. ASCII `veln-language-reference/v1`, followed by one zero byte;
2. the canonical artifact byte length as one unsigned 64-bit big-endian
   integer; and
3. the exact artifact bytes.

An ordinary Cargo build and workspace test consume the checked artifact. They
do not require SWI-Prolog. The explicit freshness route executes the source
grammar and compares the regenerated artifact and digest with the checked
files.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Generate the closed v1 catalog. | The artifact contains the ten topic identifiers, their validated relations, selected semantic blocks, and schema and generator-contract version `1`. | Generator catalog test and checked artifact. |
| Generate the lexical topic and a topic with selected productions. | The lexical grammar block is byte-equivalent to normalized complete executable-grammar output. Each selected block comes from its named executable production. | Grammar parser and selection tests plus the existing accepted and rejected grammar fixtures. |
| Select a checked example. | The selected case exists, the display name is nonempty after normalization, and every displayed file is a source input of that case's manifest command. | Descriptor validation tests and the existing toolchain specification harness. |
| Generate public token tables. | Keyword and punctuation entries come from compiler-owned records, and the projections cover lexer recognition in both directions. | Lexer projection coverage tests. |
| Supply invalid descriptor metadata, relations, grammar names, or example selections. | Generation fails before producing a replacement checked artifact. | Table-driven descriptor rejection tests. |
| Reorder equivalent set-valued inputs or vary catalog-owned Unicode and line endings. | Canonical bytes and the domain-separated digest remain equal after the specified normalization. Selected source changes only according to newline normalization. | Canonicalization cases and fixed digest transcript vectors. |
| Change a selected grammar, example, descriptor, or public token input. | Freshness verification fails until the checked artifact and digest are regenerated. A change confined to development documentation leaves both unchanged. | Input-mutation freshness cases and development-document independence case. |
| Inspect the generated bundle. | It contains no repository provenance, proposal material, maintenance command, build path, timestamp, or compiler binary version. | Bundle exclusion assertions. |
| Run the ordinary build without SWI-Prolog, then run the explicit freshness route with it. | The ordinary build consumes checked bytes. The freshness route executes the grammar and rejects any byte or digest mismatch. | Cargo consumer test and CI freshness check. |

## Completion

This proposal is complete when all acceptance rows pass, the checked artifact
and digest are current, and the implemented catalog contract is stated by the
smallest matching page under `../specification/`. Remove this page from the
Ready catalog and retain completion history under
`../reference/implemented-proposals/`.
