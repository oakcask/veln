# Names And Effects

This file specifies implemented name resolution and effect checking.

## Name Resolution

Implemented checker namespaces are:

- module imports
- value declarations, including functions, parameters, and `let` bindings
- record fields inside one record literal

Bare names resolve to local bindings. Function calls resolve to:

- compiler-known stdio calls
- local bindings with function type
- discovered function signatures by final path segment
- compiler-known prelude helper calls

Unresolved values and call targets produce `name.unresolved` diagnostics.
Duplicate declarations in the same implemented namespace produce
`name.duplicate` diagnostics at the later declaration, with the first
declaration reported as related context.

Local value bindings shadow discovered function declarations for both bare
values and calls.

A wildcard let target, `_`, evaluates its expression without declaring a local
name. It can be annotated for type checking, but it is never a resolvable
binding.

Current duplicate checks reject:

- duplicate import aliases, where the alias is the final segment of the
  imported module path
- duplicate top-level function or test names
- duplicate parameter names in one function
- a result binding that duplicates a parameter name
- duplicate `let` names in the same function value scope, including names that
  duplicate parameters
- duplicate field names in one record literal
- duplicate pattern binding names in one match arm, including names that
  duplicate bindings already visible at the arm
- duplicate field names in one record pattern

Record type annotations also require unique field names. Duplicate record type
fields are reported through invalid type annotation diagnostics because they are
part of annotation parsing rather than value-name resolution.

Module boundary checks reject `use` declarations when the source file has no
`mod` declaration. The diagnostic is `module.missing_identity` at the first
`use` declaration and includes a repair hint in `related`.

When `veln.toml` contains a `[modules]` entry for a selected source file, the
entry is checked against that file's source `mod` declaration. The diagnostic
is `module.metadata_drift` at the manifest module name when the manifest tries
to supply a module name without a source owner or when the manifest name differs
from the source `mod` name. The source declaration is canonical and is reported
as related context when present.

Named holes remain repair labels, not value declarations. Reusing a hole label
does not affect name resolution.

## Stdio Calls

The implemented compiler-known stdio calls are:

```veln
stdio::print(text: String) -> () effects [stdio]
stdio::println(text: String) -> () effects [stdio]
stdio::eprint(text: String) -> () effects [stdio]
stdio::eprintln(text: String) -> () effects [stdio]
```

Direct calls to these functions infer the `stdio` effect. Function signatures
also carry effects inferred from their bodies, so a public function or test that
calls a private helper whose body reaches `stdio` must declare `stdio` even when
the helper omitted its own `effects` clause. Function-body effect inference
follows direct function calls until a fixed point. Calls through a local binding
with a function type infer the effects written in that function type.

A public function whose declared effects omit an inferred effect reports
`effect.missing_public` with related provenance pointing at bounded call sites.
Effect diagnostics include bounded structured provenance paths. Each path
records the boundary entry, the effect-causing call entry, whether the path set
was truncated, how many frames were hidden, and how many equivalent paths were
omitted. For the current direct-call, signature-based, and body-inferred helper
inference, hidden frame counts are zero.

## Prelude Helpers

The implemented compiler-known prelude helpers are ordinary bare function calls
with prefix names. They are pure and do not infer effects.

```veln
list_len(items: List(A)) -> Int
list_is_empty(items: List(A)) -> Bool
list_push(items: List(A), value: A) -> List(A)
list_concat(left: List(A), right: List(A)) -> List(A)
list_map(items: List(A), f: fn(A) -> B) -> List(B)
list_filter(items: List(A), f: fn(A) -> Bool) -> List(A)
list_fold(items: List(A), initial: B, f: fn(B, A) -> B) -> B
list_try_map(items: List(A), f: fn(A) -> Result(B, E)) -> Result(List(B), E)
dict_get(dict: Dict(K, V), key: K) -> Option(V)
dict_contains(dict: Dict(K, V), key: K) -> Bool
dict_insert(dict: Dict(K, V), key: K, value: V) -> Dict(K, V)
dict_remove(dict: Dict(K, V), key: K) -> Dict(K, V)
option_map(value: Option(A), f: fn(A) -> B) -> Option(B)
option_and_then(value: Option(A), f: fn(A) -> Option(B)) -> Option(B)
option_unwrap_or(value: Option(A), fallback: A) -> A
result_map(value: Result(A, E), f: fn(A) -> B) -> Result(B, E)
result_map_err(value: Result(A, E), f: fn(E) -> F) -> Result(A, F)
result_and_then(value: Result(A, E), f: fn(A) -> Result(B, E)) -> Result(B, E)
```

Container update helpers return new frozen values and do not mutate their input
containers in place. `list_try_map` evaluates items in source order, stops at
the first `Err`, and otherwise returns `Ok` containing the mapped frozen list in
source order. `list_map`, `list_filter`, and `list_fold` also visit list items
in source order.

The language specification does not promise asymptotic complexity, allocation
counts, representation identity, structural sharing, hashing, or tree-balancing
behavior for these helpers. Those are implementation details until a concrete
container representation is specified. Tests should assert value semantics,
source-order traversal, `Result` short-circuiting, diagnostics, and effect
behavior rather than timings or allocation counts.
