# Canonical Type Argument Delimiters

Status: implemented

This record keeps the completed removal of legacy type-argument delimiter
compatibility after the implemented source surface moved to angle brackets.
Current behavior is specified in `../../specification/`; this page is history
and completion evidence.

## Implemented Behavior

- Declared type parameters use `type Name<A>`.
- Type constructor arguments in source type positions use `Name<A>`.
- Expression-level explicit type arguments for recognized built-in calls use
  `callee<T>(args...)`, such as `channel::bounded<String>(1)` and
  `task::spawn<String>(job)`.
- `type Name(A)` declarations are parse errors.
- `Name(A)` type constructor arguments in type positions are parse errors.
- Square-bracket explicit type arguments such as `callee[T](args...)` are parse
  errors.
- Value-level parentheses remain constructor, call, grouping, pattern, and
  function-type syntax. Square brackets remain list literal and effect-list
  syntax.

## Completion Evidence

- Source grammar and expression boundaries:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Type annotation spelling:
  [../../specification/types.md](../../specification/types.md).
- Standard concurrency call spelling:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Parse diagnostics and repair candidates:
  [../../specification/diagnostics-json.md](../../specification/diagnostics-json.md)
  and
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Executable examples cover type-delimiter diagnostics, type-delimiter repair,
  and concurrency run calls in the specification example suite.

## Notes

Angle-bracket explicit type arguments remain limited to recognized built-in
calls. This record does not generalize user-defined generic function calls,
traits, constraints, or higher-kinded type parameters.

## Update When

- Type-argument diagnostics or repair candidate fields change.
- Explicit type arguments are generalized beyond recognized built-in calls.
- The source grammar changes the accepted delimiter spelling again.
