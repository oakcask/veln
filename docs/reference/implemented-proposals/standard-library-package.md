---
review-when: The standard-library package record is superseded or its evidence changes.
---

# Standard Library Package

Status: implemented

This record preserves the completed migration from function-level embedded
source metadata to the toolchain-owned `std` package. Current behavior is
specified in
[names-effects.md](../../specification/names-effects.md) and
[commands.md](../../specification/commands.md).

## Outcome

- `crates/veln-stdlib/veln/veln.toml` defines package `std` and exports
  `prelude.veln`; `compiler_support.veln` is private.
- The standard-library build scans the package source tree deterministically,
  embeds all non-test Veln sources, and excludes `*_test.veln` and
  `.test.veln`.
- Project analysis injects the embedded package and an origin-tagged implicit
  `std::prelude` import. `std::prelude` is the only bootstrap exception.
- Standard calls resolve to Veln functions with collision-resistant internal
  names. Reachable public and private bodies lower normally; only
  `prelude_builtin::*` remains intrinsic.
- Compiler adapters preserve expected-type and callback inference without
  owning declaration visibility or helper bodies.
- User packages and dependencies cannot replace `std`, and package locking
  never emits it.

## Completion Evidence

- `crates/veln-stdlib/veln/prelude_test.veln` covers Veln control flow,
  collection callbacks and private helpers, `Option` and `Result`, source ADTs,
  and intrinsic delegation through ordinary `veln test` execution.
- CLI fixtures cover reserved root-package and dependency diagnostics and the
  package-lock rejection.
- Bundle tests cover package identity, exports, deterministic source inclusion,
  and test-source exclusion. Compiler adapter tests require every adapter name
  to be a public function declared by `std::prelude`.

## Boundary

- `Option`, `Result`, and `List` remain compiler-owned ADTs.
- The standard package is fixed to the toolchain; this work does not add a
  sysroot selector, version solver entry, or user-replaceable standard library.
- Float operator compatibility adapters and direct low-level surface-analysis
  fallback remain compiler-owned compatibility paths.
