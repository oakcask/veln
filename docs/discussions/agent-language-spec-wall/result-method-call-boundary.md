# Discussion Result: Method Call Boundary

## Picked Question

- Should method call and function call both exist in the first slice, or should
  one be delayed to keep the parser and style simpler?

## Decision

Use function calls as the only general call form in the first slice. Delay
method calls until examples show that receiver-oriented APIs materially improve
the repair loop.

The first slice may still use `.` for record field access and, if needed, for
module qualification. It should not treat `value.name(args)` as a successful
method call. If the parser recognizes that shape, `veln check` should report a
targeted diagnostic that suggests the canonical function-call form.

## Rationale

Function calls and method calls are often only two surface spellings for the
same operation. Including both immediately would add formatter, parser,
resolver, documentation, and generated-example variance before Veln has proven
its core loop. It would also force early decisions about receiver lookup,
extension methods, method namespaces, mutable receivers, and conflict
resolution.

The first slice already has several agent-facing mechanisms that need stable
diagnostics: typed holes, `Result` propagation, contracts, effects, and JSON
output. Keeping calls uniform makes these diagnostics easier to explain. A hole
inside `parse_config(text, _)` has one obvious callee and argument list; the
method form would require the tool to explain whether the receiver is an
implicit first argument, a privileged object, or a namespace lookup.

Delaying method calls does not reject fluent code permanently. Left-to-right
data flow can be evaluated through the separate pipeline question, and real
examples can later show whether method syntax pays for its extra resolution
rules.

## First-Slice Rule

- General calls use `name(arg1, arg2)` or an explicitly qualified variant
  chosen by the module grammar.
- `value.field` may be used for field access when records are available.
- `value.method(args)` is not a first-slice call form.
- `veln fmt` must not rewrite function calls into method calls.
- `veln check` should produce a specific diagnostic for method-call-shaped
  syntax instead of a vague parse error when recovery can identify it.
- Standard-library examples should use function calls consistently until the
  method-call decision is reopened.

## Open Detail

The pipeline style decision now resolves the left-to-right data-flow question:
pipeline is preferred only for multi-step transformations over one subject. It
composes with the same underlying function-call model rather than depending on
method-call resolution.

The first-slice grammar resolves qualified calls as `module::name(args)`.
That keeps `.` available for field access and avoids treating qualification as
receiver-method dispatch.

## Consequence

The first implementation can keep call parsing and name resolution small while
still reserving room for method syntax if later examples show clear diagnostic
or readability benefits.
