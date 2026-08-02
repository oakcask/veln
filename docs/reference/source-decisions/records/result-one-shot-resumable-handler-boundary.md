# Discussion Result: One-Shot Resumable Handler Boundary

Status: implemented

## Picked Question

Should Veln extend automatic lexical operation handlers with explicit deep,
one-shot resumptions?

## Decision

Retain automatic lexical operation handlers. Do not add explicit continuation,
`resume`, or `discontinue` syntax without a new accepted proposal that meets
the reconsideration gate below.

Represent recoverable failure, deadline expiry, and cancellation as ordinary
values. Keep stream, task, and cleanup ownership in the service or adapter that
acquires those resources. Keep runtime failures that cannot return a source
value as abrupt termination without a source cleanup guarantee.

## Evidence

The current HTTP/2 application callback returns an ordinary `Result` and an
immutable action list. The current client service uses `Result`, immutable
connection state, task join outcomes, and explicit stream closure. The
application boundary and client service therefore do not require a handler to
retain control of a suspended caller.

The current transport and adapter APIs already model clean end, deadline
expiry, and cancellation with `Option`, `Result`, or outcome ADTs where the
operation can return a value. Existing executable specification cases cover
automatic handler repetition, early return from a handled expression, task
handler isolation, HTTP/2 callback and join failures, client connection reuse,
and cancellable stream outcomes.

No executable evidence is added for this decision because it adds no language
or runtime behavior. Current behavior remains checked by the lexical-handler,
HTTP/2 service, transport, and cancellable-adapter cases under
`examples/specification/`.

## Rationale

The rejected design required static continuation linearity, continuation escape
checking, new source syntax, typed-IR representation, JVM lowering, and runtime
failure behavior. No checked example showed that these mechanisms removed an
ownership defect or a repeated control pattern.

An asynchronous scheduler usually needs to store a suspended continuation or
transfer it to another task. The rejected design prohibited both operations.
It therefore did not provide a sufficient scheduling abstraction despite its
implementation cost.

Typed outcomes and explicit ownership address the observed transport and
cleanup boundaries directly. If cleanup logic becomes materially duplicated,
the project can first add a focused scoped resource or structured task API.
That API does not require source-visible continuations when a callback and a
typed outcome preserve the ownership boundary.

## Reconsideration Gate

A new resumable-handler proposal must include one checked or reviewable example
that demonstrates every row below.

| Required fact | Observable evidence |
| --- | --- |
| Handler-controlled suspension | An operation suspends its caller, and the handler must retain control before the caller may continue. |
| Automatic continuation is insufficient | Returning the operation result immediately produces the wrong externally visible outcome. |
| Value and state alternatives are insufficient | A `Result`, an ADT decision, explicit state, or a lexical provider boundary obscures resource ownership or duplicates one reusable control pattern. |
| Complete disposition | Success and failure select exactly one resume or typed discontinuation. Cancellation and handler exit also define exactly one disposition. |
| Confinement is sufficient | The example works without returning, storing, or transferring a continuation, or the new proposal explicitly revises that restriction. |

The example must exist before grammar, checker, backend, or runtime acceptance
cases are added. A synchronous byte-stream wrapper does not meet the gate.

## Consequence

The current specification continues to expose automatic deep lexical handlers
without source-visible continuations. Future transport and cleanup work should
prefer typed outcomes and explicit ownership. A future proposal may revisit
resumptions only from new evidence; this rejected proposal is not an
implementation backlog.
