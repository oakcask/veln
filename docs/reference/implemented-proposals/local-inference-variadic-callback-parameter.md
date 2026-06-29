# Local Inference Variadic Callback Parameter

Status: implemented

This record keeps the completed variadic declared-helper callback parameter
inference slice after the behavior moved into the specification and executable
examples. It is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Successful same-module coverage:
  `declared-helper-variadic-callback-inference`.
- Successful imported-helper coverage:
  `declared-helper-variadic-callback-import-inference`.
- Diagnostic and unsupported-boundary coverage:
  `declared-helper-variadic-callback-inference-diagnostics` and
  `declared-helper-variadic-callback-inference-unsupported`.

## Implemented Boundary

Same-module helpers and visible imported public helpers whose declared
callback parameter type is a concrete variadic function type can provide
expected type context for a named private callback function value passed at
that argument position. The callback receives the declared fixed parameter
types and the declared variadic element type for omitted callback parameter
annotations.

The callback remains monomorphic. Incompatible body facts report the ordinary
`type.mismatch` diagnostic at the failed fact, and effect compatibility follows
the existing declared-helper callback inference rule.

## Boundaries Preserved

- Public function signatures remain explicit.
- Test declaration signatures remain explicit.
- Exported aliases and imported public function signatures are not inferred.
- Helper signatures whose function parameter type still contains `unknown`,
  including an unknown variadic element type, do not constrain callback
  parameters.
- This slice does not add anonymous callback literal syntax or a generic
  function system.

## Completion Evidence

- Executable specification examples cover successful same-module and imported
  declared helper variadic callback inference.
- Negative executable examples cover JSON and human diagnostics for
  incompatible variadic callback body facts and helper signatures whose
  variadic element type is unknown.
- Semantic tests cover parsing unknown variadic function type elements and
  preserving the unknown boundary.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this callback parameter inference
  slice is no longer listed as future proposal work.
