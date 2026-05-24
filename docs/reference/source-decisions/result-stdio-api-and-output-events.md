# Discussion Result: Stdio API and Output Events

Status: implemented

## Picked Question

- What built-in `stdio` API signatures and test-visible output event shape
  should the first implementation provide?

## Decision

Use four narrow built-in output functions and one deterministic captured-output
event shape.

The first implementation should provide these built-ins:

```veln
pub fn stdio::print(text: String) -> () effects [stdio]
pub fn stdio::println(text: String) -> () effects [stdio]
pub fn stdio::eprint(text: String) -> () effects [stdio]
pub fn stdio::eprintln(text: String) -> () effects [stdio]
```

All four functions accept only `String`, return `()`, and carry the coarse
`stdio` effect. `print` and `println` target stdout. `eprint` and `eprintln`
target stderr. `println` and `eprintln` append one logical newline after
`text`; the logical newline is `\n` in captured test events even if a later
platform adapter writes a different process line ending.

`veln test` should capture each output operation as a structured event:

```json
{
  "kind": "stdio",
  "stream": "stdout",
  "operation": "println",
  "text": "hello",
  "terminator": "newline",
  "sequence": 1,
  "node_id": "call-12",
  "span": {
    "file": "src/main.veln",
    "start": {"line": 3, "column": 3},
    "end": {"line": 3, "column": 26}
  }
}
```

Required event fields are `kind`, `stream`, `operation`, `text`,
`terminator`, `sequence`, `node_id`, and `span`. `stream` starts with
`stdout` or `stderr`. `operation` starts with `print`, `println`, `eprint`, or
`eprintln`. `terminator` starts with `none` or `newline`. `sequence` is a
monotonic integer within the selected run or test case and orders stdout and
stderr events together. The event deliberately excludes timestamps, thread IDs,
process IDs, and rendered byte encodings from the first slice.

## Rationale

The existing observable I/O decision already requires built-in output under a
coarse `stdio` effect, while deferring source-level effect handlers. The
signature set above keeps that surface concrete without introducing formatting,
display protocols, byte streams, stdin, or user-defined handlers.

The type-and-effect literature supports treating output operations as
effectful leaves with known metadata. Talpin and Jouvelot's type-and-effect
discipline and Leijen's Koka design both separate ordinary result types from
effect information; Veln's first-slice public `effects [stdio]` boundary is the
small version of that idea. Plotkin and Pretnar's handler model supports the
implementation choice: `run` can install a real process handler while `test`
installs a capture handler, without exposing handler syntax to users yet.

The event shape is intentionally operation-oriented rather than reconstructed
stream-only text. Reconstructed streams are useful for simple comparisons, but
agent repair benefits from the source identity and operation provenance that
explain why output happened. Ko and Myers' Whyline work argues for debugging
interfaces built around questions about observed behavior; retaining
`operation`, `node_id`, `span`, and `sequence` gives Veln enough evidence to
answer "where did this output come from?" without storing a full execution
trace.

The Go examples precedent is useful for the first assertion model: small tests
often compare expected textual output. Veln should preserve that future path by
making logical stdout/stderr streams reconstructible from events, while keeping
the primary capture record structured enough for diagnostics and repair.

## First-Slice Rule

- The first stdio built-ins are exactly `stdio::print`, `stdio::println`,
  `stdio::eprint`, and `stdio::eprintln`.
- Each built-in takes one `String`, returns `()`, and has effect `stdio`.
- No first-slice stdio function accepts arbitrary values, formatting strings,
  byte buffers, file handles, or stdin.
- `println` and `eprintln` append one logical newline after the provided
  string.
- `veln run` maps stdout and stderr events to the process streams through the
  standard internal handler.
- `veln test` captures output as ordered `kind: "stdio"` events with the
  required fields listed above.
- Captured test events use source-relative file names in spans and no
  machine-local absolute paths.
- Captured events must be deterministic for the same source and test
  execution. Do not include timestamps or process-local identifiers in the
  first-slice event shape.
- A test runner may also expose reconstructed logical stdout and stderr text,
  but those streams are derived from the event list rather than a replacement
  for it.

## Open Details

The first slice does not define source syntax for asserting on captured output.
[Test Declaration Syntax](result-test-declaration-syntax.md) defines how test
cases are marked, but output assertions remain a separate decision. This
decision only requires the runtime and test runner to preserve enough
structured data for future assertions and failure reports.

The first slice does not decide whether later formatted output uses pure
formatting helpers, a display protocol, or overload-like resolution. Any later
extension should lower to the same four primitive output operations or a
compatible successor event shape.

The first slice does not decide stdin, terminal capabilities, colors, raw byte
I/O, or concurrent output interleaving beyond the single logical `sequence`
order observed by the handler.

## References

- Talpin, J.-P., & Jouvelot, P. (1994). The Type and Effect Discipline.
  *Information and Computation*. https://doi.org/10.1006/inco.1994.1046
- Leijen, D. (2014). *Koka: Programming with Row Polymorphic Effect Types*.
  arXiv:1406.2061. https://arxiv.org/abs/1406.2061
- Plotkin, G., & Pretnar, M. (2009). Handlers of Algebraic Effects.
  *Programming Languages and Systems*.
  https://doi.org/10.1007/978-3-642-00590-9_7
- Ko, A. J., & Myers, B. A. (2004). Designing the whyline: A debugging
  interface for asking questions about program behavior. *CHI 2004*, 151-158.
  https://doi.org/10.1145/985692.985712
- Gerrand, A. (2015). *Testable Examples in Go*.
  https://go.dev/blog/examples

## Consequence

The first implementation can ship runnable examples and deterministic
output-aware tests without committing to a full I/O library. Agents get a small
call surface, explicit public effect drift, and source-linked output evidence
for repair loops.
