---
role: proposal
update-when: Lexical cleanup syntax, begin-scope exit behavior, task cancellation unwinding, or cleanup failure precedence is implemented or redesigned.
---

# Lexical Deferred Cleanup Runtime

This proposal remains incomplete. Its checked source and static-semantics
slice does not make lexical cleanup a supported language feature because the
runtime does not register or execute deferred blocks. The remaining work is
runtime unwinding for normal, propagated-error, failure, and cancellation
exits. The mechanism must cover files, sockets, locks, effect handlers, spans,
and future resources without requiring destructors or garbage-collector
finalizers.

## Outcome

Execute each accepted `defer` block when its nearest cleanup region exits.
Function and test bodies are cleanup regions. A `begin` expression introduces
a shorter cleanup region. A handler operation clause remains a single
expression and uses `begin` when it needs a cleanup region.

```veln
fn load(address: String) -> Result<String, LoadError> effects [net]
  let stream = net::connect(address)?
  defer
    close_or_report(stream)
  end

  net::read_to_end(stream)
end
```

```veln
let response = begin
  let stream = net::connect(address)?
  defer
    close_or_report(stream)
  end

  exchange(stream)?
end
```

The `begin` expression has the type and value of its final expression after all
registered deferred blocks complete successfully.

## Why This Form

| Form | Useful property | Reason not selected as the primary form |
| --- | --- | --- |
| C++ or Rust destruction | Cleanup follows ownership automatically. | Veln does not have the ownership and deterministic-destruction model needed to make destructor timing a language invariant. |
| Java-style `try` and `finally` | The protected region and cleanup are explicit. | `try` suggests an exception-catching model that Veln does not expose, and several resources require deeply nested regions. |
| Ruby-style `begin`, `ensure`, and `end` | `begin` is a value-producing region and `ensure` always runs. | `ensure` belongs to Ruby's exception-handler family and puts one cleanup clause after the protected body. The proposed Veln form uses `begin` only for lexical scope and would register cleanup next to each acquisition. |
| Go-style function `defer` | Cleanup is registered next to acquisition. | Function-only lifetime keeps loop or temporary resources alive longer than necessary. |
| D-style scope guard | Cleanup is registered next to acquisition and follows lexical lifetime. | This is the selected basis. The initial proposal includes unconditional exit cleanup only. |
| C#-style `using` | Common resource use is concise. | A single disposable protocol cannot express arbitrary effectful cleanup or cleanup that needs additional captured values. Veln can add library wrappers after the general mechanism exists. |

The proposal does not add `ensure`, `scope(success)`, or `scope(failure)`.
Code can use ordinary `Result` matching when it needs outcome-specific work.
The safety requirement is unconditional cleanup that cannot replace the
region's value or control transfer.

## Implemented Prerequisite

The static and tooling boundary for the source forms is current behavior in
the [source-surface specification](../specification/source-surface.md#static-cleanup-region-forms).
This proposal contains only the unimplemented runtime contract.

## Remaining Runtime Contract

When execution reaches a `defer` statement, the runtime registers its block.
The runtime captures all referenced local bindings at that point. It does not
execute the block at registration time.

## Exit and Failure Rules

The runtime executes registered blocks in reverse registration order. It runs
them when the region exits because of:

- normal completion;
- propagation through `?`;
- contract failure;
- runtime failure; or
- task cancellation that terminates the task through Veln's task runtime.

The runtime does not promise cleanup after `process::exit`, host process
termination, virtual-machine failure, or loss of the executing machine.

Cleanup failure follows these rules:

| Region state before cleanup | Cleanup outcome | Region outcome |
| --- | --- | --- |
| Successful | All blocks complete | Preserve the region value. |
| Successful | A block fails | Propagate the first cleanup failure after running the remaining blocks. |
| Already failing | All blocks complete | Preserve the original failure. |
| Already failing | One or more blocks fail | Preserve the original failure and attach cleanup failures as ordered related failures. |

A cleanup failure must not hide an earlier contract failure, runtime failure,
or cancellation. Every registered block still runs after another cleanup block
fails.

## Acceptance Model

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| C1 | A region completes normally after registering one block. | The block runs once before the region transfers its value. | Run specification case with an event recorder. |
| C2 | A region propagates `Err` through `?`. | Registered blocks run before the caller observes the `Err`. | Run specification case. |
| C3 | A region raises a contract or runtime failure. | Registered blocks run before the failure leaves the region. | Human and JSON runtime-failure cases. |
| C4 | Three blocks are registered. | They run once each in reverse registration order. | Run specification case with ordered events. |
| C5 | Acquisition fails before execution reaches `defer`. | The unregistered block does not run. | Run specification case. |
| C6 | A `begin` expression completes successfully. | Its cleanup runs before the expression value is bound outside the scope. | Run specification case. |
| C7 | Cleanup fails while the region is already failing. | The original failure remains primary and cleanup failure is related context. | Human and JSON runtime-failure cases. |
| C8 | A Veln task is cancelled while inside a cleanup region. | Task completion is not reported until registered cleanup has run. | Deterministic task-runtime case. |
| C9 | More than one cleanup fails while the region is already failing. | The original failure remains primary and cleanup failures are attached in execution order. | Human and JSON runtime-failure cases with ordered related failures. |
| C10 | A successful region has a cleanup block that fails. | The first cleanup failure becomes the region failure after every cleanup block runs. | Run specification case with an event recorder and a failing cleanup. |
| C11 | One cleanup fails before another registered cleanup runs. | The remaining cleanup still runs in reverse registration order. | Run specification case with ordered events. |

## Verification and Promotion

Runtime cases belong under `examples/specification/`. After all listed runtime
cases pass, the execution behavior must be explained in the current execution
specification and this proposal must be removed from the proposal catalog.

Task cancellation cannot satisfy this proposal by abandoning the host thread.
The task runtime must enter the same cleanup-unwind path used by other abrupt
region exits.

## Non-goals

- Automatic destruction based on garbage collection is not specified.
- Cleanup does not recover from a contract or runtime failure.
- `defer` does not replace explicit handling of meaningful close or flush
  results.
- The proposal does not define a universal disposable interface.
- `begin` does not introduce `rescue`, `ensure`, or an exception value.

## Design References

- [Go defer statements](https://go.dev/ref/spec#Defer_statements)
- [D scope guard statements](https://dlang.org/spec/statement.html#ScopeGuardStatement)
- [Ruby exception handling and `ensure`](https://ruby-doc.org/3.4/exceptions_md.html)
- [Java `finally`](https://docs.oracle.com/javase/specs/jls/se17/html/jls-14.html#jls-14.20.2)
- [C# `using`](https://learn.microsoft.com/dotnet/csharp/language-reference/language-specification/statements#the-using-statement)
