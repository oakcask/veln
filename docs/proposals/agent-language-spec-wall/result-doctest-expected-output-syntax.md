# Discussion Result: Doctest Expected Output Syntax

Status: accepted-proposal
Implementation: partially implemented

## Picked Question

- What source syntax should executable doctests use for expected stdout and
  stderr output?

## Decision

Use an adjacent output fence after an executable Veln doctest block:

````markdown
```veln
stdio::println("subtotal: 42")
```
```veln-output stream=stdout
subtotal: 42
```
````

The output fence applies only to the immediately preceding executable `veln`
fence. The first slice recognizes `veln-output stream=stdout` and
`veln-output stream=stderr`. At most one output fence per stream may be
attached to a single doctest. If any output fence is present, streams without
an output fence are expected to be empty.

The comparison is over reconstructed logical stream lines, not raw bytes. The
test harness reconstructs stdout and stderr from captured stdio events,
normalizes line endings to `\n`, and compares the visible lines in each
`veln-output` fence to the corresponding stream. This deliberately avoids
making the ordinary Markdown closing-fence newline a raw-output assertion. If
a future test needs byte-exact output, final-newline sensitivity, or
stdout/stderr interleaving, it should inspect captured stdio events through
ordinary test assertions rather than extending first-slice doctest output
syntax.

Output comparison runs only after the generated doctest wrapper type-checks,
returns `Ok(())` when it has a `Result` wrapper, and ordinary assertions pass.
Returned errors, assertion failures, and contract failures remain primary test
failures; output mismatch is reported only for successfully completed examples.

Do not put Go-style `Output:` comments inside Veln source and do not use
Python-style transcript prompts for the first slice. The Veln example body
should remain copyable Veln code, while expected output stays block-local and
source-adjacent.

## Rationale

The doctest result already chooses generated wrappers so documentation examples
remain examples first. Hoffman and Strooper's executable-example work supports
that priority: an API example is useful as a partial specification when the
toolchain checks its behavior, but the human-facing example should still read
as documentation. A separate adjacent output block gives the checker expected
behavior without inserting harness comments into the Veln program.

Python doctest and Go examples provide the main operational precedents. Python
keeps expected output next to an executable example, but its prompt transcript
style would force Veln examples to look like REPL sessions rather than normal
source. Go keeps examples as ordinary code and records expected output with a
nearby `Output:` comment, but that works best in Go source files where comment
syntax is already part of the example convention. Veln documentation examples
are fenced Markdown blocks, so a neighboring `veln-output` fence preserves the
same locality while keeping the code fence pure.

Rustdoc reinforces the block-oriented model: documentation tests are extracted
from fenced code blocks, may have block-level metadata, and are run by a tool
owned harness. Veln should follow that shape rather than embedding output
expectations in source comments before the language has settled ordinary
comment and doctest grammar details.

The stdio event decision also matters. Since `veln test` captures
source-linked stdout and stderr events, output fences do not need to encode
event order or provenance themselves. Ko and Myers' Whyline work supports
keeping enough provenance to explain observed behavior; Veln can report an
output mismatch with the expected-output block span plus the source spans of
the events that produced the actual stream.

## First-Slice Rules

- A `veln-output` fence may attach only to the immediately preceding
  executable `veln` doctest fence.
- The required `stream` attribute is exactly `stdout` or `stderr`.
- A doctest may have at most one expected-output fence for stdout and at most
  one for stderr.
- If a doctest has no `veln-output` fence, captured output is allowed but not
  compared by doctest syntax.
- If a doctest has at least one `veln-output` fence, any stream without a fence
  is expected to be empty.
- Expected output is compared as reconstructed logical stream lines after
  normalizing line endings to `\n`.
- The first slice does not use output fences for byte-exact text, final
  newline assertions, stdout/stderr interleaving assertions, hidden setup,
  negative examples, expected runtime errors, or non-runnable examples.
- Output comparison runs after parse, type, contract, effect, assertion, and
  `Result` wrapper success checks for the doctest.
- Output mismatch diagnostics should include the stream, first differing line
  when available, the expected-output fence span, and bounded provenance from
  the stdio events that produced actual output.
- Unknown `veln-output` attributes should be reported as doctest metadata
  errors, not as Veln source-language errors.

## Example Shape

````markdown
/// ```veln error=OrderError
/// let order = parse_order("A-1,2")?
/// stdio::println(render_summary(order))
/// ```
/// ```veln-output stream=stdout
/// A-1 subtotal: 2
/// ```
pub fn parse_order(text: String) -> Result(Order, OrderError)
end
````

With both streams, the doctest stays block-local:

````markdown
```veln
stdio::println("ready")
stdio::eprintln("using default config")
```
```veln-output stream=stdout
ready
```
```veln-output stream=stderr
using default config
```
````

## Open Details

The current implementation extracts documentation comment `veln` fences and
compares adjacent `veln-output stream=stdout` and
`veln-output stream=stderr` fences in `veln test`. It also type-checks
generated doctest sources in `veln check`. Metadata diagnostics, duplicate
stream diagnostics, expected-error examples, hidden setup, ignored examples,
and non-runnable examples remain future work.

The first slice intentionally does not decide exact raw-output assertions. If
examples later show that final newline, byte encoding, or stream interleaving
matter in documentation, that should be handled as a separate extension over
captured stdio events.

The first slice also leaves expected-error examples, hidden setup, ignored
examples, and non-runnable examples for later doctest metadata decisions. They
should remain block-local and should not make the visible Veln example body
carry harness ceremony.

## Consequence

Veln gets checked output examples without turning examples into transcripts or
source-comment protocols. Agents can repair output examples by editing the
local `veln-output` block, while diagnostics can still route mismatches back
to the stdio calls that produced actual output.

## References

- Hoffman, D., & Strooper, P. A. (2003). API documentation with executable
  examples. *Journal of Systems and Software*, 66(2), 143-156.
  https://doi.org/10.1016/S0164-1212(02)00055-9
- Python Software Foundation. (2026). *doctest - Test interactive Python
  examples*. Python 3.14.5 documentation.
  https://docs.python.org/3/library/doctest.html
- Gerrand, A. (2015). *Testable Examples in Go*.
  https://go.dev/blog/examples
- The Rustdoc Book contributors. (2026). *Documentation tests*. The Rust
  Project. https://doc.rust-lang.org/rustdoc/documentation-tests.html
- Ko, A. J., & Myers, B. A. (2004). Designing the whyline: A debugging
  interface for asking questions about program behavior. *CHI 2004*, 151-158.
  https://doi.org/10.1145/985692.985712
