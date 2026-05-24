# Discussion Result: Pipeline Style

Status: implemented

## Picked Question

- Should pipeline be preferred only for multi-step data flow, or should it be
  the default style for all function composition?

## Decision

Prefer pipeline only for multi-step data flow where the intermediate value is
the main subject of the expression. Do not make pipeline the default style for
all function composition in the first slice.

Plain function calls remain the canonical form for simple calls, predicates,
constructors, small nested expressions, and calls where the primary meaning is
the operation rather than the carried value.

## Rationale

Pipeline syntax is useful when it makes a transformation sequence linear:
parse input, validate the parsed value, normalize it, and render or store the
result. In that shape, the reader and the diagnostic tool can track one subject
through several named steps without mentally unpacking nested calls.

Making pipeline the default for every composition would create avoidable style
variance. Agents would need to decide whether `is_valid(trim(text))` should be
written as a pipeline even when the expression is short and predicate-shaped.
That decision is not semantically important, but it would affect examples,
formatter output, diffs, and repair suggestions.

The method-call decision already keeps first-slice calls uniform. Pipeline can
provide a left-to-right spelling for genuine data-flow chains without adding
receiver lookup, method namespaces, or a second general call model.

## First-Slice Rule

- Function call syntax remains the neutral default.
- Pipeline is preferred when an expression has at least two transformation
  steps over the same subject and reads better left to right.
- Pipeline should pass the previous value as the next function's first
  argument unless a later grammar decision introduces an explicit placeholder.
- `veln fmt` may line-break long pipelines one step per line, but it must not
  rewrite ordinary function calls into pipelines just to enforce a global style.
- `veln check` diagnostics should describe the desugared function-call shape
  when reporting type, effect, contract, or hole information inside a pipeline.
- Standard-library examples should use pipelines sparingly for data-flow chains
  and plain calls for short expressions.

## Open Detail

The first-slice grammar resolves the pipeline token as `|>` and gives it lower
precedence than boolean, comparison, arithmetic, postfix `?`, calls, and field
access. Anonymous functions remain outside the first slice.

The first slice also needs examples for fallible chains. The pipeline rule
should compose with `Result` propagation and traversal helpers such as
`try_map`, but this result does not decide whether pipeline has special
fallible behavior.

## Consequence

The first implementation can support left-to-right transformation examples
without turning pipeline into a mandatory style rule. This keeps formatter and
diagnostic behavior predictable while preserving a path for fluent data-flow
code.
