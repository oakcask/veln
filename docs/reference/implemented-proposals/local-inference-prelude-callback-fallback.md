---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Prelude Callback Fallback


This record keeps the completed source-backed prelude callback fallback slice
after the behavior moved into the specification. It is historical evidence,
not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Compiler-known prelude callback history:
  [local-inference-prelude-callback-argument.md](local-inference-prelude-callback-argument.md).
- Declared helper callback history:
  [local-inference-declared-helper-callback-argument.md](local-inference-declared-helper-callback-argument.md).

## Implemented Boundary

Bare and `prelude::` calls to source-backed prelude helpers without a
compiler-known callback rule can use the helper's embedded source declaration
as a declared-helper fallback. When a selected helper parameter is already a
concrete function type, a named private callback passed at that argument
position receives the declared function parameter types for omitted callback
parameter annotations.

The fallback does not instantiate generic source helper signatures. If the
function-typed helper parameter still contains `unknown`, it does not
constrain private callback parameters and the existing private inference
diagnostic behavior remains.

## Completion Evidence

- `crates/veln-sema/src/prelude.rs` tests cover a future source-backed prelude
  helper signature with a concrete callback parameter.
- The same tests cover the negative case where a generic callback parameter is
  normalized to `unknown` and does not enter the fallback.
- Existing checked examples under
  `../../../examples/specification/check/prelude-callback-argument-inference/`
  continue to cover the current compiler-known prelude callback helper set.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why this callback fallback slice is no
  longer listed as future proposal work.
