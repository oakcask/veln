# Canonical Type Argument Delimiters

Status: proposed

This proposal makes angle brackets the only source spelling for type
parameters and explicit type arguments. Proposal text is not current language
behavior unless `../specification/` also states it.

## Read First

- Current source grammar and declaration boundary:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current type annotation behavior:
  [../specification/types.md](../specification/types.md).
- Current standard effect call behavior:
  [../specification/names-effects.md](../specification/names-effects.md).
- Completed angle-bracket migration record:
  [../reference/implemented-proposals/type-parameter-angle-brackets.md](../reference/implemented-proposals/type-parameter-angle-brackets.md).

## Current Boundary

Angle brackets are the current source spelling for declared type parameters
and type constructor arguments in type positions:

```veln
type Envelope<A, E>
  Ok(A)
  Err(E)
end

fn parse(raw: String) -> Result<Envelope<String, Int>, ParseError>
  _
end
```

The parser still accepts legacy parenthesized spelling in type positions during
the compatibility window:

```veln
type Envelope(A, E)
  Ok(A)
  Err(E)
end

fn parse(raw: String) -> Result(Envelope(String, Int), ParseError)
  _
end
```

Expression-level explicit type arguments currently use square brackets for
recognized built-in calls:

```veln
fn make() -> { tx : Sender<String>, rx : Receiver<String> } effects [concurrency]
  channel::bounded[String](1)
end
```

The formatter canonicalizes legacy parenthesized type positions to angle
brackets. The checker does not currently report a style diagnostic for the
legacy parenthesized spelling or for square-bracket explicit type arguments on
recognized calls.

## Proposed Target

Use angle brackets for all source type parameters and explicit type arguments:

- Keep `type Name<A>` as the accepted spelling for declared type parameters.
- Keep `Name<A>` as the accepted spelling for type constructor arguments in
  annotations, record field types, function types, contract result bindings,
  doctest metadata, generated wrappers, and other type positions.
- Change expression-level explicit type arguments from
  `channel::bounded[T](capacity)` and `task::spawn[T](job)` to
  `channel::bounded<T>(capacity)` and `task::spawn<T>(job)`.
- Reject `type Name(A)` declarations. The accepted spelling is `type Name<A>`.
- Reject `Name(A)` type constructor arguments in type positions. The accepted
  spelling is `Name<A>`.
- Reject square-bracket expression-level type arguments on recognized calls.
  The accepted spelling is `callee<T>(args...)`.
- Preserve value-level parentheses for calls, constructor payloads, grouped
  expressions, patterns, and function type parameter lists.
- Preserve square brackets for list literals and effect lists such as
  `effects [concurrency]`.
- Report parse diagnostics at the legacy delimiter that state the specific
  failed fact and point to the angle-bracket spelling.
- Add repair candidates that replace legacy delimiters with angle brackets
  when the token span is unambiguous.

## Expression Parsing Boundary

Angle-bracket explicit type arguments are valid only as a postfix suffix on a
name-path callee followed by a call. For example,
`channel::bounded<String>(1)` parses as a type-applied callee call.

Expression parsing must continue to treat `<`, `<=`, `>`, and `>=` as
comparison operators outside that narrow callee suffix position. The parser
should not reinterpret arbitrary expressions such as `value < limit` as type
application.

The accepted expression-level type-argument surface remains limited to
recognized built-in calls until a later proposal generalizes explicit type
arguments for user-defined generic calls.

## Diagnostics

The primary diagnostic should stay local to the rejected delimiter. Related
notes may explain that Veln type parameters and explicit type arguments use
angle brackets, while parentheses remain value-call syntax and square brackets
remain list or effect-list syntax.

Examples:

- `type Box(A)` reports that parenthesized type parameters are no longer
  accepted for `type` declarations.
- `Result(Int, E)` in a return annotation reports that parenthesized type
  arguments are no longer accepted in type annotations.
- `channel::bounded[String](1)` reports that square-bracket explicit type
  arguments are no longer accepted for calls.

## Required Evidence

- Parser tests for accepted `channel::bounded<String>(1)` and
  `task::spawn<String>(job)` calls.
- Parser tests for rejected `type Name(A)` declarations, rejected `Name(A)`
  type annotations, and rejected `callee[T](args...)` explicit type calls.
- Parser or CLI diagnostics coverage for the human output and JSON diagnostic
  shape.
- Repair fixture coverage for safe delimiter replacement where the legacy
  type span is complete.
- Formatter fixture updates that remove legacy type-argument input as an
  accepted formatting case or canonicalize it before rejection is enabled.
- Specification updates that remove compatibility wording from
  `../specification/source-surface.md`, `../specification/types.md`, and
  `../specification/names-effects.md`.
- Example updates for concurrency and task calls that currently spell explicit
  type arguments with square brackets.

## Non-Goals

- Do not change constructor expression syntax such as `Some(value)` or
  source-declared variant payload syntax such as `Ok(A)`.
- Do not change function type syntax such as `fn(String) -> Result<(), E>`.
- Do not add generic functions, constraints, traits, or higher-kinded type
  parameters.
- Do not generalize expression-level explicit type arguments beyond recognized
  built-in calls.

## Update When

- The compatibility parser paths are removed or replaced.
- Diagnostics or repair candidate behavior for legacy type spelling changes.
- The proposal is implemented and promoted into `../specification/`.
