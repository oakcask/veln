# Codec Integer Out-Of-Range Diagnostics

Status: implemented

This record preserves the completed codec-owned integer out-of-range slice
from `../../proposals/schema-and-protocol-diagnostics.md`. Current behavior
is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and
the checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

Direct source-visible `DecodeErrorWithReason(...)` result failures and
`DecodeStep::Invalid(DecodeErrorWithReason(...))` entry results preserve the
codec-owned `codec.integer_out_of_range` id, byte offset, field path, and
source-visible failure value.

When the reason uses the narrow form with `byte_width=<value>`,
`min_value=<value>`, `max_value=<value>`, `actual_value=<value>`, and
`reason=<text>`, `veln run --json` projects separate `byte_width`,
`min_value`, `max_value`, `actual_value`, and `reason` fields in
`details.byte_diagnostic`. Human `veln run` output uses
`integer out of range at byte offset ...` as the primary message and puts the
field path, byte width, accepted integer range, actual decoded value, failure
reason, and source-visible `DecodeErrorWithReason(...)` value in related
notes.

Plain reason strings still preserve the codec-owned id and reason without
inventing integer range facts.

## Evidence

- `../../../examples/specification/run/codec-integer-out-of-range-direct-json/`
  checks the direct `Result<_, DecodeError>` path for the codec-owned id,
  byte offset, field path, display path, byte width, accepted integer range,
  actual decoded value, and failure reason.
- `../../../examples/specification/run/codec-integer-out-of-range-direct-human/`
  checks the direct path through focused human diagnostics and related notes.
- `../../../examples/specification/run/codec-integer-out-of-range-step-json/`
  checks the `DecodeStep::Invalid(DecodeErrorWithReason(...))` entry path for
  the same structured JSON details.
- `../../../examples/specification/run/codec-integer-out-of-range-step-human/`
  checks the `DecodeStep::Invalid(...)` path through focused human diagnostics
  and related notes.
- `crates/veln-backend-jvm/src/tests.rs` checks that direct runtime result
  tracing extracts `byte_width`, `min_value`, `max_value`, `actual_value`,
  and `reason` from the source-visible reason text and leaves plain integer
  out-of-range reasons unstructured.
- `crates/veln-cli/src/commands/run.rs` checks the focused human projection
  from `details.byte_diagnostic`.

## Remaining Work

The broader schema and protocol diagnostics proposal remains open for
diagnostic ids and payloads outside the implemented codec decode
command-facing slices.
