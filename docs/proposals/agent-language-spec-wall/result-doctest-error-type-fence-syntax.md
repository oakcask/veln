# Discussion Result: Doctest Error Type Fence Syntax

Status: accepted-proposal

## Picked Question

- What exact syntax should a doctest use when `?` needs an explicit error type
  at the doctest boundary?

## Decision

Use a fenced-code info-string attribute on executable Veln doctests:

````markdown
```veln error=ConfigError
let cfg = load_config("app.veln")?
assert_eq(cfg.name, "demo")
```
````

The first info-string token identifies the language and remains `veln`. The
first slice recognizes `error=<TypePath>` as a doctest harness attribute, not
as source-language syntax. `TypePath` should use the same type-path spelling
accepted in ordinary Veln type annotations, including qualified names when
those are available.

When this attribute is present, the generated private doctest wrapper returns
`Result((), <TypePath>)`, and every propagated `?` in the example must either
produce that error type or be explicitly converted before propagation. The
attribute is optional when the error type can be inferred from one concrete
propagated error type or from an unambiguous documented public item context.
It is required when mixed fallible operations would otherwise leave the
doctest wrapper error type ambiguous.

Do not use a doc-comment directive or a visible wrapper function for this first
slice. The boundary belongs to the example block metadata, while the example
body should stay copyable Veln code.

## Rationale

The earlier doctest result decided that examples should remain examples first:
the test harness may wrap them, but users should not have to write harness
ceremony such as a visible wrapper function or a trailing `Ok(())`. Hoffman
and Strooper argue for executable API examples as readable partial
specifications, and that framing makes the human-facing example body the
important artifact. Putting the error type in the fence metadata preserves
that body.

Rustdoc is the closest operational precedent. It extracts documentation code
blocks, applies block-level attributes, wraps examples for testing, and has
special behavior for `?` in documentation tests. Veln should reuse the useful
shape, code-block metadata plus a generated wrapper, while making the error
type explicit with a small Veln-specific attribute when inference is not
enough.

Python doctest and Go examples reinforce the locality rule from another
direction: expected behavior is recorded adjacent to the executable example,
not in a distant manifest. Knuth's literate-programming argument points the
same way at a higher level: explanatory text, example code, and tool-owned
checking metadata should stay close enough that they age together. A fence
attribute gives agents a stable, local repair target: add, remove, or change
`error=...` on the block that contains the failing `?`.

A doc-comment directive would be more verbose and easier to detach from the
exact block it configures. A visible wrapper function would make examples less
copyable and would leak generated harness mechanics into documentation. A
fence attribute is the smallest explicit boundary that still gives the checker
and diagnostics a concrete source span.

## First-Slice Rules

- Executable Veln doctest fences start with `veln` as the first info-string
  token.
- The first slice recognizes the optional doctest info-string attribute
  `error=<TypePath>`.
- `error=<TypePath>` applies only to the fenced block where it appears.
- The attribute sets the generated wrapper return type to
  `Result((), <TypePath>)`.
- The attribute is harness metadata, not Veln source syntax, and must not be
  visible inside the generated example body.
- A doctest using `?` without an inferrable or contextual error type should
  receive a diagnostic that suggests adding `error=<TypePath>` to the fence.
- A doctest whose `error=<TypePath>` conflicts with a propagated `?` should
  report the conflicting propagation site and suggest an explicit conversion or
  a different fence error type.
- The first slice does not require hidden setup lines, expected-error modes, or
  expected-output syntax to share this attribute mechanism, but later doctest
  metadata should avoid conflicting with `error=`.

## Example Shape

The source documentation stays compact:

````markdown
/// ```veln error=ConfigError
/// let cfg = load_config("app.veln")?
/// assert_eq(cfg.name, "demo")
/// ```
pub fn load_config(path: String) -> Result(Config, ConfigError) effects fs
end
````

The checker treats the block as if the harness had generated:

```veln
fn __doctest_load_config() -> Result((), ConfigError) effects fs
  let cfg = load_config("app.veln")?
  assert_eq(cfg.name, "demo")
  Ok(())
end
```

## Open Details

The broader doctest info-string grammar remains intentionally small. Output
comparison is handled separately by
[Doctest Expected Output Syntax](result-doctest-expected-output-syntax.md).
Future decisions may add flags for hidden setup, negative examples, expected
runtime errors, or non-runnable examples. Those additions should be block-local
metadata and should not make the visible Veln example body carry test harness
ceremony.

The first slice also leaves exact parser recovery for malformed attributes
open. The minimum requirement is that diagnostics point at the fence
info-string and distinguish unknown doctest attributes from Veln type errors
inside the example body.

## Consequence

Veln gets an explicit doctest error-type boundary without making fallible API
examples noisier. Agents can repair ambiguous doctests by editing one local
metadata field, and users still see ordinary Veln code in the documented
example.

## References

- Hoffman, D., & Strooper, P. A. (2003). API documentation with executable
  examples. *Journal of Systems and Software*, 66(2), 143-156.
  https://doi.org/10.1016/S0164-1212(02)00055-9
- The Rustdoc Book contributors. (2026). *Documentation tests*.
  https://doc.rust-lang.org/rustdoc/documentation-tests.html
- Python Software Foundation. (2026). *doctest - Test interactive Python
  examples*. https://docs.python.org/3/library/doctest.html
- Gerrand, A. (2015). *Testable Examples in Go*.
  https://go.dev/blog/examples
- Knuth, D. E. (1984). Literate Programming. *The Computer Journal*, 27(2),
  97-111. https://doi.org/10.1093/comjnl/27.2.97
