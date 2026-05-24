# Discussion Result: Doctest Result Propagation

Status: accepted-proposal

## Picked Question

- How should doctest examples handle `Result` and `?`?

## Decision

Executable doctest examples may use `?`, but only inside a result-returning
doctest context supplied by the test harness.

For the first slice, every executable doctest should be checked as an isolated
generated private test function. If a doctest body contains `?`, the generated
function returns `Result((), E)`. The checker may infer `E` when all
propagated fallible operations use one concrete error type, or when the
documented public item gives an unambiguous `Result(_, E)` context. If more
than one incompatible error type appears, the doctest must declare the intended
error type at the doctest boundary.

The success value should be implicit. Authors should not need to write a
trailing `Ok(())` or type-disambiguating success expression just to make a
doctest with `?` run. A doctest passes when the generated wrapper returns
`Ok(())` and any assertions or expected-output checks pass. A returned `Err`
is a test failure unless a later negative-example mode explicitly says that an
error is expected.

## Rationale

Doctests are useful only if they remain examples first. Hoffman and Strooper's
work on executable examples frames API examples as readable, partial formal
specifications whose consistency can be checked by running them. Python and Go
make the same product choice in different forms: examples are documentation
that the toolchain can execute or compare against expected output. Knuth's
literate programming work is a heavier ancestor of the same locality idea:
source-adjacent explanation should be close enough to code to stay coherent.

Rust is the closest precedent for Veln's `Result` and `?` question. Rustdoc
extracts examples, wraps small examples so authors do not need to write a full
`main`, and supports `?` in documentation tests by giving the example a
result-returning context. It also exposes a friction point: when `?` leaves the
error type ambiguous, examples need explicit disambiguation. Veln should keep
the useful part, a result-returning doctest context, but avoid pushing noisy
type-disambiguating success expressions into examples.

This matches Veln's existing error-inference boundary. Doctests are generated
private functions, so they may use the same narrow private-helper inference
rule: infer one concrete propagated error type when obvious, but require an
explicit boundary when mixed errors would otherwise force an unstable inferred
union. The visible example remains copyable, while the diagnostic can point at
the exact `?` whose error type does not fit the doctest's chosen error type.

## First-Slice Rules

- `veln check` extracts and type-checks executable doctest examples.
- `veln test` runs executable doctest examples through the same static gates as
  normal test files.
- A doctest containing `?` is valid only when the generated doctest wrapper has
  a known `Result((), E)` return type.
- The checker may infer the doctest error type when every propagated fallible
  operation has the same concrete error type.
- The checker may use the documented public item's explicit `Result(_, E)`
  return type as context only when doing so is unambiguous.
- A doctest with incompatible propagated error types must declare the intended
  error type at the doctest boundary.
- The generated wrapper appends the success value; the source example should
  not require a visible `Ok(())` purely for harness mechanics.
- Assertions and expected-output checks are the preferred way to show success
  behavior. Printing or comparing the debug representation of `Ok` and `Err`
  should not be the default style.
- Returned `Err` values are failures in the first slice. Negative examples and
  expected-error doctests can be designed later.
- User-authored hidden setup lines are not required for the first slice. The
  generated wrapper is tool-owned and should be visible in diagnostics when it
  matters.

## Example Shape

An example attached to `load_config` can stay focused on the API behavior:

```veln
/// ```veln
/// let cfg = load_config("app.veln")?
/// assert_eq(cfg.name, "demo")
/// ```
pub fn load_config(path: String) -> Result(Config, ConfigError) effects fs
end
```

The checker treats the block as if it were wrapped in a private generated test:

```veln
fn __doctest_load_config() -> Result((), ConfigError) effects fs
  let cfg = load_config("app.veln")?
  assert_eq(cfg.name, "demo")
  Ok(())
end
```

If the block also calls an operation returning `Result(_, IoError)` and no
conversion is present, the diagnostic should ask for either an explicit
conversion near that `?` or an explicit doctest error type at the block
boundary.

## Open Details

Resolved by
[Doctest Error Type Fence Syntax](result-doctest-error-type-fence-syntax.md):
write an explicit doctest error type as a fenced-code info-string attribute,
`error=<TypePath>`, on the executable Veln doctest block.

Resolved by
[Doctest Expected Output Syntax](result-doctest-expected-output-syntax.md):
compare stdout and stderr with adjacent `veln-output stream=...` fences
attached to the immediately preceding executable Veln doctest block, after the
result-returning wrapper succeeds.

## Consequence

Veln doctests can demonstrate fallible APIs without teaching users harness
ceremony. Agents get a small local repair surface: when `?` works, the example
reads like normal Veln code; when it fails, the checker can report the missing
or incompatible doctest error type instead of producing a confusing wrapper
failure.

## References

- `hoffman2003-api-executable-examples`: executable examples as readable,
  partial formal API specifications checked by running tests.
- `rustdoc-documentation-tests`: official Rust practice for extracted
  documentation tests, generated wrappers, and `?` in result-returning doctests.
- `python-doctest`: official Python practice for examples that serve as both
  documentation and tests with expected-output comparison.
- `go-testable-examples`: official Go practice for examples displayed in
  documentation and verified by the package test suite.
- `knuth1984-literate-programming`: historical research context for keeping
  explanation and executable source close enough to preserve coherence.
