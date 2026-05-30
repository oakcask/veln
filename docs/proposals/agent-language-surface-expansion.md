# Agent Language Surface Expansion

Status: proposed

This page tracks language surface features that remain outside the implemented
source subset. Proposal text here is not current language behavior unless
`../specification/` also states it.

## Read First

- Current source grammar and expression subset:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current type behavior:
  [../specification/types.md](../specification/types.md).
- Current names and effects:
  [../specification/names-effects.md](../specification/names-effects.md).

## Current Boundary

The implemented surface includes modules, imports, `fn`, `test`, contracts,
`let`, records, dictionaries, lists, `match`, built-in `Result` and `Option`
constructors, holes, `satisfy`, pipelines, calls, field access, and the current
operator subset.

The implemented lowering and execution boundary does not include user-defined
ADT declarations, method calls, loops, mutation, classes, traits, macros,
comprehensions, anonymous functions, custom operators, task selection, foreign
declarations, or package manifest fields beyond `[modules]`.

## Proposed Targets

Each feature below needs a narrow proposal before implementation:

- User-defined ADTs and constructor namespace rules.
- Method calls or receiver-style sugar, if they remain useful after targeted
  diagnostics for method-call-shaped syntax.
- Looping or comprehension syntax that does not duplicate existing `match` and
  helper-call patterns.
- Mutation or controlled state, including its interaction with frozen values.
- Anonymous functions and any extra inference needed for callback-heavy code.
- Traits, classes, macros, custom operators, and foreign declarations.
- Source-level task selection beyond current compiler-known concurrency calls.

## Non-Targets

- Do not add equivalent spellings that increase generation variance without a
  repair-loop benefit.
- Do not implement a broad feature family from this page directly; split one
  concrete feature into its own short proposal first.
