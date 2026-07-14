# Codec Trailing Input Diagnostics

Status: implemented

This record preserves the completed codec-owned trailing input slice from
`../../proposals/schema-and-protocol-diagnostics.md`. Current behavior is
specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

Direct source-visible `DecodeErrorWithReason(...)` result failures and
`DecodeStep::Invalid(DecodeErrorWithReason(...))` entry results preserve the
codec-owned `codec.trailing_input` id, byte offset, field path, and
source-visible failure value.

When the reason uses
`consumed_count=<n>; available_count=<n>; remaining_count=<n>; reason=<text>`,
`veln run --json` projects separate count and reason fields if the counts are
nonnegative, remaining is positive, and consumed plus remaining equals
available. Human `veln run` output keeps the primary message focused on
trailing input and puts the field path, counts, reason, and source-visible
decode error in related notes.

Plain or malformed reason shapes preserve the codec-owned id and original
reason without inventing count facts. Decoders do not reject trailing input
automatically as part of this slice.

## Evidence

- `../../../examples/specification/run/codec-trailing-input-direct-json/` and
  `../../../examples/specification/run/codec-trailing-input-direct-human/`
  check the direct result path.
- `../../../examples/specification/run/codec-trailing-input-step-json/` and
  `../../../examples/specification/run/codec-trailing-input-step-human/`
  check the `DecodeStep::Invalid(...)` path.
- `../../../examples/specification/run/codec-trailing-input-malformed-direct-json/`
  and
  `../../../examples/specification/run/codec-trailing-input-plain-step-human/`
  check malformed and absent structured fields.
- `crates/veln-backend-jvm/src/tests.rs` checks structured extraction and
  rejection of inconsistent counts in runtime result tracing.
- `crates/veln-cli/src/commands/run.rs` checks the focused human projection.

## Remaining Work

The broader schema and protocol diagnostics proposal remains open for
diagnostic ids and payloads outside the implemented codec decode
command-facing slices.
