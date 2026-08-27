---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference And Annotation Elision


## Outcome

Veln performs local, monomorphic expected-type propagation across the selected
local binding, initializer, constructor, pattern, callback, return, match-arm,
if-branch, collection, dictionary, and compiler-known or concretely declared
helper paths. Public signatures remain explicit and unconstrained or ambiguous
local facts still require annotations.

Current behavior and diagnostic boundaries are specified under
`../../specification/types.md` and `../../specification/diagnostics-json.md`.

## Evidence

Executable evidence lives in the inference cases under
`../../../examples/specification/check/`. The focused local-inference records
in this directory retain completion evidence for the individual paths,
examples cleanup, and diagnostic details.

## Boundary

This work does not add generalized let-polymorphism, infer exported
signatures, or infer through an `unknown` helper signature. A new expected-type
source must be proposed as a distinct capability; another effect combination
or same-shaped callback path is not an automatic continuation of this work.
