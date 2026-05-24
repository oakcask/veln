# Language Specification Overview

This file defines the stability boundary for behavior implemented in the
current Veln workspace.

## Fixed Behavior

The following behavior is fixed for the implemented slice:

- The standard edit loop is `veln fmt`, `veln check --json`,
  `veln run <entry>`, and `veln test [--json]`.
- Source paths in diagnostics and JSON are project-relative paths using `/`
  separators.
- Diagnostics use the stable top-level check JSON envelope described in
  [diagnostics-json.md](diagnostics-json.md).
- Human diagnostics keep the primary message focused on the failed fact at the
  reported span; causes, provenance, and repair hints belong in related notes.
- `NodeId` values are session-local and deterministic for a single parse/lower
  pass. They are stable enough for diagnostics in one command result, but are
  not persistent source IDs.

## Outside This Reference

The following behavior is not fixed by this reference:

- The broader target grammar in
  [../../proposals/grammar-target.md](../../proposals/grammar-target.md) where
  parser, AST, lowering, or backend support is absent.
- The exact shape of kind-specific diagnostic `details` fields not listed in
  [diagnostics-json.md](diagnostics-json.md).
- Runtime contract enforcement, package manifests, imports, modules beyond
  source discovery, persistent build caches, and entry arguments.
