# Execution Boundary

This file specifies the implemented execution boundary.

## Core And IR

Checked core is produced only after semantic diagnostics have no errors. Typed
IR is produced only when checked core is complete. Reachable holes, missing
expressions, constructor arity gaps, call arity gaps, and recognized
concurrency calls block executable IR. For selected `run` and `test` entries,
reachability includes direct function calls, bare and `use` alias-qualified
function declaration values used inside reachable expressions, and function
calls in reachable contract predicates. Reachability also follows bare and
`use` alias-qualified function declaration values passed as contract call
arguments. Calls through a function-typed local binding or parameter are
conservative: when the surface graph does not identify one concrete function
declaration, reachability includes visible function declarations with the same
argument count. In a named source module, a bare function reference resolves
reachability only to functions owned by that same source module. Qualified
calls and function values resolved through selected-file `use` aliases keep
the imported module identity, so same-named functions from other modules are
not included only because their local name matches. Bare local bindings,
parameters, and match-pattern bindings shadow same-named function declarations
for selected-entry reachability; a shadowed bare name is treated as the local
value, not as a function declaration value. The implemented execution fixtures
cover function declarations used as function-typed values, function-typed value
calls, opaque function-typed value call reachability, contract helper
reachability, contract function value reachability, imported-call
reachable-hole blocking, selected-entry reachable-hole blocking, local
shadowing of function declarations, and selected-entry concurrency blockers
before JVM execution.
When a function or test body omits the final expression line, checked core and
typed IR materialize that omission as an explicit `()` return.

The typed IR is runtime-neutral. JVM class names, Java method names, boxed
runtime representation, generated artifact paths, cache keys, and runtime
helper layout are backend details and are not language facts.

Stdio operations are serialized at the runtime handler boundary. Each
`stdio::print`, `stdio::println`, `stdio::eprint`, and `stdio::eprintln`
operation writes its complete logical output and records its test event while
holding the same handler lock. Captured event `sequence` values therefore
define one total operation order across stdout and stderr for a selected run or
test case, including calls made by spawned tasks.

## JVM Backend

The JVM backend generates Java source for the implemented IR subset:

- functions, parameters, locals, expression statements, and returns
- omitted tail expressions as `()` returns
- literals, records, lists, `Ok`, `Err`, `Some`, `None`, their `Result::` or
  `Option::` qualified forms, and `?`
- `match` expressions over literals, `_`, bindings, and built-in `Option` and
  `Result` constructors, after finite-domain exhaustiveness diagnostics have
  passed
- record field access
- stdio builtins, prelude helpers, ordinary function calls, and function-value
  calls
- bounded channel construction, sender clone, send, receive, and close calls
- two-receiver channel selection calls with optional timeout
- task spawn, join, and cancellation calls
- pipelines with named or qualified call targets lowered to calls with the
  left expression inserted as the first argument
- runtime `require` checks at function entry and runtime `ensure` checks before
  tail-expression returns and `?` early returns
- integer and boolean operators used by the implemented type rules

Generated runtime helpers may use mutable builders while constructing records,
lists, and dictionary update results. Values returned to Veln user code are
frozen at that boundary: records and dictionaries are exposed as unmodifiable
maps, lists are exposed as unmodifiable lists, and prelude container updates
return new frozen containers instead of mutating the input value in place.

Bounded channel values are backend-owned runtime handles. `channel::bounded`
and `channel::bounded[T]` return a record with `tx` and `rx` fields.
`channel::clone(tx)` returns another sender endpoint for the same channel.
Sending freezes the sent value before crossing the channel boundary. On a
positive-capacity channel, sending waits while the queue is full and then
returns `Ok(())` after the value is queued. Receiving blocks until a queued
value is available or the sender endpoint is closed. It returns `Some(value)`
for a received value and `None` after the channel is closed and drained. A
capacity of zero creates a no-buffer rendezvous channel. It has no queue
storage: sending waits until a receiver is ready, transfers the value directly,
and then returns `Ok(())`. A waiting receive on a zero-capacity channel returns
`Some(value)` when the paired send transfers a value.
Closing the sender endpoint prevents later sends from succeeding and wakes
waiting receivers.
`channel::select(left, right)` observes two receivers with the same item type.
It returns the first ready value as `Some({index, value})`, using `0` for the
left receiver and `1` for the right receiver, and returns `None` only after
both receivers are closed and drained. If both receivers are ready during one
runtime poll, repeated selections rotate the first polled receiver so that
ties alternate between `0` and `1`.
`channel::select_priority(left, right)` has the same receiver and return
behavior, except ties in one runtime poll always choose the left receiver.
`channel::select_timeout(left, right, timeout_ms)` has the same receiver,
return, and rotating tie-breaking behavior. It also returns `None` when no
value is selected before the non-negative millisecond timeout elapses. A
negative timeout waits without a timeout, matching `channel::select`.
`channel::select_result`, `channel::select_priority_result`, and
`channel::select_timeout_result` use the same readiness, tie-breaking,
closed-channel, and timeout rules as their non-result counterparts. They
return `Ok(Some(selected))` when a receiver produces a value, `Ok(None)` when
selection closes or times out without a value, and `Err(SelectError)` when
cooperative cancellation interrupts the waiting selection.

Task values are backend-owned runtime handles. `task::spawn` starts a
zero-argument callable on a JVM thread and freezes the returned value before it
crosses the task boundary. `task::join` waits for that task and returns
`Ok(value)` on ordinary completion or `Err(JoinError)` on interruption,
cancellation, or runtime failure. `task::cancel` requests cooperative
cancellation by interrupting the task.

This freeze rule is an observable language boundary only through value
immutability and update semantics. The exact JVM representation, copying
strategy, and later structural-sharing choices remain backend details.

Runtime contract failures stop the selected `run` entry or fail the selected
test case. Human output names the failed clause text, function boundary, source
identity, and blame route. `veln run --json` reports one top-level structured
runtime error record. `veln test --json` embeds runtime contract failures in
the failed case with structured runtime contract details. `require` uses caller
blame; `ensure` uses implementation blame. When `?` propagates an error result
out of a function, the function's `ensure` clauses run before that early
return.

The JVM execution path keeps a persistent class cache for generated Java
sources. Cache hits may skip `javac`, but command results, stdout, stderr,
contract traces, and captured stdio events are defined as if the generated
program was compiled for that invocation.
