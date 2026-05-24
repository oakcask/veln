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
- record field access
- stdio builtins, prelude helpers, ordinary function calls, and function-value
  calls
- integer and boolean operators used by the implemented type rules
