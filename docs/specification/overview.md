---
role: specification
authority: normative
specification-coverage: usage=#fixed-behavior; behavior=#fixed-behavior; limits=#outside-this-reference
update-when: The implemented command loop, diagnostic envelope, source identity, or documented stability boundary changes.
---

# Language Specification Overview

This file defines the stability boundary for behavior implemented in the
current Veln workspace.

## Fixed Behavior

The following behavior is fixed for the implemented slice:

- The standard edit loop is `veln fmt`, `veln check --json`,
  `veln run [--json] <entry> [-- arg ...]`, `veln test [--json]`, and
  `veln doc`.
- `veln check` and `veln test` extract executable doctest fences from
  documentation line comments; `veln test` can compare adjacent expected
  stdout and stderr fences.
- Source paths in diagnostics and JSON are project-relative paths using `/`
  separators.
- A call-site location from a dependency source exposes the dependency's
  public package identity, its package-local logical module path, and its
  package-relative source path. Equivalent dependency trees expose identical
  locations after the project is moved. See
  [call-site-declarations.md](call-site-declarations.md#declaration-behavior).
- A call-site location from generated doctest source uses the original
  documentation path and coordinates only when both span boundaries map to
  that source. Otherwise, it keeps the complete generated span. See
  [call-site-declarations.md](call-site-declarations.md#declaration-behavior).
- A library-generated call-site location uses a canonical virtual path derived
  from its package-relative source and generator identity. The value remains
  unchanged when retained past the originating function return or when the
  source tree moves between absolute roots. See
  [call-site-declarations.md](call-site-declarations.md#declaration-behavior).
- Diagnostics use the stable top-level check JSON envelope described in
  [diagnostics-json.md](diagnostics-json.md).
- Human diagnostics keep the primary message focused on the failed fact at the
  reported span; causes, provenance, and repair hints belong in related notes.
- `begin` expressions and `defer` statements have a fixed source, static
  semantics, formatting, navigation, and executable surface. Checked core,
  typed IR, and the JVM backend implement registration-time capture and
  reverse-order cleanup on normal completion, postfix `?` propagation,
  exceptional exit, and task cancellation. All registered cleanup runs;
  existing failures remain primary with ordered related cleanup failures, and
  the first cleanup failure becomes primary for an otherwise successful
  region. See
  [source-surface.md](source-surface.md#cleanup-region-forms) and
  [execution.md](execution.md#runtime-readiness-and-host-boundaries).
- `NodeId` values are session-local and deterministic for a single parse/lower
  pass. They are stable enough for diagnostics in one command result, but are
  not persistent source IDs.

## Outside This Reference

The following behavior is not fixed by this reference:

- Source-decision history that predates the categorized reference files.
- The exact shape of kind-specific diagnostic `details` fields not listed in
  [diagnostics-json.md](diagnostics-json.md).
- Package manifests beyond implemented package fields, tool fields,
  `[lib].exports` validation, imports, modules beyond source discovery, and
  the exact on-disk layout of build caches.
