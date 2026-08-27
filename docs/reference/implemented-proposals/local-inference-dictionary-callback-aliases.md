---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Dictionary Callback Aliases


This record keeps the completed dictionary callback helper alias slice for the
local inference proposal. Current behavior is specified in
`../../specification/types.md#read-first`,
`../../specification/types.md#inference`, and executable examples under
`../../../examples/specification/check/`.

## Completed Behavior

The source-visible aliases `dict_map_with`, `dict_filter_with`,
`dict_fold_with`, and `dict_try_map_with` use the same compiler-known prelude
callback signature path as the existing dictionary helpers. Each alias accepts
an explicit context argument before the dictionary and passes that context as
the first callback argument.

For concrete `Dict<K, V>` inputs, the aliases infer named private callback
parameters that receive keys and values as `K` and `V`. `dict_fold_with` also
infers its accumulator parameter from the fold result context, and
`dict_try_map_with` uses an expected `Result<Dict<K, A>, E>` result to
constrain callback returns as `Result<A, E>` when available.

## Evidence

- Successful executable specification coverage:
  `../../../examples/specification/check/prelude-dictionary-callback-alias-inference/`.
- Focused diagnostic executable specification coverage:
  `../../../examples/specification/check/prelude-dictionary-callback-alias-inference-diagnostics/`.
- Semantic unit coverage checks the lowered callback parameter types for all
  four aliases.

## Boundaries

This slice does not add anonymous callback literal syntax, infer ordinary
user-defined higher-order helpers, or infer public, exported, or imported
function signatures.
