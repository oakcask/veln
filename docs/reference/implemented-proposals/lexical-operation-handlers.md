# Lexical Operation Handlers

Status: implemented

## Summary

Lexical handlers supply every operation of one nominal operation effect for
the dynamic evaluation of one `handle Body with Handler(arguments)`
expression. A provider function returns the declared operation result, and the
suspended computation resumes automatically. Source does not expose
continuations, `resume`, aborting handlers, shallow handlers, or multi-shot
handlers.

Current behavior is specified by
`../../specification/source-surface.md`,
`../../specification/names-effects.md`, and the executable cases under
`../../../examples/specification/`.

## Implemented Boundary

- `handler ... handles ... end` declarations and `handle ... with ...`
  expressions are accepted by the parser, formatter, semantic AST, checked
  core, typed IR, editor token classification, and JVM execution path.
- A handler names context parameters, one handled nominal effect, an optional
  `effects [...]` list, and provider bindings from operation name to function
  path.
- The checker requires exactly one provider for every operation declared by
  the handled effect. Missing, duplicate, and unknown operation bindings are
  declaration diagnostics.
- Each provider receives the handler context parameters before the operation
  parameters and returns the declared operation result type.
- A provider may not retain the handled effect of its own handler.
- Public handlers must declare every retained provider effect. Private
  handlers infer retained provider effects.
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
| `check/lexical-handler-private-effect-inference` | A private handler retains effects inferred from its providers. |
| `check/handler-operation-signatures` | Missing, duplicate, unknown, mismatched, and recursive providers are rejected with handler-specific related context. |
| `run/lexical-handler-nesting` | Inner handlers shadow outer handlers only during the inner body. |
| `run/lexical-handler-repeated-operations` | A deep handler supplies repeated operations in evaluation order. |
| `run/lexical-handler-context-evaluation` | Context arguments evaluate once before the handled body. |
| `run/lexical-handler-unhandled-entry` | Runnable entry boundaries reject retained user-defined effects. |
| `check/lexical-handler-task-boundary` | Spawned jobs do not inherit an installed handler. |

The source grammar and accepted surface fixture are checked by
`../../specification/source-surface-executable.pl` and
`../../specification/source-surface-fixtures/accepted/handler-declaration.veln`.

## Follow-Ups

General resumable handlers remain separate planned work under
`../../proposals/one-shot-resumable-effect-handlers.md`. HTTP/2 connection
driver work may use lexical handlers but does not change the handler
semantics recorded here.
