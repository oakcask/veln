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
- Diagnostics use the stable top-level check JSON envelope described in
  [diagnostics-json.md](diagnostics-json.md).
- Human diagnostics keep the primary message focused on the failed fact at the
  reported span; causes, provenance, and repair hints belong in related notes.
- `begin` expressions and `defer` statements have a fixed source, static
  semantics, formatting, and navigation surface. A reachable use remains
  non-executable and blocks checked-core and typed-IR readiness with
  `deferred_cleanup_runtime`. An internal checked-core, typed-IR, and JVM
  foundation implements registration-time capture and reverse-order cleanup
  on normal completion. Public integration and the remaining failure and
  cancellation paths remain proposal work. See
  [source-surface.md](source-surface.md#static-cleanup-region-forms) and
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
