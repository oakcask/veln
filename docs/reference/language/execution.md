# Execution Boundary

This file specifies the implemented execution boundary.

## Core And IR

Checked core is produced only after semantic diagnostics have no errors. Typed
IR is produced only when checked core is complete. Reachable holes, missing
expressions, constructor arity gaps, and call arity gaps block executable IR.

The typed IR is runtime-neutral. JVM class names, Java method names, boxed
runtime representation, generated artifact paths, and runtime helper layout are
backend details and are not language facts.

## JVM Backend

The JVM backend generates Java source for the implemented IR subset:

- functions, parameters, locals, expression statements, and returns
- literals, records, lists, `Ok`, `Err`, `Some`, and `?`
- `match` expressions over literals, `_`, bindings, and built-in `Option` and
  `Result` constructors
- record field access
- stdio builtins, prelude helpers, ordinary function calls, and function-value
  calls
- integer and boolean operators used by the implemented type rules

Generated runtime helpers may use mutable builders while constructing records,
lists, and dictionary update results. Values returned to Veln user code are
frozen at that boundary: records and dictionaries are exposed as unmodifiable
maps, lists are exposed as unmodifiable lists, and prelude container updates
return new frozen containers instead of mutating the input value in place.

This freeze rule is an observable language boundary only through value
immutability and update semantics. The exact JVM representation, copying
strategy, and later structural-sharing choices remain backend details.
