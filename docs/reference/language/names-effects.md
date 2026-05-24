# Names And Effects

This file specifies implemented name resolution and effect checking.

## Name Resolution

Implemented checker namespaces are:

- module imports
- value declarations, including functions, parameters, and `let` bindings
- record fields inside one record literal

Bare names resolve to local bindings. Function calls resolve to:

- compiler-known stdio calls
- discovered function signatures by final path segment
- local bindings with function type
- compiler-known prelude helper calls

Unresolved values and call targets produce `name.unresolved` diagnostics.
Duplicate declarations in the same implemented namespace produce
`name.duplicate` diagnostics at the later declaration, with the first
declaration reported as related context.

Current duplicate checks reject:

- duplicate import aliases, where the alias is the final segment of the
  imported module path
- duplicate top-level function or test names
- duplicate parameter names in one function
- duplicate `let` names in the same function value scope, including names that
  duplicate parameters
- duplicate field names in one record literal

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

Direct calls to these functions infer the `stdio` effect. A public function
whose declared effects omit an inferred effect reports `effect.missing_public`
with related provenance pointing at bounded call sites.

Transitive effect inference through helper functions is limited to discovered
function signatures. Rich transitive provenance fields remain future work.

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

Container update helpers return new values. `list_try_map` evaluates items in
source order, stops at the first `Err`, and otherwise returns `Ok` containing
the mapped list in source order.
