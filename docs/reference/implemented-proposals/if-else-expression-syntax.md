# If Else Expression Syntax

Status: implemented

Implemented source behavior includes `if` / `else if` / `else` / `end`
expression syntax for ordinary Boolean branching, distinct surface AST
preservation through semantic analysis input, formatter preservation, type and
effect checking aligned with equivalent `match Bool` expressions, and checked
diagnostic evidence. Current behavior is specified in
`../../specification/source-surface.md` and `../../specification/types.md`.

## Problem

Veln currently uses `match` for both finite-domain destructuring and simple
Boolean branches. That keeps the expression model small, but it makes common
true/false control flow longer than the condition it protects:

```veln
match observed_length > expected_length
	true => reject_too_large(observed_length)
	false => accept_length(observed_length)
```

Nested Boolean checks become especially noisy because each branch repeats
`true` and `false` as pattern labels:

```veln
match input_ready
	true => decode_ready(input)
	false => match input_closed
		true => finish_closed(input)
		false => wait_for_more(input)
```

The source syntax should let examples express ordinary Boolean decisions
without making `match` do double duty for every conditional expression.

## Goals

- Add a readable expression form for Boolean branching:
  `if condition ... else if condition ... else ... end`.
- Keep `if` an expression, not a statement-only control-flow form.
- Require every `if` expression to produce one type-compatible value.
- Preserve `match` as the canonical destructuring and exhaustiveness form for
  ADTs, records, and other finite domains.
- Keep parsing and formatting deterministic so nested `if` and `match`
  expressions remain easy to read.
- Reuse existing Boolean expression checking, branch type unification, effect
  checking, typed-hole context, and diagnostics wherever possible.

## Non-Goals

- Do not remove or deprecate `match Bool`.
- Do not add statement blocks, mutable control flow, early return, `elsif`, or
  ternary syntax.
- Do not add pattern matching inside `if` conditions.
- Do not infer truthiness for non-`Bool` values.
- Do not add optional braces, parentheses, or colon-delimited branch bodies.
- Do not introduce a fall-through or implicit-unit branch value.

## Syntax

An `if` expression contains one leading condition, zero or more `else if`
conditions, one required `else` branch, and a closing `end` marker:

```veln
if observed_length > expected_length
	reject_too_large(observed_length)
else if input_closed
	finish_closed(input)
else
	wait_for_more(input)
end
```

The condition after `if` or `else if` is an ordinary expression checked as
`Bool`. Each branch body is an expression body with the same body shape used by
current function and match-arm expression bodies. Branch bodies may contain
nested `if`, `match`, `let`, calls, records, collections, and other existing
expression forms.

The parser should treat `else if` as a chained conditional branch, not as an
`else` body whose first expression happens to be another `if`. That gives the
formatter one canonical layout and gives diagnostics a stable branch index.

The required `else` keeps `if` total as an expression. A conditional decision
that intentionally has no alternative must continue to encode an explicit
ordinary value such as `None`, `Ok(value)`, or a domain-specific no-op value.

## Desugaring Model

Semantically, `if` is equivalent to a nested `match` over `Bool`:

```veln
if first
	first_value
else if second
	second_value
else
	default_value
end
```

has the same value behavior as:

```veln
match first
	true => first_value
	false => match second
		true => second_value
		false => default_value
```

The implementation does not need to lower through surface `match` text, but
the observable checking rules should match this model unless this proposal
states a narrower diagnostic shape.

## Type And Effect Checking

Each condition must type check as `Bool`. A non-`Bool` condition is a
type-checking diagnostic at the condition expression span. Related notes may
name the branch keyword and the condition's inferred type.

All branch result expressions must be assignment-compatible with one expected
result type. If an expected type is available from an enclosing expression,
return position, local binding annotation, call argument, record field,
constructor payload, or match arm, that expected type should flow into every
branch body. If no expected type is available, the checker should compute the
same least common branch result accepted by equivalent `match` behavior.

Branch effects combine the same way as `match` arm effects. The `if`
expression requires the enclosing function or expression context to permit the
union of effects required by reachable conditions and branch bodies.

Typed holes inside a condition receive expected type `Bool`. Typed holes inside
branch bodies receive the enclosing expected result type when one exists.

## Exhaustiveness And Reachability

Because every `if` expression requires `else`, it is syntactically exhaustive
for Boolean control flow. The checker does not need a separate missing-branch
exhaustiveness diagnostic.

Constant-condition reachability may be added as a later warning or lint, but
the first slice should not reject:

```veln
if true
	value
else
	fallback
end
```

The formatter and checker should still parse and check all branches, including
branches whose condition is a literal, so diagnostics inside those bodies remain
visible and examples stay deterministic.

## Parser And AST

The AST should represent `if` as its own expression node instead of erasing it
immediately into `match`. The node should preserve:

- the primary `if` condition span
- ordered `else if` condition spans
- ordered branch body spans
- the optional keyword spans needed by diagnostics and editor token output
- the final `else` body span
- the closing `end` span

Keeping a distinct node lets formatter, editor support, source maps, and
diagnostics report the syntax the user wrote. Later lowering may reuse the same
semantic path as Boolean `match` once source spans are attached to the right
condition and branch facts.

## Formatter

The formatter should canonicalize `if` expressions as:

```veln
if condition
	body
else if condition
	body
else
	body
end
```

Nested `if` expressions inside a branch body are indented one level deeper.
`else if` stays on one line. The formatter should not rewrite `if` to `match`
or rewrite `match Bool` to `if`; preserving the user's chosen branch form keeps
source diffs focused.

## Style Lint

A future style lint should prefer `if` / `else if` / `else` over `match` when
the matched value is `Bool` and the expression is being used for ordinary
true/false value branching. The lint should not apply when the source is
specifically exercising `match` parsing, Boolean pattern arms, exhaustiveness
behavior, formatter preservation, or diagnostic recovery for `match`.

The lint should report style guidance only. It should not make `match Bool`
invalid source, and it should not require the formatter to rewrite one branch
form into the other.

## Diagnostics

Initial diagnostics should cover:

- missing `else` before `end` or end of input
- missing `end` after an `if` expression
- `else if` without a following condition
- `else` followed by another branch keyword before a body expression
- non-`Bool` `if` or `else if` condition
- incompatible branch result types

Primary messages should focus on the failed fact at the reported span. For
example, a non-`Bool` condition should point at the condition expression rather
than the whole `if`. An incompatible branch result should point at the branch
body that fails the expected result type, with related notes for the earlier
branch or enclosing expected type.

JSON diagnostics may reuse existing parse and type mismatch ids where their
details already describe the failure. New ids are useful only when callers need
to distinguish missing `else`, missing `end`, or branch-chain parse recovery
from generic expression parse failures.

## Examples Acceptance

Completion evidence lives under `../../../examples/specification/`:

- successful `check` behavior, including nested `if` and `match` expressions
  in opposite orders:
  `../../../examples/specification/check/if-expression-syntax/case.toml`
- formatter preservation:
  `../../../examples/specification/fmt/if-expression-syntax/case.toml`
- non-`Bool` condition diagnostics in JSON and human output:
  `../../../examples/specification/check/if-expression-condition-diagnostics/case.toml`
  and
  `../../../examples/specification/check/if-expression-condition-human/case.toml`
- incompatible branch result diagnostics in JSON and human output:
  `../../../examples/specification/check/if-expression-branch-diagnostics/case.toml`
  and
  `../../../examples/specification/check/if-expression-branch-human/case.toml`
- parse recovery for missing `else`, missing `end`, missing `else if`
  condition, and malformed `else` branch bodies:
  `../../../examples/specification/check/if-expression-parse-recovery-diagnostics/case.toml`

Existing examples may migrate from `match Bool` to `if` only when the example
is not specifically testing `match` parsing, Boolean pattern arms, or
exhaustiveness behavior. The migration should be mechanical and should keep the
same behavior and expected output.

## Implementation Order

1. Add tokens and parser recovery for `if`, `else`, and `end` as expression
   syntax.
2. Add a distinct AST expression node and source metadata for branch spans.
3. Add formatter support and parser/formatter round-trip fixtures.
4. Lower or check `if` through the existing Boolean branch path while
   preserving source spans for diagnostics.
5. Add type, effect, typed-hole, and repair-context coverage matching
   equivalent `match Bool` behavior.
6. Add specification examples and promote implemented behavior into
   `../../specification/source-surface.md` only after the behavior exists.

## Deferred Questions

- Should `if` conditions allow a line break immediately after `if` before the
  condition expression, or should that remain a parse error for clearer
  recovery?
- Should branch-chain diagnostics expose stable branch indexes in JSON details,
  or are source spans enough for editor clients?
