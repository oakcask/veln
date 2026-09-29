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
| Ruby-style `begin`, `ensure`, and `end` | `begin` is a value-producing region and `ensure` always runs. | `ensure` belongs to Ruby's exception-handler family and puts one cleanup clause after the protected body. Veln uses `begin` only for lexical scope and registers cleanup next to each acquisition. |
| Go-style function `defer` | Cleanup is registered next to acquisition. | Function-only lifetime keeps loop or temporary resources alive longer than necessary. |
| D-style scope guard | Cleanup is registered next to acquisition and follows lexical lifetime. | This is the selected basis. The initial proposal includes unconditional exit cleanup only. |
| C#-style `using` | Common resource use is concise. | A single disposable protocol cannot express arbitrary effectful cleanup or cleanup that needs additional captured values. Veln can add library wrappers after the general mechanism exists. |

The proposal does not add `ensure`, `scope(success)`, or `scope(failure)`.
Code can use ordinary `Result` matching when it needs outcome-specific work.
The safety requirement is unconditional cleanup that cannot replace the
region's value or control transfer.

## Source and Static-semantics Evidence

The incomplete proposal has checked source and static-semantics evidence for
these forms:

```ebnf
DeferStatement ::= "defer" NL Body "end" NL?
BeginExpr      ::= "begin" NL Body "end"
```

The executable grammar, accepted and rejected fixtures, parser, formatter,
syntax navigation, LSP, and MCP retain the source forms and their spans for the
later runtime slice. Static checks enforce capture scope, unit result,
result-propagation, and nested-registration restrictions. The grammar has no
explicit `return`, `break`, `continue`, or other control-transfer form, so
there is no separate source case for transfer out of a deferred block.

| Case | Observable requirement | Evidence |
| --- | --- | --- |
| C9 | A deferred block that uses `?`, produces a non-unit value, or uses a source-supported control transfer out of the block fails checking with a repair note on the containing deferred block. | Checked human and JSON diagnostics cover the expressible restrictions. The grammar has no outward control-transfer form. |
| C10 | Parsing and formatting preserve both forms and their block ranges, and definition, reference, and rename operations respect bindings within those ranges. | Parser, formatter, LSP, and MCP cases check round trips and navigation. |

These cases are prerequisite evidence only. They do not promote either form to
the current language specification while runtime cases C1 through C8 remain
unimplemented.

## Remaining Runtime Contract

When execution reaches a `defer` statement, the runtime registers its block.
The runtime captures all referenced local bindings at that point. It does not
execute the block at registration time. A deferred block cannot refer to a
binding declared after the statement.

Fallible cleanup must handle its ordinary `Result` inside the block. The
example helper `close_or_report` represents that explicit policy; it is not a
new standard-library operation.

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
| C11 | More than one cleanup fails while the region is already failing. | The original failure remains primary and cleanup failures are attached in execution order. | Human and JSON runtime-failure cases with ordered related failures. |
| C12 | A successful region has a cleanup block that fails. | The first cleanup failure becomes the region failure after every cleanup block runs. | Run specification case with an event recorder and a failing cleanup. |
| C13 | One cleanup fails before another registered cleanup runs. | The remaining cleanup still runs in reverse registration order. | Run specification case with ordered events. |

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
