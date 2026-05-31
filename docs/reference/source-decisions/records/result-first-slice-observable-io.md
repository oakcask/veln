# Discussion Result: First-Slice Observable I/O

Status: implemented

## Picked Question

- Should the first slice include any I/O, and if so should it expose effect
  handlers as source-level language features or keep them internal?

## Decision

Include minimal built-in observable output in the first slice.

The first implementation should provide a built-in `stdio` module with output
operations such as `stdio::print`, `stdio::println`, and `stdio::eprintln`.
These operations are source-level functions, not special statements. They
carry the coarse `stdio` effect, and public functions that call them directly
or transitively must declare `effects [stdio]`.

Do not expose source-level effect handler syntax in the first slice. The
implementation should still represent built-in I/O internally as effect
operations routed through handlers. `veln run` should install the standard
handler that writes to process stdout and stderr. `veln test` should install or
compose a capture handler so tests and diagnostics can observe output without
depending on ambient process streams.

Standard input is deferred. The first slice needs observable output so runnable
programs can be evaluated, but input adds blocking behavior, EOF policy,
encoding questions, fixture design, and extra non-determinism. The internal
operation and handler representation should leave room for stdin later without
requiring a source-level handler design now.

## Rationale

Without observable output, the first slice can check pure functions and run
tests, but `veln run` cannot demonstrate meaningful small programs. That would
make the language difficult to evaluate as a runnable programming environment
and would under-test public effect declarations, effect provenance, runtime
execution, and output-facing examples.

Starting with built-in `stdio` keeps the first user-facing surface small. A
library design may become appropriate later, but early examples should not need
package setup, foreign binding syntax, or capability injection just to print a
line. Built-ins are also useful effect-inference leaves: the checker can trust
their metadata and produce stable missing-effect diagnostics.

Keeping handlers internal preserves a future path without making users learn a
handler language before the effect model has examples. Internally, treating
output as operations such as `stdio.write`, `stdio.write_line`, and
`stdio.write_error_line` gives `run`, `test`, diagnostics, and future tooling
one model for execution, captured output, and effect provenance.

## First-Slice Rule

- `stdio` is a first-slice coarse effect label.
- The first implementation provides built-in output functions under the
  qualified `stdio` namespace.
- `stdio::print` and `stdio::println` write to stdout. `stdio::eprintln` writes
  to stderr.
- These output functions accept strings in the first slice. General formatting,
  display traits, debug rendering, and overloaded printing are deferred.
- Public functions that may perform stdio output must include `stdio` in their
  `effects [...]` declaration.
- Private helpers may omit effect annotations; stdio output is inferred and
  propagated under the existing public-boundary effect rule.
- Missing stdio declarations produce ordinary `kind: "effect"` diagnostics
  grouped under the coarse `stdio` label.
- Contract expressions and hole `satisfy` predicates still reject I/O and
  effectful calls.
- Source code has no first-slice syntax for defining, installing, or handling
  effects.
- The interpreter or runtime IR should represent built-in stdio as effect
  operations routed through implementation-owned handlers.
- `veln run` uses a standard stdio handler connected to process stdout and
  stderr.
- `veln test` should be able to capture stdio output through a handler so tests
  can assert or report output deterministically.

## Example Shape

```veln
pub fn main() -> Result<(), AppError> effects [stdio]
  stdio::println("hello")
  Ok(())
end
```

If `main` declared `effects []`, `veln check` should report a missing `stdio`
effect at the public boundary and include a bounded provenance path to the
`stdio::println` call or its underlying `stdio.write_line` operation.

## Open Detail

The exact built-in signatures can stay narrow at first. The important first
contract is that output functions are known effectful operations with stable
metadata. A later formatting decision can add pure conversion helpers,
formatting functions, or a display protocol.

The exact test assertion surface for captured output is not decided here. The
runtime should preserve enough structured output events for the test command to
report or compare them once test syntax is specified.

This decision does not decide whether future stdin uses the same coarse
`stdio` label with advisory access metadata, a separate effect label, or a
more precise capability model. The first-slice source declaration remains
coarse.

## Consequence

First-slice programs can produce observable output while the public effect
contract remains explicit and small. The implementation gains an internal
effect-operation boundary that supports `run`, captured `test` output, and
future handler design without exposing effect handlers as user syntax too
early.
