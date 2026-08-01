# One-Shot Resumable Effect Handlers

Status: proposed

## Summary

Extend lexical operation handlers with explicit deep, one-shot resumptions.
This proposal is a conditional follow-up. It must not be selected merely to
make the first handler implementation more general.

## Activation Gate

Implementation may start only when a checked or reviewable HTTP/2, scheduling,
streaming, or cleanup example demonstrates all of these facts:

- The computation must suspend at an operation and transfer control to its
  handler.
- Returning one operation result and automatically continuing cannot express
  the required outcome.
- Rewriting the example as ordinary `Result`, ADT decisions, explicit state,
  or the lexical provider boundary would obscure ownership or duplicate a
  reusable control pattern.
- The example identifies what happens to the suspended computation on success,
  failure, cancellation, and handler exit.

Until that gate is met, [Lexical Operation Handlers](../reference/implemented-proposals/lexical-operation-handlers.md)
remains the selected language scope.

## Proposed Semantics

A resumable handler receives a continuation for the suspended computation.
The continuation accepts the declared operation result. It produces the
handled expression result.

- A continuation is one-shot.
- A handler must resume or discontinue each captured continuation exactly
  once before its clause completes.
- A second resume or discontinue is rejected statically when local checking
  proves it. Otherwise it fails at the continuation operation boundary.
- A deep resume reinstalls the same handler around the resumed computation.
- An operation that a resumable clause explicitly forwards continues to the
  next enclosing handler for that nominal effect.
- A continuation cannot escape the handler clause, be stored in an aggregate,
  be returned, or be sent to another task.
- Discontinuation supplies a typed failure compatible with the handled
  expression's declared outcome. It does not create an untyped exception
  channel.
- Host effects performed by the clause remain in the outer effect set.

The final surface spelling is intentionally gated on an accepted fixture. The
grammar must still expose separate `resume` and `discontinue` operations and
must distinguish an automatically resuming handler from a resumable handler.
The parser proposal that activates this work must choose one canonical
spelling and add accepted and rejected fixtures before backend lowering.

## Continuation State Model

| Current state | Event | Next state | Observable result |
| --- | --- | --- | --- |
| Captured | Resume with valid operation result | Consumed | Suspended computation continues under the same handler |
| Captured | Discontinue with compatible failure | Consumed | Suspended computation terminates through the typed handled outcome |
| Captured | Clause returns without either action | Invalid | Handler diagnostic or runtime continuation failure at the clause boundary |
| Consumed | Resume | Invalid | No second execution of the suspended computation |
| Consumed | Discontinue | Invalid | No second termination of the suspended computation |
| Captured | Store, return, or task transfer | Invalid | Static escape diagnostic at the attempted transfer |

This state table is authoritative. Backend stack shape, exception tables, and
continuation object layout are not language behavior.

## Effect Semantics

Resumption does not add a new public effect label. The handled effect is
removed and handler-clause effects are added by the same rule as an automatic
lexical handler. An operation forwarded to an outer handler remains in the
inner expression until the outer handler removes it.

Effect-row polymorphism is not part of this proposal. A resumable handler uses
concrete nominal and host effect sets.

## Acceptance Model

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| One resume | The suspended computation receives the supplied value and continues once | `run/one-shot-handler-resume` |
| Deep repeated operation | A resumed computation selects the same handler for its next operation | `run/one-shot-handler-deep-resume` |
| Discontinue | The computation produces the declared typed failure and does not continue after the operation | `run/one-shot-handler-discontinue` |
| Missing disposition | A clause that neither resumes nor discontinues is rejected when statically visible | `check/one-shot-handler-linearity` |
| Double resume | The second use is rejected or produces the dedicated continuation failure before user code runs twice | `run/one-shot-handler-double-resume-json` |
| Escaping continuation | Return, aggregate storage, and task transfer are rejected | `check/one-shot-handler-escape` |
| Forwarded operation | The next enclosing matching handler receives the operation | `run/one-shot-handler-forwarding` |
| Host effect replacement | Clause host effects remain visible after the nominal effect is handled | `check/one-shot-handler-effect-replacement` |

The relative paths are planned directories below `examples/specification/`.
The activation example must be recorded in this proposal before any of these
implementation cases are added.

## Non-Goals

- Do not add multi-shot continuations.
- Do not add shallow handlers.
- Do not permit continuation cloning.
- Do not permit continuation transfer between tasks.
- Do not use resumptions solely to wrap synchronous byte-stream reads and
  writes that automatic lexical handlers already express.
- Do not prescribe a CPS transform, segmented stack, JVM exception strategy,
  or runtime object representation.

## Completion Boundary

This proposal is complete only after the activation evidence is recorded, the
one-shot state table is checked on the reference JVM backend, escape behavior
is checked statically, and the implemented rules are promoted to the language
specification. Meeting the activation gate does not require the HTTP/2 service
proposal to adopt explicit resumptions.
