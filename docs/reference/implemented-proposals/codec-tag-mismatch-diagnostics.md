# Codec Tag Mismatch Diagnostics

Status: implemented

This record preserves the completed codec-owned tag mismatch slice from
`schema-and-protocol-diagnostics.md`. Current behavior is
specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

Direct source-visible `DecodeErrorWithReason(...)` result failures and
`DecodeStep::Invalid(DecodeErrorWithReason(...))` entry results preserve the
codec-owned `codec.tag_mismatch` id, byte offset, field path, and
source-visible failure value.

When the reason uses
`expected_tag=<value>; actual_tag=<value>; reason=<text>`, `veln run --json`
projects separate `expected_tag`, `actual_tag`, and `reason` fields in
`details.byte_diagnostic`. Human `veln run` output uses
`tag mismatch at byte offset ...` as the primary message and puts the field
path, expected tag, actual tag, failure reason, and source-visible
`DecodeErrorWithReason(...)` value in related notes.

Plain reason strings still preserve the codec-owned id and reason without
inventing tag facts.

## Evidence

- `../../../examples/specification/run/codec-tag-mismatch-direct-json/` checks
  the direct `Result<_, DecodeError>` path for the codec-owned id, byte
  offset, field path, display path, expected tag, actual tag, and failure
  reason.
- `../../../examples/specification/run/codec-tag-mismatch-direct-human/`
  checks the direct path through focused human diagnostics and related notes.
- `../../../examples/specification/run/codec-tag-mismatch-step-json/` checks
  the `DecodeStep::Invalid(DecodeErrorWithReason(...))` entry path for the
  same structured JSON details.
- `../../../examples/specification/run/codec-tag-mismatch-step-human/` checks
  the `DecodeStep::Invalid(...)` path through focused human diagnostics and
  related notes.
- `crates/veln-backend-jvm/src/tests.rs` checks that direct runtime result
  tracing extracts `expected_tag`, `actual_tag`, and `reason` from the
  source-visible reason text and leaves plain tag mismatch reasons
  unstructured.
- `crates/veln-cli/src/commands/run.rs` checks the focused human projection
  from `details.byte_diagnostic`.

## Remaining Work

The broader schema and protocol diagnostics proposal remains open for
diagnostic ids and payloads outside the implemented codec decode command-facing
slices.
