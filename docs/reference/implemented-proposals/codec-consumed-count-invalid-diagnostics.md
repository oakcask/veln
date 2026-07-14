# Codec Consumed Count Invalid Diagnostics

Status: implemented

This record preserves the completed codec-owned consumed-count invalid slice
from `schema-and-protocol-diagnostics.md`. Current behavior is
specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

Hand-written decode boundaries validate `Decoded(value, consumed)` against the
supplied `ByteView`. A negative consumed count or a count larger than the view
length is projected as `codec.consumed_count_invalid` instead of succeeding or
becoming a retryable readiness failure.

Command-facing projection keeps the primary human message focused on the
invalid consumed-count fact at the reported byte offset. Related notes carry
the schema-local field path when available, the supplied view length, the
actual consumed count, the reason, and the source-visible
`DecodeErrorWithReason(...)` value. `run --json` projects the same id, byte
offset, field path, `field_path_display`, `available_count`,
`actual_consumed_count`, and reason into `details.byte_diagnostic`.

## Evidence

- `../../../examples/specification/run/codec-consumed-count-invalid-json/`
  checks the command-facing JSON projection for an oversized consumed count.
- `../../../examples/specification/run/codec-consumed-count-invalid-negative-json/`
  checks the command-facing JSON projection for a negative consumed count.
- `../../../examples/specification/run/codec-consumed-count-invalid-human/`
  checks focused human diagnostics and related notes.
- JVM runtime tests check that codec boundary validation creates structured
  `available_count`, `actual_consumed_count`, and reason details for oversized
  and negative consumed counts.
- `../../specification/run-json.md`, `../../specification/commands.md`, and
  `../../specification/execution.md` summarize the implemented behavior and
  route readers to executable evidence.

## Remaining Work

The broader schema and protocol diagnostics proposal remains open for
diagnostic ids and payloads outside the implemented codec-owned slices.
