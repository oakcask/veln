# Codec Hand-Written NeedEnd Boundary

Status: implemented

This record preserves the completed same-module hand-written codec
`NeedEnd` readiness slice from the superseded codec execution design.
Current behavior is specified by `../../specification/execution.md` and the
checked executable examples under `../../../examples/specification/run/`.

## Outcome

A codec declaration with a `decode with` clause exposes the codec item name as
an ordinary source call in the declaring module. The call forwards the
caller-supplied `ByteView` and `ByteOffset` to the referenced decoder
function.

When the referenced decoder returns `NeedMore(NeedEnd)`, the codec boundary
preserves that `DecodeStep` value unchanged. The boundary does not validate a
consumed byte count for `NeedMore`, and the command-facing closed-input
reporting path projects the returned value as `codec.incomplete_input` with
`need_end` readiness, matching direct `DecodeStep::NeedMore(NeedEnd)` entry
results.

## Evidence

- `../../../examples/specification/run/codec-decode-need-end-boundary-human/`
  checks a same-module hand-written `decode with` codec item call whose
  referenced decoder returns `NeedMore(NeedEnd)` and whose closed-input human
  diagnostic reports `codec.incomplete_input`.
- `../../../examples/specification/run/codec-decode-need-end-boundary-json/`
  checks the same codec item path through `run --json`, including the
  `details.byte_diagnostic` id, byte offset, empty field path, and `need_end`
  readiness.
- `../../../examples/specification/run/codec-decode-need-end-human/` and
  `../../../examples/specification/run/codec-decode-need-end-json/` check the
  direct `DecodeStep::NeedMore(NeedEnd)` entry-result projection used by the
  codec item path.

## Remaining Work

The source-level codec route is closed by
[Schema Binary Pattern Boundary](schema-binary-pattern-boundary.md). Current
schema work should use explicit schema operations and ordinary functions.
