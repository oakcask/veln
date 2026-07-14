# Codec Owned Decode Invalid Id Diagnostics

Status: implemented

This record preserves the completed codec-owned decode invalid-input id slice
from `schema-and-protocol-diagnostics.md`. Current behavior is
specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

Hand-written `decode with` codec boundaries and direct source-visible
`Result<_, DecodeError>` failures preserve codec-owned
`DecodeErrorWithReason(...)` ids beyond the generic `codec.invalid_input`
path. Direct `DecodeStep::Invalid(DecodeErrorWithReason(...))` entry results
preserve the same codec-owned ids and reason details. The checked
packet-kind slice uses `codec.packet_kind_invalid` at the reported byte
offset after the decoder has read the rejected kind byte.

Command-facing projection keeps the primary human message focused on the
invalid decode fact at the byte offset. Related notes carry the schema-local
field path, plain reason, and source-visible `DecodeErrorWithReason(...)`
value. `run --json` projects the same id, byte offset, field path,
`field_path_display`, and reason into `details.byte_diagnostic`; helper-only
context fields stay absent unless the reason is a registered byte-helper
failure message.

## Evidence

- `../../../examples/specification/run/codec-decode-invalid-owned-id-json/`
  checks the hand-written codec item call path for
  `codec.packet_kind_invalid`, absolute byte offset, field path,
  `field_path_display`, plain reason, and absent helper-only fields.
- `../../../examples/specification/run/codec-decode-invalid-owned-id-human/`
  checks the same codec item path through focused human diagnostics and
  related notes.
- `../../../examples/specification/run/codec-decode-error-owned-id-direct-json/`
  checks the direct `Result<_, DecodeError>` path for the same codec-owned id,
  byte offset, field path, display path, and reason.
- `../../../examples/specification/run/codec-decode-error-owned-id-direct-human/`
  checks the direct path through focused human diagnostics and related notes.
- `../../../examples/specification/run/codec-packet-kind-invalid-direct-json/`
  and
  `../../../examples/specification/run/codec-packet-kind-invalid-direct-human/`
  check the target-named direct `Result<_, DecodeError>` path.
- `../../../examples/specification/run/codec-packet-kind-invalid-step-json/`
  and
  `../../../examples/specification/run/codec-packet-kind-invalid-step-human/`
  check the direct `DecodeStep::Invalid(DecodeErrorWithReason(...))` entry
  path.
- `../../specification/run-json.md`, `../../specification/commands.md`, and
  `../../specification/execution.md` summarize the implemented behavior and
  route readers to executable evidence.

## Remaining Work

The broader schema and protocol diagnostics proposal remains open for
diagnostic ids and payloads outside the implemented codec decode
`codec.invalid_input`, `codec.packet_kind_invalid`, and
`codec.consumed_count_invalid` command-facing slices.
