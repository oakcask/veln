---
role: specification
authority: normative
specification-coverage: usage=#fixed-behavior; behavior=#fixed-behavior; limits=#outside-this-reference
update-when: The implemented command loop, diagnostic envelope, source identity, module-value command and package-documentation availability boundary, or documented stability boundary changes.
---

# Language Specification Overview

This file defines the stability boundary for behavior implemented in the
current Veln workspace.

## Fixed Behavior

The following behavior is fixed for the implemented slice:

- The standard edit loop is `veln fmt`, `veln check --json`,
  `veln run [--json] <entry> [-- arg ...]`, `veln test [--json]`, and
  `veln doc`.
- The JVM toolchain has a Java 25 baseline. Building the backend requires
  `javac` 25 or newer, generated classes target Java 25, and `veln run` and
  `veln test` reject an older selected runtime before entry launch. There is no
  older-JDK or lower-classfile fallback. See
  [execution.md](execution.md#runtime-readiness-and-host-boundaries),
  [command-run.md](command-run.md#analysis-and-execution-gates), and
  [command-test.md](command-test.md#gates-and-failures).
- JVM task execution uses one unpooled virtual thread per admitted task, while a
  non-daemon platform entry launcher owns invocation lifetime. Each run and
  test entry owns a distinct task lifetime: successful entries drain admitted
  descendants, failed entries cancel and drain them, and a task or cleanup
  that never finishes keeps the invocation incomplete without an implicit
  deadline. See [effects.md](effects.md#concurrency-calls),
  [execution.md](execution.md#lightweight-task-execution-and-invocation-lifetime),
  [command-run.md](command-run.md#analysis-and-execution-gates), and
  [command-test.md](command-test.md#selection-and-jobs).
- `veln check` and `veln test` extract executable doctest fences from sources
  with canonical virtual-source origins; a rejected origin reports
  `module.invalid_source_path` without stopping analysis of other selected
  input. `veln test` can compare adjacent expected stdout and stderr fences.
- Source paths in diagnostics and JSON are project-relative paths using `/`
  separators.
- An explicit import alias is a qualified-only local binding. The alias path
  and the import's full written target path preserve one target package and
  module identity through semantic analysis, lowering, reachability, and
  execution. See [source-surface.md](source-surface.md#usage-and-declaration-forms),
  [name-resolution.md](name-resolution.md#explicit-import-aliases), and
  [execution.md](execution.md#runtime-readiness-and-host-boundaries).
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
- A named-generic argument or function input, variadic-element, result, or
  effect-set mismatch makes the complete checked program and package-documentation
  catalog unavailable. Analysis retains all
  applicable mismatches instead of publishing a partial package signature;
  `veln run` applies this gate before entry reachability and does not execute
  the rejected program. See [types.md](types.md#compatibility-and-limits),
  [command-run.md](command-run.md#analysis-and-execution-gates), and
  [package-documentation.md](package-documentation.md#generation-gates).
- Typed module `let` declarations have a fixed parsing, formatting, and
  package-aware same-module and imported semantic-analysis surface for finite
  immutable data. `veln fmt` accepts a valid declaration. After initializer
  and package-wide dependency checking, `veln check`, `veln run`, and `veln
  test` accept valid declarations and runtime reads materialize their finite
  data. Package documentation publishes accepted public declarations; see
  [package-documentation.md](package-documentation.md#published-boundary).
  Checked core and typed IR retain each accepted declaration, finite
  initializer, and selected package-aware reference. See
  [source-surface.md](source-surface.md#module-immutable-value-declarations),
  [diagnostics-json.md](diagnostics-json.md#diagnostic-families), and
  [execution.md](execution.md#runtime-readiness-and-host-boundaries).
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
