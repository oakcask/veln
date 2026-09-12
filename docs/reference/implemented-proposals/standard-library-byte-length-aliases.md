---
role: implementation-record
authority: supporting
update-when: The public standard-library byte collection length API, public function-alias behavior, or byte-length alias evidence changes.
---

# Standard-Library Byte Length Aliases

This record preserves the completed addition of `byte_chunk_len` and
`byte_view_len` as public `std::prelude` function aliases. Current behavior is
specified in [prelude-helpers.md](../../specification/prelude-helpers.md) and
checked by the executable prelude-helper case.

## Outcome

- `std::prelude` exports `byte_chunk_len` as a public function alias of
  `byte_chunk_count`.
- `std::prelude` exports `byte_view_len` as a public function alias of
  `byte_view_count`.
- Each alias has the target function's parameter and result types and returns
  the same `ByteCount` value as its target for the same byte collection input.
- The existing `byte_chunk_count` and `byte_view_count` spellings remain public
  and unchanged.

## Completion Evidence

- `examples/specification/run/prelude-helpers/` covers empty and non-empty
  `ByteChunk` and `ByteView` inputs through bare alias calls,
  `prelude::`-qualified alias calls, the existing target functions, and alias
  use as function values.
- `crates/veln-sema/src/tests/calls_pipeline_and_float_types/pipelines_and_aliases.rs`
  checks that public alias calls and function-value uses lower through the
  source-backed alias targets.
- The standard-library virtual-source and package-documentation specification
  cases are refreshed with the standard-library snapshot that includes both
  public aliases.

## Boundary

This work does not deprecate either `count` spelling, rewrite existing call
sites, add a generic collection-length protocol, add compiler intrinsics,
change byte storage, or implement MCP function-alias reference lookup.
