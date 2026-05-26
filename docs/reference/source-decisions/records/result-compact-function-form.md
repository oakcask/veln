# Discussion Result: Compact Function Form

Status: implemented

## Picked Question

- If `end` is used, should single-expression functions allow a compact form, or
  would that add too much syntax variance?

## Decision

Do not include a separate compact syntax for named single-expression functions
in the first slice. A named `fn` body always uses the normal block form and
closes with `end`, even when the body contains only one expression.

The body may still be expression-oriented. A single final expression can be the
function value without requiring an explicit `return`; the restriction is on the
outer function-body shape, not on expression-oriented evaluation.

## Rationale

The first slice should optimize for predictable repair and diagnostic behavior
more than for the shortest source text. A compact function form would create two
spellings for the same named function:

- a block body closed by `end`
- a compact expression body without `end`

That extra variance makes generated examples, formatter output, parser
recovery, and review diffs less uniform before the core repair loop has been
validated. It also weakens the value of the block-structure decision: a missing
`end` diagnostic is simpler when every named function body follows the same
outer shape.

Keeping one named-function shape does not force verbose imperative code. The
language can still treat the last expression as the result, and `veln fmt` can
keep one-expression bodies visually small.

## First-Slice Rule

- A named `fn` always has a body region closed by `end`.
- A one-expression named function uses the same outer body shape as any other
  named function.
- The last expression in a function body may provide the function result when
  its type matches the declared or inferred return type.
- The formatter should keep simple one-expression bodies compact through
  indentation and line breaking, not through a second function syntax.
- Parser diagnostics for named functions can assume that an opened `fn` expects
  a matching `end`.

## Open Detail

Anonymous functions or callback literals may still need a compact expression
form for collection operations. That is a separate grammar question because
callbacks appear inside expressions and may have different readability and
nesting pressure than named declarations.

## Consequence

The first implementation can keep named-function parsing, formatting, and JSON
diagnostics uniform. If examples later show that one-expression named helpers
are too noisy, compact syntax can be evaluated against concrete formatter and
diagnostic output instead of being added speculatively.
