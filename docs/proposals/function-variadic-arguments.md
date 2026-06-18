# Function Variadic Arguments

Status: proposed

This proposal defines source-level variadic function parameters for ordinary
functions and function values. It is not current language behavior until the
matching specification page and executable examples state it.

## Problem

Call sites that naturally pass a sequence of homogeneous values must currently
build an explicit `List<T>` value before calling a helper. That keeps the core
model small, but makes logging helpers, string assembly, assertion helpers,
and small collection builders noisier than the fixed-arity call surface around
them.

The language already checks exact call arity and function-value assignment.
Variadic support needs to preserve those checks for fixed parameters while
giving a clear source spelling for "zero or more values of one type".

## Scope

The first slice adds variadic parameters to ordinary function declarations and
function types:

```veln
fn join_words(separator: String, words: ...String) -> String
  join_word_list(separator, words)
end

fn log_all(prefix: String, messages: ...String) -> ()
  write_lines(prefix, messages)
end
```

At a call site, arguments after the fixed parameters are gathered into the
variadic parameter:

```veln
join_words(", ", "red", "green", "blue")
log_all("debug")
```

Inside the function body, the variadic binding has type `List<T>`. In the
examples above, `words` and `messages` have type `List<String>`.

## Discussion Result: Surface Syntax

A variadic parameter is written as `name: ...T`.

The variadic marker belongs to parameter syntax, not to ordinary type
annotation syntax. `...T` is accepted only for declaration parameters and
function type parameters. It is invalid in let annotations, return annotations,
record field types, constructor payloads, schema field types, and type
arguments unless it appears as the final parameter of a function type.

The source parser and AST should preserve whether a parameter is variadic
instead of leaving the marker only inside a raw type string. That lets
placement diagnostics and formatter output point at the variadic marker before
lowering turns the body binding into `List<T>`.

Only the final parameter in a parameter list may be variadic. A function may
have at most one variadic parameter.

```veln
fn valid(prefix: String, values: ...String) -> String
  join_word_list(prefix, values)
end
```

These declarations are invalid:

```veln
fn rest_not_last(values: ...String, suffix: String) -> String
  suffix
end

fn duplicate_rest(left: ...String, right: ...String) -> String
  ""
end
```

The parser should report the invalid variadic marker at the parameter that
breaks the rule. The diagnostic should identify the failed declaration fact,
not speculate about call-site intent.

## Discussion Result: Type Model

The source type of a variadic function preserves the variadic marker:

```veln
fn(String, ...String) -> ()
```

This keeps function-value behavior aligned with call behavior. A callable with
one fixed `String` parameter and a variadic `String` tail is not the same source
type as a callable with two fixed parameters:

```veln
fn(String, List<String>) -> ()
fn(String, ...String) -> ()
```

Assignment compatibility for function types checks:

- the same fixed parameter count before the variadic tail,
- pairwise assignability for fixed parameter types,
- matching variadic presence,
- assignability between variadic element types,
- return type compatibility, and
- existing effect compatibility.

The variadic element type is monomorphic at each function declaration or
function type. A variadic parameter cannot mix unrelated element types unless
the element type itself admits those values through ordinary assignment rules.

A variadic function type is not assignment-compatible with any fixed-arity
function type solely because some calls overlap. In particular,
`fn(String, ...String) -> ()` is not assignment-compatible with
`fn(String) -> ()`, `fn(String, String) -> ()`, or
`fn(String, List<String>) -> ()` unless a later proposal adds an explicit
adapter or spread operation.

When this proposal is promoted into the current specification, prose that uses
`fn(T, ...) -> U` as descriptive shorthand for arbitrary ordinary function
types should be rewritten. Variadic source syntax should always show a concrete
element type after the marker, such as `fn(String, ...String) -> ()`, so
readers and tools do not confuse meta-ellipsis with implemented syntax.

## Discussion Result: Function-Typed Reachability

Calls through a function-typed local binding or parameter keep using the
source-level function type, not the lowered `List<T>` representation, to choose
conservative reachable declarations.

For a non-variadic function type, the existing exact argument-count reachability
rule remains unchanged. For a variadic function type with `N` fixed
parameters, a written call with at least `N` arguments may conservatively reach
visible declarations whose source type has the same fixed parameter count and a
matching variadic tail. A written call with fewer than `N` arguments is already
an arity error and should not add extra reachability candidates.

## Discussion Result: Call Checking

For a non-variadic function, existing exact arity checks remain unchanged.

For a variadic function with `N` fixed parameters, a call must provide at least
`N` arguments. The first `N` arguments are checked against the fixed parameter
types. Every remaining argument is checked against the variadic element type.

```veln
fn collect(label: String, values: ...Int) -> Int
  list_sum(values)
end

collect("ok")
collect("ok", 1, 2, 3)
```

The first call passes an empty list to `values`. The second call passes a list
with three `Int` values.

When a call has fewer than the fixed parameter count, the arity diagnostic
should say that the call expects at least the fixed count:

```text
call expects at least 1 argument(s), but got 0
```

When an extra variadic argument has the wrong type, the primary diagnostic
should report the type mismatch at that argument. A related note may identify
the variadic parameter and its element type.

## Discussion Result: Lowering And Execution

The checked core may lower a variadic parameter to an ordinary `List<T>`
binding in the function body. This keeps execution and local inference aligned
with existing list behavior.

Call lowering constructs the gathered list in source argument order. The
runtime value observed by the callee is the same value a caller would have
passed manually to a non-variadic helper that accepts `List<T>`.

The source-level function type remains variadic even if the lowered
representation uses `List<T>`. Diagnostic output, editor support, formatter
output, and documentation generation should use the source-level variadic
shape.

## Discussion Result: Pipeline Calls

The pipeline operator keeps its current rule: the left expression is checked as
the first argument of the named or qualified call on the right.

```veln
fn tag_all(prefix: String, values: ...String) -> String
  join_word_list(prefix, values)
end

"debug" |> tag_all("red", "green")
```

For a variadic callee, that is equivalent to:

```veln
tag_all("debug", "red", "green")
```

If the left expression fills a fixed parameter, later written arguments are
checked in their shifted positions. If the left expression lands in the
variadic tail, it is checked against the variadic element type and gathered
with the remaining tail arguments.

```veln
fn words(values: ...String) -> String
  join_word_list(" ", values)
end

"red" |> words("green")
```

The second example is equivalent to `words("red", "green")`.

## Discussion Result: Entry Arguments

Command entry functions may be variadic only when every fixed parameter and the
variadic element type are command-line convertible under the existing entry
argument rules.

For a selected variadic entry function with `N` fixed parameters, command-line
execution requires at least `N` entry arguments. Extra entry arguments are
converted to the variadic element type and gathered into the callee binding.

This preserves exact argument count behavior for non-variadic entry functions
and avoids adding separate command-line spread syntax.

## Discussion Result: Formatter And Editor Support

The formatter preserves the `name: ...T` spelling and the final-parameter
position.

Semantic token support should classify the parameter name like other parameter
declarations. The `...` marker does not introduce a new symbol.

Generated documentation should render variadic parameters with their source
spelling and should not describe the lowered `List<T>` representation as the
public call shape.

## Diagnostics

The proposal needs diagnostic coverage for:

- variadic parameter not in final position,
- more than one variadic parameter,
- variadic marker used outside declaration or function-type parameter syntax,
- variadic parameter missing its element type annotation,
- missing fixed arguments at a variadic call,
- wrong element type in a variadic argument,
- function-value assignment mismatch between variadic and non-variadic
  callable types,
- invalid variadic entry argument conversion.

Every variadic parameter must spell its element type. Private function
inference does not infer the element type for `name: ...`; that form is an
invalid annotation rather than an omitted private parameter annotation.

Human diagnostics should keep the primary message on the failed fact at the
reported span. Related notes can identify the callee declaration, the variadic
parameter, or the expected element type.

## Examples

Executable specification coverage should include:

- a run case where a variadic function receives zero tail arguments,
- a run case where a variadic function receives multiple tail arguments in
  order,
- a check case for too few fixed arguments,
- a check case for a wrong tail element type,
- a check case for invalid declaration placement,
- a check case for a variadic marker outside parameter syntax,
- a check case for a missing variadic element type annotation,
- a check case for function-value assignment compatibility,
- a check or run case that proves function-typed reachability follows the
  source-level variadic type rather than the lowered `List<T>` representation,
- a pipeline case that fills a fixed parameter, and
- a pipeline case that fills the variadic tail.

## Non-Goals

This proposal does not add:

- spread calls such as `log_all("debug", ...messages)`,
- named arguments,
- default arguments,
- heterogeneous variadic parameters,
- variadic constructors,
- variadic test declarations,
- format-string-sensitive typing, or
- source overloading by arity.

Spread calls can be a later proposal once the basic variadic call and function
type model is implemented.

## Completion Criteria

The proposal is complete when:

- `docs/specification/` states the implemented variadic declaration, type,
  call, assignment, function-typed reachability, pipeline, and entry-argument
  rules,
- `examples/specification/` includes the executable coverage listed above,
- parser, formatter, checker, lowerer, runtime, diagnostics, editor support,
  and documentation output agree on the source-level variadic shape, and
- this proposal is either moved to implemented proposal history or replaced by
  a short closed route that points to the implemented behavior.
