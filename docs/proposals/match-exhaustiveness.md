# Match Exhaustiveness

Status: open-proposal
Implementation: not implemented

This proposal requires `match` expressions to be statically exhaustive before
they can lower to runnable code. It is not a source for current behavior; use
`../reference/language/` for the implemented `match` rules.

## Read First

- Current source behavior:
  [../reference/language/source-surface.md](../reference/language/source-surface.md).
- Current type inference behavior:
  [../reference/language/types.md](../reference/language/types.md).
- Current execution behavior:
  [../reference/language/execution.md](../reference/language/execution.md).

## Proposal

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

## Implementation Plan

1. Add a semantic coverage pass after scrutinee type inference and before IR
   lowering.
2. Classify finite built-in domains and catch-all patterns before handling more
   complex nested coverage.
3. Emit focused diagnostics for the first missing case while preserving
   type-checking of arm expressions where possible.
4. Add checker tests for exhaustive and non-exhaustive `Bool`, `Option`, and
   `Result` matches.
5. Update the current language reference only after the compiler enforces the
   proposal.

## Skip Unless Needed

- Do not treat this proposal as implemented behavior unless
  `../reference/language/` also states the implemented rule.
- Do not require user-defined algebraic data type coverage through this
  proposal until those types are implemented in the language reference.
