---
role: proposal
update-when: The public standard-library byte collection length API, public function-alias behavior, or planned byte-length alias evidence changes.
---

# Standard-Library Byte Length Aliases

## Summary

Add `byte_chunk_len` and `byte_view_len` as public `std::prelude` function
aliases for `byte_chunk_count` and `byte_view_count`. This gives byte
collections the same concise length vocabulary as `vec_len` while preserving
the typed `ByteCount` result and every existing source spelling.

## Readiness

This proposal is ready because the standard library already exports the two
target functions and the language already supports public function aliases,
implicit prelude lookup, qualified prelude lookup, and package documentation
for aliases. The existing targets already return `ByteCount`, so the aliases
do not require a new runtime primitive, type rule, or byte representation.

## Scope

The standard prelude exports these aliases:

```veln
pub fn byte_chunk_len = byte_chunk_count
pub fn byte_view_len = byte_view_count
```

Each alias has the target function's parameter and result types. Bare calls,
`prelude::`-qualified calls, and function-value uses follow the existing public
function-alias rules. The aliases return the same `ByteCount` value as their
targets for the same `ByteChunk` or `ByteView` input.

The existing `byte_chunk_count` and `byte_view_count` functions remain public
and unchanged. This proposal does not deprecate either target, rewrite
existing call sites, add a generic collection-length protocol, add
compiler-known intrinsics, change byte storage, or implement MCP reference
lookup.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Call each alias in bare and `prelude::`-qualified form with empty and non-empty inputs. | Each call typechecks as returning `ByteCount` and returns the same value as the corresponding `count` target. | One executable prelude-helper specification case covering both aliases and both target functions. |
| Use each alias as a function value with its target's function type. | The alias is accepted with the same parameter and result types as its target. | The executable prelude-helper case plus standard-library semantic coverage. |
| Continue to call `byte_chunk_count` and `byte_view_count`. | Existing source remains accepted with unchanged results. | The same executable case and the existing standard-library suite. |
| Inspect generated standard-library source and package documentation. | Both aliases are exported as public function aliases and the generated bundle remains fresh. | Standard-library virtual-source and package-documentation freshness checks. |

## Completion

This proposal is complete when every acceptance row has checked evidence,
`docs/specification/prelude-helpers.md` states the implemented aliases, and the
generated standard-library documentation artifacts include them. Move the
completed record to `docs/reference/implemented-proposals/` and remove this
page from the active proposal catalog. Only then may the standard-library
function-alias reference proposal move from Blocked to Ready.
