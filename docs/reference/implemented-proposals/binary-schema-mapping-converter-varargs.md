# Binary Schema Mapping Converter Varargs

Status: implemented

This record preserves the completed generated decode mapping converter-call
slice from `../../proposals/schema-declaration-surface.md`. Current behavior
is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode mapping evaluates pure same-module converter
calls and imported public pure converter calls with one or more supported
structural arguments. The converter arity is governed by ordinary function-call
type checking rather than a fixed generated-helper ladder.

Converter arguments remain inside the existing structural mapping vocabulary:
schema-local fields, record construction, ADT constructor construction,
supported pure converter calls, field selections from record-shaped mapping
expressions, and supported integer mapping expressions.

This slice does not add arbitrary ordinary function calls, private imported
converter access, bare imported converter names, effects, runtime state, or
schema mapping expressions outside the current structural vocabulary.

## Evidence

This historical slice was retired with schema-level mapping support.

## Remaining Work

The broader schema declaration proposal remains open for schema runtime
mapping outside the implemented structural mapping vocabulary and for binary
schema fields outside the implemented generated helper slices.
