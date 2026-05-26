# Match Exhaustiveness

Status: implemented
Implementation: implemented in the current checker for `Bool`, `Option(T)`,
and `Result(T, E)`

This proposal records the promotion of static `match` exhaustiveness checking.
It is not the source for current behavior; use `../reference/language/` for
the implemented `match` rules.

## Read First

- Current source behavior:
  [../reference/language/source-surface.md](../reference/language/source-surface.md).
- Current type inference behavior:
  [../reference/language/types.md](../reference/language/types.md).
- Current execution behavior:
  [../reference/language/execution.md](../reference/language/execution.md).

## Promoted Behavior

Every `match` expression must cover every value in the scrutinee type that the
compiler can classify for the implemented pattern language. A non-exhaustive
`match` should be rejected during checking instead of relying on backend
fallback code that fails at runtime.

The first implementation should require exhaustive coverage for:

- `Bool`, where both `true` and `false` must be covered unless a catch-all arm
  is present.
- `Option(T)`, where `Some(_)` and `None` must be covered unless a catch-all
  arm is present.
- `Result(T, E)`, where `Ok(_)` and `Err(_)` must be covered unless a catch-all
  arm is present.

The checker should treat `_` and binding patterns as catch-all arms. Record
patterns, literals outside finite built-in domains, and scrutinee types that are
unknown during checking may initially require a catch-all arm because the
compiler cannot prove full coverage from enumerated constructors alone.

## Diagnostics

Report one primary diagnostic at the `match` expression when no arm covers a
remaining case. The primary message should state the specific missing case, for
example `match is missing case None` or `match is missing case false`.

Use related notes for the scrutinee type and for arms that prove partial
coverage. Human output should cover both the primary message and the related
context expected from the diagnostic.

## Compatibility

This is a source-breaking change for programs that currently compile but can
fall through to `non-exhaustive match` at runtime. The break is intentional:
callers should express fallback behavior with `_` or a binding arm, or enumerate
the finite cases they intend to handle.

Backends may keep defensive runtime fallback code for malformed IR or future
coverage gaps, but normal checked programs should not depend on that fallback.

## Implemented Scope

- The semantic checker runs coverage after scrutinee type inference and arm
  expression checking.
- Coverage is classified for `Bool`, `Option(T)`, and `Result(T, E)`.
- `_` and binding patterns are catch-all arms.
- The checker emits a focused `type.match_non_exhaustive` diagnostic for the
  first missing case, with related notes for the scrutinee type and arms that
  prove partial coverage.
- Checked core and IR are not produced while a non-exhaustive finite-domain
  match diagnostic is present.

## Skip Unless Needed

- Do not treat this proposal as implemented behavior unless
  `../reference/language/` also states the implemented rule.
- Do not require user-defined algebraic data type coverage through this
  proposal until those types are implemented in the language reference.
