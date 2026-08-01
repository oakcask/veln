# Lexical Operation Handlers

Status: proposed

## Summary

Add nominal operation effects and lexical handlers that supply an
implementation for one effect within one expression. An operation call
suspends at the lexical boundary only long enough for the selected provider
function to return its result. Evaluation then resumes automatically.

This proposal does not expose continuations. It tests whether Veln can make
effectful standard-library code independent from a concrete host capability
without adding general control effects.

## Current Boundary

Current `effects [...]` declarations contain a fixed set of coarse host labels.
The checker infers those labels from compiler-known calls and requires public
boundaries to cover them. Current source cannot declare an effect operation,
perform one, or eliminate an effect with a handler.

The existing labels such as `net`, `time`, and `concurrency` remain host
effects. This proposal does not turn them into interceptable operations.

## Proposed Source Surface

The following sketch is the proposed canonical shape. The executable grammar
and accepted/rejected fixtures are authoritative once implemented.

```veln
pub effect DuplexStream
	read_chunk() -> Option<ByteChunk>
	write_chunks(chunks: List<ByteChunk>) -> ()
end
```

An effect name is nominal and module-owned. Imported effect names may appear as
qualified paths in function and function-type effect sets.

```veln
pub fn drive() -> Result<(), String> effects [transport::DuplexStream]
	match perform transport::DuplexStream::read_chunk()
		Some(bytes) => consume(bytes)
		None => Ok(())
	end
end
```

A handler declaration binds an explicit context and maps every operation to a
named provider function. A provider receives the handler context before the
operation arguments.

```veln
pub handler net_stream(stream: NetStream) handles transport::DuplexStream effects [net]
	read_chunk = read_net_chunk
	write_chunks = write_net_chunks
end
```

```veln
fn read_net_chunk(stream: NetStream) -> Option<ByteChunk> effects [net]
	net::read_chunk_or_end(stream)
end

fn write_net_chunks(stream: NetStream, chunks: List<ByteChunk>) -> () effects [net]
	net::write_chunks(stream, chunks)
end
```

The `handle` expression selects the handler for the dynamic evaluation of its
body while keeping the selection lexically visible.

```veln
handle drive() with net_stream(stream)
```

The formatter uses this single spelling. A handler is not a first-class value
in this proposal. Source cannot store, return, or select a handler through an
ordinary function value.

## Static Semantics

- `perform E::operation(arguments)` contributes nominal effect `E`.
- The operation arguments and result use the types declared by `E`.
- A handler must provide exactly one provider for every operation of its
  handled effect.
- Each provider must accept the handler context parameters followed by the
  declared operation parameters. It must return the declared operation result.
- A public handler must declare the union of the effects used by its provider
  functions. A private handler may use the existing private inference rule.
- A provider must not perform the effect implemented by its own handler.
- A `handle body with provider` expression removes the handled effect from the
  body's effect set and adds the handler's effect set.
- An inner handler for the same nominal effect shadows an outer handler while
  its body is evaluated.
- A user-defined effect may remain in an exported function type. A runnable
  entry point must not retain a user-defined effect after all surrounding
  handlers have been checked.
- Existing host effects remain valid at runnable entry points under their
  current runtime boundaries.
- Contracts cannot perform operations or install handlers. They retain their
  current effect-free call rule.

The effect-set transformation is authoritative for checker acceptance:

| Body effects | Handled effect | Handler effects | Expression effects |
| --- | --- | --- | --- |
| `[E]` | `E` | `[]` | `[]` |
| `[E, net]` | `E` | `[]` | `[net]` |
| `[E]` | `E` | `[net]` | `[net]` |
| `[E, time]` | `E` | `[net]` | `[net, time]` |
| `[time]` | `E` | `[net]` | `[net, time]` |

Effect sets remain unordered and duplicate-free. The final row is
conservative: installing a handler contributes its declared effects even when
the handled operation is not reached in that execution.

## Runtime Semantics

Operation handling is deep for the delimited body: after a provider returns,
later operations in the same body select the same handler again. A provider
returns exactly one operation result. It cannot abort, duplicate, retain, or
replace the suspended computation.

Handler selection is local to the current evaluation. `task::spawn` and
`task::spawn_with` do not inherit an installed handler implicitly. A spawned
job must install the handler it needs inside its own function body. This rule
keeps handler ownership explicit across the existing concurrency boundary.

The handler context may contain opaque host-owned resources such as
`NetStream`. The owner that installs the handler remains responsible for
shutdown and close unless the handled computation's documented contract says
otherwise.

## Acceptance Model

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| Declared operation call | The containing public function must declare the nominal effect | `check/user-effect-operation-boundaries` |
| Unknown operation | The operation name is rejected at the operation span | `check/user-effect-operation-boundaries` |
| Complete pure handler | The handled expression has no remaining nominal effect | `check/lexical-handler-effect-replacement` |
| Host-backed handler | The nominal effect is replaced by `net` | `check/lexical-handler-effect-replacement` |
| Missing provider | The handler declaration is rejected and names the missing operation in related context | `check/handler-operation-signatures` |
| Provider type mismatch | The provider binding is rejected at the binding span | `check/handler-operation-signatures` |
| Provider performs its handled effect | The provider is rejected as a recursive handler definition | `check/handler-operation-signatures` |
| Nested handlers | The innermost matching handler supplies the result, then the outer handler remains selected | `run/lexical-handler-nesting` |
| Repeated operations | The same deep handler processes operations in source evaluation order | `run/lexical-handler-repeated-operations` |
| Unhandled runnable effect | `veln run` and `veln test` reject the entry boundary before execution | `run/lexical-handler-unhandled-entry` |
| Spawn boundary | A handler outside a spawned job does not satisfy an operation inside that job | `check/lexical-handler-task-boundary` |

The relative paths in this table are planned directories below
`examples/specification/`. They do not describe currently passing evidence.
Parser, formatter, checker, checked-core, typed-IR, and JVM backend tests must
also cover the corresponding source nodes and lowering boundary.

## Diagnostics

The primary diagnostic message identifies the failed fact at its source span.
Missing providers, candidate effect declarations, and provider signatures
belong in related notes. JSON details should distinguish effect declaration,
operation call, handler declaration, provider binding, and runnable-entry
boundaries without exposing backend representation.

## Non-Goals

- Do not expose a continuation or `resume` value.
- Do not add aborting, shallow, or multi-shot handlers.
- Do not add effect variables or row polymorphism.
- Do not make handlers first-class values.
- Do not implicitly propagate handlers into tasks.
- Do not permit handlers to intercept `net`, `time`, `concurrency`, or another
  compiler-owned host effect.
- Do not convert runtime host failures into ordinary `Result` values.
- Do not prescribe dictionary passing, continuation passing, JVM exceptions,
  or another backend implementation strategy as language behavior.

## Completion Boundary

This proposal is complete when the source surface, effect transformation,
nested selection, task boundary, runnable-entry gate, diagnostics, and JVM
execution cases above are checked and the implemented behavior is promoted to
the matching specification pages. General resumptions remain separate work.
