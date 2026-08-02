# One-Shot Resumable Effect Handlers

Status: rejected

## Decision

Veln does not add explicit one-shot resumptions without a checked example that
requires handler-controlled suspension and continuation disposition. No current
HTTP/2, scheduling, streaming, cancellation, or cleanup example meets that
gate.

The implemented automatic lexical handler boundary remains current. Recoverable
failure and cancellation remain ordinary values. Services and adapters retain
explicit ownership of streams, tasks, and cleanup state.

## Rationale And Reconsideration

The durable decision, supporting evidence, and the gate for a new proposal are
recorded in
[One-Shot Resumable Handler Boundary](../reference/source-decisions/records/result-one-shot-resumable-handler-boundary.md).

This path remains only as a stable route for old proposal links. It does not
authorize implementation work.
