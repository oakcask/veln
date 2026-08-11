---
role: specification
authority: normative
update-when: The schema-v1 published language-reference artifact, topic descriptors, selected specification cases, executable grammar output, compiler-owned token tables, or maintenance commands change.
---

# Published Language Reference Artifact

The checked schema-v1 JSON artifact is the current transport-independent
language-reference catalog. It contains the closed v1 set of ten topics. Each
topic has an identifier, title, summary, keywords, supporting body, related
topic identifiers, selected grammar blocks, and selected checked source
examples.

The topic descriptors select grammar production names and files from existing
specification cases. Generation fails when an identifier is invalid or
duplicated, required metadata is empty, a relation or grammar production is
invalid, or a selected file is absent from its case command. The selected
cases remain executable through the toolchain harness. The artifact includes
the selected source text but does not include case names, repository paths,
proposal text, or maintenance commands.

The executable grammar supplies the complete grammar and selected production
blocks. The lexer-owned keyword and punctuation tables supply the public token
projections. The generator does not maintain copies of those inputs.

## Canonical Bytes And Digest

The artifact uses lexicographically ordered object keys, compact JSON, one
terminal LF, and schema and generator-contract versions of `1`. Catalog-owned
text is Unicode Normalization Form C with LF line endings. Selected source
preserves scalar content after newline normalization.

The lowercase digest is SHA-256 over this transcript:

1. ASCII `veln-language-reference/v1`, followed by one zero byte;
2. the canonical artifact byte length as one unsigned 64-bit big-endian
   integer; and
3. the exact artifact bytes.

The transcript is domain-separated from other Veln digests. The artifact and
digest contain no timestamp, build path, or compiler binary version.

## Maintenance And Verification

An ordinary Cargo build or workspace test consumes the checked artifact and
digest. It does not execute SWI-Prolog. Run the explicit generation route after
changing an input:

```text
cargo run -p veln-repo-language-reference -- generate
```

Run the freshness route to generate the executable grammar with SWI-Prolog and
compare all canonical bytes and the digest:

```text
cargo run -p veln-repo-language-reference -- verify
```

The package tests cover descriptor rejection, selected case-file validation,
canonical serialization, the digest transcript, input-change freshness
failure, and bundle exclusions. Existing toolchain harness cases execute every
selected source file; no separate example duplicates are required.
