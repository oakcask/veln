# Execution Boundary

This file specifies the implemented execution boundary.

## Core And IR

Checked core is produced only after semantic diagnostics have no errors. Typed
IR is produced only when checked core is complete. Reachable holes, missing
expressions, constructor arity gaps, and call arity gaps block executable IR.
For selected `run` and `test` entries, reachability includes direct function
calls, bare function declaration values used inside reachable expressions, and
function calls in reachable contract predicates. The implemented execution
fixtures cover function declarations used as function-typed values,
function-typed value calls, contract helper reachability, and selected-entry
reachable-hole blocking before JVM execution.

The typed IR is runtime-neutral. JVM class names, Java method names, boxed
runtime representation, generated artifact paths, and runtime helper layout are
backend details and are not language facts.

## JVM Backend

The JVM backend generates Java source for the implemented IR subset:

- functions, parameters, locals, expression statements, and returns
- literals, records, lists, `Ok`, `Err`, `Some`, `None`, their `Result::` or
  `Option::` qualified forms, and `?`
- `match` expressions over literals, `_`, bindings, and built-in `Option` and
  `Result` constructors
- record field access
- stdio builtins, prelude helpers, ordinary function calls, and function-value
  calls
- pipelines lowered to calls with the left expression inserted as the first
  argument
- runtime `require` checks at function entry and runtime `ensure` checks before
  ordinary tail-expression returns
- integer and boolean operators used by the implemented type rules

Generated runtime helpers may use mutable builders while constructing records,
lists, and dictionary update results. Values returned to Veln user code are
frozen at that boundary: records and dictionaries are exposed as unmodifiable
maps, lists are exposed as unmodifiable lists, and prelude container updates
return new frozen containers instead of mutating the input value in place.

This freeze rule is an observable language boundary only through value
immutability and update semantics. The exact JVM representation, copying
strategy, and later structural-sharing choices remain backend details.

Runtime contract failures stop the selected `run` entry or fail the selected
test case. Human output names the failed clause text, function boundary, source
identity, and blame route. `veln run --json` reports one top-level structured
runtime error record. `veln test --json` embeds runtime contract failures in
the failed case with structured runtime contract details. `require` uses caller
blame; `ensure` uses implementation blame.
