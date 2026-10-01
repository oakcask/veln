---
role: proposal
update-when: Public deferred-cleanup execution, task-cancellation unwind, cleanup-failure precedence, continued cleanup, or readiness evidence is implemented or redesigned.
---

# Public Lexical Deferred Cleanup Integration

The public executable pipeline still rejects a selected entry that can reach
`begin` or `defer`. The internal JVM behavior behind that readiness gate is
specified by the [execution boundary](../specification/execution.md#runtime-readiness-and-host-boundaries).
This proposal contains only the work that remains before the gate can open.

## Outcome

Public commands accept and execute the existing lexical cleanup forms after
the remaining runtime rules and public evidence are complete. Cleanup must
cover files, sockets, locks, effect handlers, spans, and future resources
without relying on destructors or garbage-collector finalizers.

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

The public pipeline must use the existing checked-core, typed-IR, and JVM
foundation. It must not bypass the readiness blocker until all remaining
acceptance cases and public evidence pass.

## Task Cancellation

Task cancellation that terminates a Veln task must enter the cleanup-unwind
path before task completion is reported. It must use the snapshots captured
when each cleanup was registered. It must not retain a live binding slot or
resolve a captured name again when the region exits.

Task cancellation cannot satisfy this proposal by abandoning the host thread.

## Cleanup-Failure Rules

Cleanup failure has the following remaining planned precedence:

| Region state before cleanup | Cleanup outcome | Region outcome |
| --- | --- | --- |
| Successful | One or more blocks fail. | Run the remaining blocks, then propagate the first cleanup failure. |
| Already failing | One or more blocks fail. | Run the remaining blocks, preserve the original failure, and attach cleanup failures as ordered related failures. |

A cleanup failure must not hide an earlier contract failure, runtime failure,
or cancellation. Every registered block must still run after another cleanup
block fails.

## Remaining Acceptance Cases

| Case | Input or transition | Required observation | Planned evidence |
| --- | --- | --- | --- |
| C7 | Cleanup fails while the region is already failing. | The original failure remains primary and the cleanup failure is related context. | Human and JSON runtime-failure cases. |
| C8 | A Veln task is cancelled inside a cleanup region. | Task completion is not reported until registered cleanup has run. | Deterministic task-runtime case. |
| C9 | More than one cleanup fails while the region is already failing. | The original failure remains primary and cleanup failures are attached in execution order. | Human and JSON runtime-failure cases with ordered related failures. |
| C10 | A successful region has a cleanup block that fails. | The first cleanup failure becomes the region failure after every cleanup block runs. | Run specification case with an event recorder and a failing cleanup. |
| C11 | One cleanup fails before another registered cleanup runs. | The remaining cleanup still runs in reverse registration order. | Run specification case with ordered events. |

## Public Evidence and Promotion

Before the readiness gate opens, `examples/specification/` must exercise the
public executable pipeline for the cleanup behavior owned by the execution
specification. Public human and JSON command cases must also cover contract and
runtime failures. Internal compiler and backend tests do not replace this
public evidence.

After those cases and the remaining acceptance cases pass, update the current
execution specification, remove the readiness blocker, and remove this
proposal and its catalog entry.

## Limits

- Cleanup after `process::exit`, host process termination, virtual-machine
  failure, or loss of the executing machine is not promised.
- Automatic destruction based on garbage collection is not specified.
- Cleanup does not recover from a contract or runtime failure.
- `defer` does not replace explicit handling of meaningful close or flush
  results.
- The proposal does not define a universal disposable interface.
- `begin` does not introduce `rescue`, `ensure`, or an exception value.
