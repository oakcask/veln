---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Declared Helper Callback Alias


This record keeps the completed public member alias boundary for declared
helper callback argument inference after the behavior moved into the
specification and executable examples. It is historical evidence, not the
source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Successful public alias coverage:
  `../../../examples/specification/check/declared-helper-callback-alias-inference/`.
- Unsupported public alias coverage:
  `../../../examples/specification/check/declared-helper-callback-alias-inference-unsupported/`.

## Implemented Boundary

A visible public function alias carries the concrete function signature of its
resolved target. When a call reaches such an alias and the aliased helper has
a concrete function-typed parameter, the checker uses that parameter type as
the expected type for a named same-module private callback passed at the
matching argument position.

The rule covers fixed callback parameter lists through the alias boundary.
The callback return still has to satisfy the aliased helper's declared
function return type.

## Boundaries Preserved

- Public function signatures remain explicit.
- Exported aliases are not inferred; they re-export an already resolved
  target signature.
- Helper function parameter types that still contain `unknown` do not
  constrain callback parameters, even when reached through a public alias.
- This slice does not add alias-chain callback inference beyond the existing
  public member alias resolution rules.

## Completion Evidence

- Executable specification examples cover successful callback parameter
  inference through an imported public function alias.
- Negative executable examples cover an imported public function alias whose
  aliased helper has a function parameter containing `unknown`.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this public alias callback inference
  boundary is no longer listed as future proposal work.
