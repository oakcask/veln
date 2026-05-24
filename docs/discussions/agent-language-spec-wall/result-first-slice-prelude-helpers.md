# Discussion Result: First-Slice Prelude Helpers

## Picked Question

- Which minimum prelude helpers are part of the first examples and tests,
  especially functional container updates and fallible traversal helpers such
  as `try_map`?

## Decision

The first examples and golden tests may rely on a small, explicitly named
prelude for immutable containers, `Option`, and `Result`.

Use prefix-style function names instead of methods or type-class-style generic
names. The first-slice surface already chose plain function calls over method
calls, and the type system has built-in parametric containers but no user
generics, traits, or type classes. A name such as `list_try_map` is therefore
clearer than a free `try_map` whose dispatch rules would be invisible.

Required first-slice helpers:

- Lists: `list_len`, `list_is_empty`, `list_push`, `list_concat`,
  `list_map`, `list_filter`, `list_fold`, and `list_try_map`.
- Dictionaries: `dict_get`, `dict_contains`, `dict_insert`, and
  `dict_remove`.
- `Option`: `option_map`, `option_and_then`, and `option_unwrap_or`.
- `Result`: `result_map`, `result_map_err`, and `result_and_then`.

`list_try_map(items, f)` has this semantic shape:

```text
list_try_map(List<A>, fn(A) -> Result<B, E>) -> Result<List<B>, E>
```

It evaluates items in source order, stops at the first `Err`, returns that
error unchanged, and otherwise returns `Ok` with the mapped list in the same
order. It is the standard first-slice way to apply `?`-producing work across a
list. `?` still propagates only inside the result-returning function that calls
`list_try_map`; it does not gain special pipeline behavior.

## Rationale

The mutability decision says that container updates return new values rather
than modifying old ones. Okasaki's account of persistence gives the same
conceptual model: old and new versions can coexist, with unaffected structure
shareable by the implementation. Veln's prelude names should therefore make
updates read as value production. `dict_insert(existing, key, value)` and
`list_push(existing, value)` are acceptable even if an implementation later
uses structural sharing, copy-on-write, or another representation strategy.

Haskell is a useful precedent for a small functional vocabulary. Its Standard
Prelude exports `Maybe`, `Either`, `mapM`, `sequence`, `maybe`, and `either`,
while `Data.List` supplies ordinary list transformations and folds. Veln should
not copy Haskell's type-class surface into the first slice, but it should copy
the idea that examples need a compact, boring vocabulary for map/filter/fold
work before introducing a larger library.

Effectful traversal is the main missing piece for agent repair loops. McBride
and Paterson describe applicative programming as a general abstraction for
effectful application and traversal. That is too abstract for the first Veln
slice, but it supports the shape of the operation: traverse a container while
accumulating the surrounding effect. Since Veln's first explicit effect carrier
for recoverable failure is `Result`, the concrete helper should be
`list_try_map`, not a generic `traverse`.

Rust gives a pragmatic short-circuiting precedent. Its iterator APIs separate
ordinary `map` from fallible iteration such as `try_fold`, and collecting an
iterator of `Result` values stops at the first error. Veln should make that
behavior direct and diagnostic-friendly: when an agent writes `list_map` with a
function returning `Result`, `veln check` can suggest `list_try_map` and show
the expected callback type.

## First-Slice Rules

- Prelude helper names are ordinary functions in the prelude namespace.
- First examples and golden tests should use only the helpers listed in this
  result unless a later decision expands the prelude.
- `list_push` returns a new list with the value appended at the end.
- `list_concat` returns a new list containing the left list followed by the
  right list.
- `dict_insert` and `dict_remove` return new dictionaries; they do not mutate
  the input dictionary.
- `dict_get` returns `Option<V>`.
- `list_map` accepts a pure value-returning callback. If the callback returns
  `Result`, diagnostics should suggest `list_try_map`.
- `list_try_map` is the only first-slice fallible traversal helper required by
  examples and tests.
- Pipelines may use these helpers, but the pipeline operator has no special
  fallible semantics.
- Record update syntax remains outside this decision. First-slice examples
  should rebuild small records explicitly or move state that needs keyed
  updates into dictionaries.

## Open Details

The first implementation can choose whether these helpers are compiler-known
built-ins, ordinary generated prelude functions, or host-runtime intrinsics.
The observable contract is the function name, type shape, value semantics, and
diagnostic behavior.

The exact complexity guarantees remain open. The implementation should not
promise persistent vector or hash-array mapped trie performance in the language
spec until container representation is chosen.

Names for indexed operations such as `list_at`, `list_take`, `list_drop`,
`dict_keys`, and `dict_values` are intentionally deferred. They are common, but
they are not needed to settle the first examples around immutable updates,
fallible traversal, and pipeline style.

## References

- `okasaki1998-persistence`
- `mcbride2008-applicative-programming`
- `haskell-2010-report`
- `rust-std-iterator`
- `rust-std-result-fromiterator`

## Consequence

The first Veln examples get a stable prelude small enough for agents to
memorize and diagnostics to target. Immutable update examples no longer need
ad hoc helper names, and fallible collection code has one canonical shape:
`list_try_map` over a callback returning `Result`.
