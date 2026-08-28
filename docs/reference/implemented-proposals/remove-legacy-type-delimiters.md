---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Remove Legacy Type Delimiters

Veln no longer treats old type delimiter spellings as migration-specific
syntax. Angle brackets are the only spelling for declared type parameters,
type constructor arguments, and expression-level explicit type arguments.

## Read First

- Current source syntax:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current type annotation behavior:
  [../../specification/types.md](../../specification/types.md).
- Current parse repair behavior:
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Historical delimiter migration:
  [type-parameter-angle-brackets.md](type-parameter-angle-brackets.md)
  and
  [canonical-type-argument-delimiters.md](canonical-type-argument-delimiters.md).

## Problem

The language has already moved to canonical angle-bracket syntax:

```veln
type Box<A>
  Wrap(A)
end

fn unwrap(value: Box<Int>) -> Int
  _
end
```

Legacy delimiter spellings such as `type Box(A)`, `Box(Int)`, and
`callee[T](value)` are no longer accepted source syntax. Before this cleanup,
the parser still had migration-specific diagnostic and repair paths that
recognized those old forms and emitted dedicated replacement candidates. That
kept old spelling alive as a special case in the grammar, diagnostics, JSON
output, tests, and repair surface after the migration window had closed.

## Implemented Behavior

The legacy delimiter special case is removed from source parsing and
diagnostics.

- Keep `type Name<A>` as the only declared type-parameter spelling.
- Keep `Name<A>` as the only type constructor argument spelling in type
  positions.
- Keep `callee<T>(args...)` as the only implemented expression-level explicit
  type argument spelling for supported callees.
- Parse or check `type Name(A)`, `Name(A)` in type positions, and
  `callee[T](args...)` through ordinary error paths instead of
  legacy-delimiter-specific diagnostics.
- Stop emitting legacy delimiter safe repair candidates that replace `(`/`)` or
  `[`/`]` with `<`/`>`.
- Keep ordinary parser recovery good enough to report the next useful source
  location after the malformed delimiter.

## Diagnostics

Remove the dedicated legacy diagnostic ids from the current surface:

- `parse.legacy_type_parameter_delimiters`
- `parse.legacy_type_argument_delimiters`
- `parse.legacy_call_type_argument_delimiters`

Malformed old syntax should report the closest ordinary parse diagnostic for
the actual failed fact, such as an expected token, unexpected token, or invalid
type annotation boundary. JSON output should not include delimiter replacement
candidate edits for these cases.

## Scope

In scope:

- Parser branches that detect old delimiter spellings only to produce migration
  diagnostics.
- Repair candidate generation for legacy delimiter replacement.
- Specification examples and diagnostic JSON fixtures that assert legacy
  delimiter ids or replacement edits.
- Documentation that still describes legacy delimiter repair as current
  behavior.

Out of scope:

- Changing the canonical angle-bracket syntax.
- Adding higher-kinded type parameters, constraints, traits, or generalized
  user-defined generic calls.
- Changing value-level parentheses for calls, variant payloads, grouping,
  function types, or patterns.
- Changing square brackets for list literals or effect lists.

## Completion Evidence

- Source-surface syntax:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Type annotation behavior:
  [../../specification/types.md](../../specification/types.md) and
  [../../specification/types.md](../../specification/types.md).
- Diagnostic JSON and repair boundaries:
  [../../specification/diagnostics-json.md](../../specification/diagnostics-json.md),
  [../../specification/diagnostics-json.md](../../specification/diagnostics-json.md),
  and
  [../../specification/repair-candidates.md](../../specification/repair-candidates.md).
- Executable examples:
  `examples/specification/check/type-delimiter-diagnostics` and
  `examples/specification/repair/type-delimiter-refuse`.

## Acceptance Criteria

- Old delimiter spellings are rejected without any `parse.legacy_*delimiter*`
  diagnostic id.
- JSON diagnostics for old delimiter spellings do not include replacement edits
  that translate delimiters to `<` and `>`.
- The formatter and documentation continue to render only canonical
  angle-bracket type syntax.
- Specification examples cover the new ordinary-error behavior for old
  delimiter spellings, or existing examples are removed when they no longer
  document a distinct behavior.
