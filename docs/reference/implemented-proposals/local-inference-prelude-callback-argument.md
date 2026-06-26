# Local Inference Prelude Callback Argument

Status: implemented

This record keeps the completed compiler-known prelude callback argument
inference slice after the behavior moved into the specification and executable
examples. It is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Checked example coverage:
  `../../../examples/specification/check/prelude-callback-argument-inference/`.
- Focused callback diagnostic coverage:
  `../../../examples/specification/check/prelude-callback-argument-inference-diagnostics/`
  and `../../../examples/specification/check/prelude-helper-diagnostics/`.

## Implemented Boundary

Compiler-known collection, option, and result prelude helpers push concrete
input item, success, or error types into named private callback function
parameters. The implemented helper set is `vec_map`, `vec_filter`, `vec_fold`,
`vec_try_map`, `list_map`, `list_filter`, `list_fold`, `list_try_map`,
`option_map`, `option_and_then`, `result_map`, `result_map_err`, and
`result_and_then`.

The rule is local and monomorphic. It applies to named private callback
function values used by the compiler-known helper signature path. It does not
infer public signatures, imported public signatures, anonymous callback literal
syntax, ordinary user-defined higher-order helpers, or helper aliases that are
not in the compiler-known prelude signature path.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Existing helper result-context inference for empty callback returns remains
  part of the same prelude helper path.
- Result-returning callbacks passed to `vec_map` still report the ordinary
  callback type mismatch with a repair hint toward `vec_try_map`.

## Completion Evidence

- Executable specification examples cover successful named private callback
  parameter inference from compiler-known collection, option, and result helper
  input types.
- Negative executable examples cover incompatible callback facts with the
  focused `type.mismatch` diagnostic.
- Existing helper diagnostic examples preserve the `vec_try_map` repair hint
  for a `Result`-returning callback passed to `vec_map`.
- Semantic tests cover the helper set, including fold callbacks, try-map
  callbacks, result success and error callbacks, and qualified `prelude::`
  helper calls.
- The proposal page keeps only remaining callback work outside this
  compiler-known helper path.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this callback argument inference slice
  is no longer listed as future proposal work.
