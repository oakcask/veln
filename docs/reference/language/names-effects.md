# Names And Effects

Status: implemented
Date: 2026-05-24

This file specifies implemented name resolution and effect checking.

## Name Resolution

Bare names resolve to local bindings. Calls resolve to:

- compiler-known stdio calls
- discovered function signatures by final path segment
- local bindings with function type

Unresolved values and call targets produce `name.unresolved` diagnostics.

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
