---
role: proposal
update-when: The proposed generic trace::in_span and traced::spawn signatures, generic-function dependency, result preservation, or cleanup acceptance changes.
---

# Preserve Callback Results Through Tracing

Make tracing wrappers preserve the callback's result type and effect row.
This is a Ready design with an explicit implementation dependency on
[Generic Functions](generic-functions.md). It must not be implemented by
specializing source functions at observed call sites.

## Current boundary and dependency

[trace::in_span](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/trace.veln) and
[traced::spawn](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/traced.veln) already quantify an
effect row, but fix the callback result to `Result<(), String>`. A callback
returning a value or a domain-specific error therefore needs an adapter merely
to add tracing. Their bodies forward results without inspecting them.

The generic-functions proposal distinguishes implemented effect polymorphism
from incomplete ordinary function type schemes. That is a concrete language
limitation relevant to generalizing these wrappers, not evidence of poor
historical design. The inspected source does not establish the original
author's intent. Implement this proposal once ordinary type binders work
through bodies, imported callbacks, artifacts, and JVM emission.

## Proposed signatures

Illustrative future source signatures, not currently executable evidence:

```veln
trace::in_span<T, effect E>(request: SpanRequest, body: fn(Span) -> T effects [...E]) -> T effects [observe::Observe, ...E]
traced::spawn<T, effect E>(context: TraceContext, callback: fn(TraceContext) -> T effects [...E]) -> Task<T> effects [concurrency, ...E]
```

`T` may itself be `Result<Value, DomainError>`, a scalar, a record, an ADT,
or unit. There is no additional result wrapper or conversion to `String`.
`traced::spawn` still returns a task; `task::join` retains its separate task
failure boundary. Tracing does not merge a callback's domain error with task
cancellation or cleanup failure.

On ordinary execution, the wrapper invokes the callback exactly once. Task
cancellation before callback entry may prevent invocation. For an established
span scope, `in_span` finishes its span exactly once after normal return,
ordinary error propagation, or abrupt unwinding under the existing cleanup
rules. `traced::spawn` propagates context without creating another span itself. If cleanup itself fails, the existing
cleanup precedence remains authoritative; preservation of a callback result
is conditional on successful cleanup. Context propagation and handler
inheritance retain their existing behavior.

## Related work and alternatives

The inspected [tracing 0.1.44 Span::in_scope](https://docs.rs/tracing/0.1.44/tracing/struct.Span.html#method.in_scope)
returns the callback's arbitrary `T`.
[Rust thread::spawn](https://doc.rust-lang.org/std/thread/fn.spawn.html)
also transports an arbitrary return value in its join handle. These signatures
support result-preserving wrappers, not Rust lifetime or thread rules for Veln.
Veln retains its explicit context, effect, task, and cleanup contracts.

Adding separate wrappers for every payload or error type would repeat a
compiler limitation in the API. Restricting the redesign to
`Result<T, Error>` would still exclude callbacks that do not fail. One result
parameter covers both cases with the least public surface.

## Planned acceptance

| Case | Required observation | Intended verification |
| --- | --- | --- |
| Scalar, record, and ADT returns | The wrapper returns the exact callback value. | Stdlib trace/traced companions |
| `Result<Value, DomainError>` success and failure | Payload and typed error survive without conversion. | Stdlib companions with independent expected values |
| Unit/String existing callers | The same explicit result remains representable. | Migrated companions |
| Multiple instantiations and imported aliases | Different result types do not constrain each other. | Generic checker and artifact tests |
| Pure and effectful callbacks | Effects are preserved without adding unrelated effects. | Checker tests and stdlib behavior |
| Normal, propagated-error, and cancellation exits | An established in_span scope has one finish after one start; cancellation before callback entry need not run the callback; traced::spawn adds no span. Cleanup precedence and task failures are retained. | Scoped Veln observation support; cleanup/task integration |

No acceptance row is implemented by this document. Use separate scoped Veln
handlers for deterministic observation evidence. API behavior belongs in
[stdlib companions](../reference/toolchain-test-harness.md#test-placement-policy).
Use guarded checker/backend test runners when implementing the generic boundary.

## Migration and source form

Replace the existing signatures and remove now-unnecessary result-to-string
adapters at callers. No additional wrapper family or compatibility alias is
needed. The declarations use the generic-functions canonical binder form;
ordinary formatting does not add type parameters or rewrite callers. Existing
calls with inferable `T` retain their spelling. Formatter convergence and
binder-layout rules remain owned by the generic-functions proposal and the
[source surface](../specification/source-surface.md).

The [observability specification](../specification/standard-library-observability.md)
continues to describe current fixed-result behavior until implementation and
its executable evidence land. Promote the new signatures and lifecycle
examples there, then remove this proposal and its catalog entry.
