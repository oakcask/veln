---
role: implementation-record
authority: supporting
update-when: Generated documentation behavior or its executable evidence changes.
---

# Agent Module, Package, And Documentation Model

This page records completion evidence for the module, package, and generated
documentation target. Use the specification pages for current behavior.

## Read First

- Current source surface:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current command behavior:
  [../../specification/commands.md](../../specification/commands.md).
- Planned work now only belongs in
  [../../proposals/README.md](../../proposals/README.md).

## Outcome

The target implemented package metadata and tool metadata in `veln.toml`.
String-valued `[package]` fields describe package-scale facts, and
string-valued `[tool.<name>]` fields describe tool-specific facts. Later
module and manifest work superseded the original source `mod` and manifest
module metadata boundary; use the current specification for source module
identity and manifest export behavior.

`veln doc` generates Markdown from selected non-companion source files and the
manifest. The output includes package and tool metadata, source modules,
imports, public types, public constructors, public functions, contracts,
documentation line comments, executable doctest fences, expected-output
fences, and ADR-lite records. Hidden doctest setup lines are omitted from
generated examples. Exact `.test.veln` companions are excluded from generated
public documentation, and `_test.veln` integration-test modules remain
ordinary documentation inputs.

Dedicated export lists were not added by this target. Later manifest work
added the current `[lib].exports` package export surface.

Later language-service work added a transport-independent package
documentation catalog for exported package APIs. The catalog is separate from
`veln doc` Markdown generation. It binds a validated package identity,
captured package snapshot, and same-capture manifest. It publishes canonical
result bytes, a documentation digest, stable `veln-doc:` resource URIs,
status-only failure results, exported public modules and declarations,
constructor documentation, public schema documentation references, visible
doctests, stream-aware expected-output fences, declaration-location lookup,
and a closed package metadata allowlist.

## Completion Evidence

- Manifest package and tool fields are parsed and preserved independently from
  source declarations.
- `veln doc` has a parse gate and refuses documentation output when manifest
  validation reports errors.
- Generated documentation is covered by
  `../../../examples/specification/doc/generated-markdown/`.
- Companion exclusion from generated documentation is covered by
  `../../../examples/specification/doc/companion-discovery-exclusion/`,
  `../../../examples/specification/doc/companion-explicit-exclusion/`,
  `../../../examples/specification/doc/companion-only-result/`, and
  `../../../examples/specification/doc/integration-test-suffix-rendered/`.
- Public API documentation derives from declarations and attached source
  comments, not from proposal text.
- ADR-lite records remain source metadata and appear in generated docs without
  affecting parsing, checking, lowering, or execution.
- The transport-independent package documentation catalog is specified by
  `../../specification/package-documentation.md` and covered by
  `cargo test -p veln-language-service` plus package catalog fixtures under
  `../../../examples/specification/doc/`.

## Read When

- Checking why package metadata and generated documentation are no longer
  listed as active proposal work.
- Reviewing completion evidence before changing manifest or documentation
  generation behavior.
