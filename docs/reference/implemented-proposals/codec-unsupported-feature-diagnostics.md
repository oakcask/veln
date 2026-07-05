# Codec Unsupported Feature Diagnostics

Status: implemented

This record preserves the completed codec-owned unsupported feature slice from
`../../proposals/schema-and-protocol-diagnostics.md`. Current behavior is
specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

Direct source-visible `DecodeErrorWithReason(...)` result failures and
`DecodeStep::Invalid(DecodeErrorWithReason(...))` entry results preserve the
codec-owned `codec.unsupported_feature` id, byte offset, field path, and
source-visible failure value.

When the reason uses `feature=<value>; reason=<text>`, `veln run --json`
projects separate `unsupported_feature` and `reason` fields in
`details.byte_diagnostic`. Human `veln run` output uses
`unsupported feature failed at byte offset ...` as the primary message and
puts the field path, unsupported feature, failure reason, and source-visible
`DecodeErrorWithReason(...)` value in related notes.

Plain reason strings still preserve the codec-owned id and reason without
inventing unsupported feature facts.

## Evidence

- `../../../examples/specification/run/codec-unsupported-feature-direct-json/`
  checks the direct `Result<_, DecodeError>` path for the codec-owned id, byte
  offset, field path, display path, unsupported feature, and failure reason.
- `../../../examples/specification/run/codec-unsupported-feature-direct-human/`
  checks the direct path through focused human diagnostics and related notes.
- `../../../examples/specification/run/codec-unsupported-feature-step-json/`
  checks the `DecodeStep::Invalid(DecodeErrorWithReason(...))` entry path for
  the same structured JSON details.
- `../../../examples/specification/run/codec-unsupported-feature-step-human/`
  checks the `DecodeStep::Invalid(...)` path through focused human diagnostics
  and related notes.
- `crates/veln-backend-jvm/src/tests.rs` checks that direct runtime result
  tracing extracts `unsupported_feature` and `reason` from the source-visible
  reason text and leaves plain unsupported feature reasons unstructured.
- `crates/veln-cli/src/commands/run.rs` checks the focused human projection
  from `details.byte_diagnostic`.

## Remaining Work

The broader schema and protocol diagnostics proposal remains open for
diagnostic ids and payloads outside the implemented codec decode command-facing
slices.
