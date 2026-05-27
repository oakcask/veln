# Discussion Result: Prelude Complexity Guarantees

Status: proposed

## Picked Question

- What complexity guarantees, if any, should first-slice prelude helpers make
  before Veln has chosen concrete persistent vec and dictionary
  representations?

## Decision

The first slice should not promise asymptotic complexity guarantees for prelude
container helpers. The normative contract is value semantics, input
preservation, source-order traversal, `Result` short-circuiting for
`vec_try_map`, and deterministic diagnostics. Complexity claims remain
implementation notes until the language chooses and documents concrete
container representations.

In particular, the language spec should not say that `vec_push`,
`vec_concat`, `dict_insert`, or `dict_remove` are `O(1)`, `O(log n)`, or
amortized `O(1)`. It should also avoid promising persistent vector,
copy-on-write array, balanced-tree, or hash-array mapped trie behavior through
the prelude API. A later representation decision may add documented
performance classes, but that decision must name the representation family,
sharing assumptions, and any adversarial-key or memory-retention caveats.

The first implementation may still choose reasonable internal data structures.
Those choices are optimization freedom, not source compatibility. Golden tests
should assert observable behavior and diagnostics, not helper timings or
allocation counts.

## Rationale

Okasaki's work on persistent data structures supports Veln's user-facing
semantic model: updates return new values while old versions remain usable.
That research also shows why representation matters. The same surface operation
can have different costs depending on whether it is backed by a simple list,
tree, vector-like structure, copy-on-write storage, or another persistent
representation. Promising costs before choosing that representation would turn
an implementation shortcut into a language contract.

The existing prelude decision deliberately uses ordinary prefix-named helpers
instead of method dispatch, traits, or type classes. McBride and Paterson's
applicative traversal work supports the semantic shape of
`vec_try_map`: traverse in order while sequencing a surrounding effect. Rust's
`Result` collection behavior gives the concrete short-circuiting precedent.
These sources justify specifying evaluation order and error propagation for
repairability, but they do not require Veln to expose container complexity in
the first slice.

This distinction helps agents. An agent needs to know that
`dict_insert(existing, key, value)` does not mutate `existing`, that
`vec_try_map` preserves element order, and that the first `Err` is returned
unchanged. Those facts affect correctness and repair search. The agent usually
does not need an early spec-level promise that appending is amortized constant
time. If performance becomes part of a failing requirement, that should be
handled by explicit performance tests or a later representation-backed
reference document.

## First-Slice Rules

- Prelude helper specifications should state value semantics and observable
  evaluation behavior, not asymptotic complexity.
- `vec_map`, `vec_filter`, `vec_fold`, and `vec_try_map` evaluate vec
  elements in source order.
- `vec_try_map` stops at the first `Err`, returns that error unchanged, and
  does not evaluate later elements.
- `vec_push`, `vec_concat`, `dict_insert`, and `dict_remove` return new
  values and leave their inputs semantically unchanged.
- The spec should not require an allocation strategy, structural sharing
  strategy, hashing strategy, or tree balancing strategy for first-slice
  helpers.
- Human-facing documentation may describe current implementation performance
  only as non-normative implementation notes.
- Golden tests should not assert timing, allocation counts, or representation
  identity for prelude helpers.

## Open Details

The concrete container representation remains open. Once examples and runtime
targets show real pressure, Veln should make a separate representation
decision that can responsibly document performance classes.

The first slice also does not define performance diagnostics. If future agents
need repair loops for accidental quadratic behavior, that belongs in a later
lint or profiling decision rather than in the initial prelude helper contract.

## Consequence

The first implementation can choose simple, debuggable containers without
locking the language into premature performance promises. The source contract
still gives agents the facts needed for correctness repairs: immutability,
order, and `Result` propagation.

## References

- `okasaki1998-persistence`
- `mcbride2008-applicative-programming`
- `rust-std-result-fromiterator`
