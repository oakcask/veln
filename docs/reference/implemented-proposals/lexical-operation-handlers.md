---
role: implementation-record
authority: supporting
review-when: The lexical handler implementation boundary, replacement proposal, or executable handler evidence changes.
---

# Lexical Operation Handlers

## Summary

Lexical handlers supply every operation of one nominal operation effect for
the dynamic evaluation of one `handle Body with Handler(arguments)`
expression. Current source writes explicit operation clauses in the form
`operation(parameter_names) => expression`. The clause expression returns the
declared operation result, and the suspended computation resumes
automatically. Source does not expose continuations, `resume`, aborting
handlers, shallow handlers, or multi-shot handlers.

The initial implementation used `operation = function_path` provider
bindings. That source form is historical and was superseded by
[Explicit Handler Operation Clauses](explicit-handler-operation-clauses.md).
Current specifications, checked examples, and standard-library handlers use
only explicit operation clauses.

Current behavior is specified by
`../../specification/source-surface.md`,
`../../specification/names-effects.md`, and the executable cases under
`../../../examples/specification/`.

## Implemented Boundary

- `handler ... handles ... end` declarations and `handle ... with ...`
  expressions are accepted by the parser, formatter, semantic AST, checked
  core, typed IR, editor token classification, and JVM execution path.
- A handler names context parameters, one handled nominal effect, an optional
  `effects [...]` list, and one operation clause for each operation of the
  handled effect.
- Each operation clause binds zero or more untyped operation parameters and
  evaluates an ordinary expression body. Handler context parameters are normal
  lexical bindings in the clause body.
- The checker requires exactly one operation clause for every operation
  declared by the handled effect. Missing, duplicate, and unknown operation
  clauses are declaration diagnostics.
- A clause must bind the same number of parameters as the handled operation,
  must not repeat a binding name, and must return the declared operation
  result type.
- A clause parameter shadows an outer handler context parameter of the same
  name only inside that clause body. Other clauses keep the outer context
  binding unless they declare their own shadowing parameter.
- A clause body may not retain the handled effect of its own handler.
- Public handlers must declare every retained clause effect. Private handlers
  infer retained clause effects.
- Declared handler effects are canonicalized and duplicate-free.
- A `handle` expression contributes `C union (B without E) union H`, where
  `C` is the union of context argument effects, `B` is the body effect set,
  `E` is the handled nominal effect, and `H` is the handler effect set.
- Handler context arguments evaluate once, left to right, before the handled
  body.
- Handling is deep for repeated operations in the handled body.
- An inner handler for the same nominal effect shadows an outer handler only
  while the inner body evaluates.
- Handler selection is local to the current task. `task::spawn` and
  `task::spawn_with` do not inherit installed handlers.
- Runnable `veln run` and `veln test` entry boundaries reject any retained
  user-defined effect after handler checking.
- Contracts cannot install handlers and retain their effect-free call rule.
- Compiler-owned host effects such as `net`, `time`, and `concurrency` remain
  host effects, not interceptable nominal operations.

## Evidence

The executable specification covers the externally visible behavior:

| Case | Observation |
| --- | --- |
| `check/lexical-handler-effect-replacement` | Handling removes the nominal effect and retains handler effects. |
| `check/lexical-handler-private-effect-inference` | A private handler retains effects inferred from its operation clause expressions. |
| `check/lexical-handler-public-effect-declarations` | A public handler must declare retained clause effects, while duplicate declarations are canonicalized. |
| `check/handler-operation-parameter-signatures` | Wrong operation-parameter counts, duplicate bindings, result-type failures, and ordinary call failures inside clauses are rejected. |
| `check/handler-operation-signatures` | Missing, duplicate, unknown, and recursive operation clauses are rejected with handler-specific related context. |
| `check/handler-operation-signatures-human` | Handler-specific related context is rendered in human diagnostics. |
| `run/handler-operation-direct-clauses` | Zero-, one-, and multi-parameter clauses can return direct expressions, rename operation parameters, and shadow context parameters lexically. |
| `run/handler-operation-synthetic-name-collision` | Lowered clause helpers do not collide with user-visible names. |
| `run/lexical-handler-nesting` | Inner handlers shadow outer handlers only during the inner body. |
| `run/lexical-handler-repeated-operations` | A deep handler supplies repeated operations in evaluation order. |
| `run/lexical-handler-context-evaluation` | Context arguments evaluate once before the handled body. |
| `run/lexical-handler-early-return-cleanup` | An inner handler is removed when `?` returns early from the function that installed it. |
| `run/lexical-handler-unhandled-entry` | Runnable entry boundaries reject retained user-defined effects. |
| `check/lexical-handler-task-boundary` | A lexical handler around a task creation expression can discharge the task call's exposed job effect row. |
| `test/lexical-handler-success` | `veln test` runs a test whose retained operation effect is fully handled. |

The source grammar and accepted surface fixture are checked by
`../../specification/source-surface-executable.pl` and
`../../specification/source-surface-fixtures/accepted/handler-declaration.veln`.
The rejected surface fixture
`../../specification/source-surface-fixtures/rejected/handler-operation-old-syntax.veln`
keeps the superseded `operation = function_path` form out of the current
grammar.

## Superseded Provider Boundary

Provider bindings, implicit context-then-operation argument ordering,
provider-retained effects, and provider-specific coverage diagnostics describe
only the first implementation of lexical handlers. They are not part of the
current source surface or public diagnostic contract.

The current boundary is the explicit operation-clause contract: clauses bind
operation parameters in source, call external functions only through ordinary
expressions, retain effects from those expressions, and report coverage through
operation-clause diagnostics. Remaining provider wording in this record is
historical and points to the completed replacement record above.

## Boundary Decision

The project rejected explicit one-shot resumptions because no checked example
requires handler-controlled suspension or continuation disposition. The durable
decision and reconsideration gate are recorded in
[One-Shot Resumable Handler Boundary](../source-decisions/records/result-one-shot-resumable-handler-boundary.md).
HTTP/2 connection driver work may use lexical handlers but does not change the
handler semantics recorded here.
