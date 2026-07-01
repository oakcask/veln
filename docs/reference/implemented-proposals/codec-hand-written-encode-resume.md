# Codec Hand-Written Encode Resume

Status: implemented

This record preserves the completed same-module hand-written codec encode
resume slice from `../../proposals/codec-execution-boundary.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

A codec declaration with an `encode with` clause exposes the codec item name
as an ordinary source call in the declaring module. The call accepts the
referenced encoder function's parameters and forwards them to that function.

The returned `EncodeStep<TState>` value is preserved unchanged. `Encoded`
carries complete output chunks, `Partial` carries the committed chunk list,
produced `ByteCount`, and next encoder state, and `Invalid` carries the
source-visible `EncodeError`.

For partial output, the caller owns the returned state. The same codec item can
be called again with that state, and ordinary source can observe the later
`Encoded(List<ByteChunk>)` completion.

## Evidence

- Hand-written codec tests check same-module `encode with` calls that observe
  `Encoded`, `Partial`, and `Invalid` outcomes. The partial path reads the
  committed chunks, produced count, and returned state, then passes that state
  to a later call to the same codec item and observes `Encoded` output.
- `../../../examples/specification/run/codec-encode-invalid-step-human/` and
  `../../../examples/specification/run/codec-encode-invalid-step-json/` check
  the command-facing projection when the hand-written codec returns
  `Invalid(EncodeError(...))`.
- The semantic lowering tests in `veln-sema` check that a same-module
  hand-written codec encode call resolves through the codec item name to the
  named encoder function.

## Remaining Work

The source-level codec route is closed by
[Schema Binary Pattern Boundary](schema-binary-pattern-boundary.md). Current
schema work should use explicit schema operations and ordinary functions.
