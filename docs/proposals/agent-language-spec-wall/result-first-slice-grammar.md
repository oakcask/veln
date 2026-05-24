# Discussion Result: First-Slice Grammar

Status: accepted-proposal

## Picked Question

- What exact grammar should the first Veln slice use, now that block structure,
  calls, contracts, effects, typed holes, and public API boundaries have
  separate decisions?

## Decision

Commit the first slice to a small, line-oriented, keyword-delimited,
expression-centered grammar. The grammar should be exact enough for parser,
formatter, diagnostics, examples, and golden tests, but intentionally exclude
surface alternatives that do not shorten the repair loop.

The first slice includes modules, imports, public and private functions,
typed parameters, explicit public return types, optional result bindings,
public effect declarations, `require` and `ensure` clauses, `let` bindings,
tail-expression blocks, records, lists, `match`, built-in `Option` and
`Result` constructors and patterns, plain function calls, qualified function
calls, field access, pipelines, `?`, and expression holes.

The first slice excludes statement braces, semicolon-separated statement
lists, indentation-sensitive nesting, method calls, user-defined ADT
declarations, loops, mutation, classes, traits, macros, comprehensions,
anonymous functions, custom operators, and dictionary literals. Dictionary
types may appear in signatures as `Dict(K, V)`, but literal syntax should wait
until examples show a repair-loop need.

## Canonical Grammar

The canonical grammar now lives in
[Veln First-Slice Grammar Target](../../proposals/grammar-target.md). This discussion result
records the original decision and rationale; later grammar updates such as
`test` declarations and hole `satisfy` clauses are consolidated in the
proposal document.

## Rationale

The first grammar should optimize for parseable partial programs and stable
repair targets, not for maximum expressiveness. This is an inference from the
literature rather than a direct empirical result about agents.

Stefik and Siebert's empirical work shows that syntax choices materially affect
novice accuracy and that familiar C-style punctuation is not automatically
better. Lappi, Tirronen, and Itkonen's replication work reinforces the broader
point that many common syntactic choices are unintuitive and should be treated
as evidence-bearing design decisions. For Veln, that argues against adding
multiple equivalent spellings early.

Qiu, Li, Barr, and Su studied syntax use across a large Java corpus and found
that rule use is uneven and contextual. That supports a deliberately restricted
first grammar: ship the common repair-loop structures first, then add syntax
only when examples show sustained pressure.

Medeiros, Alvez Junior, and Mascarenhas motivate robust syntax recovery for
tools that need useful ASTs even when programs are invalid. Veln has the same
constraint for agents. Explicit `end`, newline separators, restricted block
forms, and a small set of synchronization tokens give `veln check --json`
better recovery anchors than a wide grammar full of optional separators,
overloaded punctuation, and expression-level statement blocks.

Typed-hole and partial-program research also favors keeping holes inside the
ordinary expression grammar. Type-directed completion and live typed-hole work
depend on local expected types flowing through parseable incomplete programs.
For Veln, `_` and `_name` should therefore be normal expressions, usable inside
calls, records, lists, matches, and tail positions.

## First-Slice Rules

- Public functions use `pub fn`; private functions use `fn`.
- Public functions must include an explicit return type and an explicit
  `effects [...]` clause, including `effects []` for pure public functions.
- Private functions may omit `effects [...]`; effect inference and diagnostics
  follow the effect-boundary decision.
- A result binding uses `-> name: Type`; it is visible only to `ensure`
  clauses for that function.
- Function bodies are newline-separated `let` statements followed by an
  optional tail expression. The tail expression is the returned value.
- General calls use `name(args)` or `module::name(args)`.
- `value.name(args)` is not a call form. `value.field` is field access only.
- Pipelines use `expr |> name(args)` and insert the piped expression as the
  first argument of the target call.
- Pipeline targets are ordinary calls in the grammar. The first slice accepts
  only named or qualified calls as pipeline targets; placeholder-based pipeline
  targets remain a later syntax decision.
- `match` arms use `pattern => expression` and close with `end`.
- Constructor patterns use the general constructor pattern grammar. In the
  first slice, the resolver accepts only the built-in `Option` and `Result`
  constructors: `Some(value)`, `None`, `Ok(value)`, and `Err(error)`.
- Unqualified constructor names must start with an uppercase letter. Qualified
  names in pattern position are parsed as constructor names, but user-defined
  constructors remain unavailable in the first slice.
- Type syntax uses `TypePath` and type-argument application rather than a
  hard-coded list of built-in type forms. The first-slice resolver accepts
  primitive types, built-in parametric forms, collection types, function types,
  and opaque named types according to the type-system decisions.
- Function types may carry an `effects [...]` suffix using the same spelling as
  function declarations. If the first checker does not yet model higher-order
  effects, it should report an unsupported function-type effect rather than a
  parse error.
- `?` is a postfix operator and follows the existing `Result` propagation
  decisions.
- Braces are records or record patterns, not statement blocks.
- Missing `end`, missing newline separators, malformed call arguments, and
  method-call-shaped syntax should produce targeted parse diagnostics when
  recovery can identify the construct.

## Open Details

This decision does not freeze the final concrete syntax for package manifests,
foreign declarations, doctest fences, dictionary literals, or future
user-defined data type declarations. Test declaration syntax is resolved by
[Test Declaration Syntax](../../reference/source-decisions/result-test-declaration-syntax.md), which adds a
top-level `test` item and supersedes treating ordinary zero-argument `fn`
declarations as durable test syntax.

Record expressions and record patterns intentionally require explicit
`name: value` and `name: pattern` fields in the first slice. Shorthand fields,
rest patterns, spreads, and update syntax remain open syntax questions.

Contract clauses use a narrower predicate production than full `Expr`; see
[Contract Predicate Parsing](../../reference/source-decisions/result-contract-predicate-parsing.md) for the
implementation rule.

The grammar sketch allows record types with `{ field: Type }`, but the parser
may need context-sensitive handling because record expressions use the same
braces. That is acceptable in the first slice because the contexts are
syntactically distinct: signatures and annotations expect types, while
expression positions expect values.

Operator precedence is intentionally small. If examples show confusion around
mixed boolean, arithmetic, pipeline, and `?` expressions, the formatter should
prefer parentheses before the language adds more operators.

## References

- Stefik, A., & Siebert, S. (2013). An empirical investigation into programming
  language syntax. *ACM Transactions on Computing Education*, 13(4), Article
  19. https://doi.org/10.1145/2534973
- Lappi, V., Tirronen, V., & Itkonen, J. (2023). A replication study on the
  intuitiveness of programming language syntax. *Software Quality Journal*,
  31, 1211-1240. https://doi.org/10.1007/s11219-023-09631-7
- Qiu, D., Li, B., Barr, E. T., & Su, Z. (2017). Understanding the syntactic
  rule usage in java. *Journal of Systems and Software*, 123, 160-172.
  https://doi.org/10.1016/j.jss.2016.10.017
- Medeiros, S. Q. de, Alvez Junior, G. de A., & Mascarenhas, F. (2019).
  *Automatic Syntax Error Reporting and Recovery in Parsing Expression
  Grammars*. arXiv:1905.02145. https://arxiv.org/abs/1905.02145
- Perelman, D., Gulwani, S., Ball, T., & Grossman, D. (2012). Type-directed
  completion of partial expressions. *PLDI 2012*, 275-286.
  https://doi.org/10.1145/2254064.2254098
- Omar, C., Voysey, I., Chugh, R., & Hammer, M. A. (2019). Live functional
  programming with typed holes. *Proceedings of the ACM on Programming
  Languages*, 3(POPL), 1-32. https://doi.org/10.1145/3290327

## Consequence

The first parser, formatter, examples, and golden diagnostics can now target
one concrete source shape. Later syntax proposals must show that they improve
the repair loop enough to pay for extra parser, formatter, resolver, and
diagnostic complexity.
