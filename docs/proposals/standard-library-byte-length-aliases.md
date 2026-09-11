---
role: proposal
update-when: The public standard-library byte collection length API, public function-alias behavior, or planned byte-length alias evidence changes.
---

# Standard-Library Byte View Length Alias

## Summary

Add `byte_view_len` as a public `std::prelude` function alias for
`byte_view_count`. `byte_chunk_len` is already implemented as the matching
public alias for `byte_chunk_count`; this proposal keeps the remaining byte
view work active without restating implemented chunk behavior as planned work.

## Readiness

This proposal is ready because the standard library already exports
`byte_view_count` and the language already supports public function aliases,
implicit prelude lookup, qualified prelude lookup, and package documentation
for aliases. The existing target already returns `ByteCount`, so the alias does
not require a new runtime primitive, type rule, or byte representation.

## Scope

The standard prelude must export this alias:

```veln
pub fn byte_view_len = byte_view_count
```

The alias has the target function's parameter and result types. Bare calls,
`prelude::`-qualified calls, and function-value uses follow the existing public
function-alias rules. The alias returns the same `ByteCount` value as
`byte_view_count` for the same `ByteView` input.

The existing `byte_chunk_count`, `byte_chunk_len`, and `byte_view_count`
functions remain public and unchanged. This proposal does not deprecate a
target, rewrite existing call sites, add a generic collection-length protocol,
add compiler-known intrinsics, change byte storage, or implement MCP reference
lookup.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Call `byte_view_len` in bare and `prelude::`-qualified form with empty and non-empty inputs. | Each call typechecks as returning `ByteCount` and returns the same value as `byte_view_count`. | One executable prelude-helper specification case covering the remaining alias and target function. |
| Use `byte_view_len` as a function value with its target's function type. | The alias is accepted with the same parameter and result types as its target. | The executable prelude-helper case plus standard-library semantic coverage. |
| Continue to call `byte_chunk_count` and `byte_view_count`. | Existing source remains accepted with unchanged results. | The same executable case and the existing standard-library suite. |
| Inspect generated standard-library source and package documentation. | `byte_view_len` is exported as a public function alias and the generated bundle remains fresh. | Standard-library virtual-source and package-documentation freshness checks. |

## Completion

This proposal is complete when every acceptance row has checked evidence,
`docs/specification/prelude-helpers.md` states the implemented byte-view alias,
and the generated standard-library documentation artifacts include it. Move the
completed record to `docs/reference/implemented-proposals/` and remove this
page from the active proposal catalog.
