# Binary Schema Flag Decode Bindings

Status: implemented

This record preserves the completed visible flag bitset decode binding slice
from `../../proposals/schema-declaration-surface.md` and
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/execution.md` and checked executable
examples under `../../../examples/specification/run/`.

## Outcome

Generated `byte_decode_<schema>` and `byte_decode_step_<schema>` helpers accept
visible `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`, `Flag32be`,
`Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`, `Flag56be`,
`Flag56le`, `Flag64be`, and `Flag64le` binary schema fields.

Flag fields consume the same byte width and byte order as the matching unsigned
primitive, but produce source-visible flag bitset values rather than raw `Int`
fields. Short input reports the existing `schema.truncated_field` diagnostic
at the flag field path.

## Evidence

- The `binary-schema-flag16be-decode` case checks successful generated decode
  for a visible flag bitset field.
- The `binary-schema-flag16be-decode-step` case checks that the decode-step
  helper preserves the flag bitset value and reports the consumed byte count.
- The `binary-schema-flag16be-truncated-json` case checks the command-facing
  truncation payload for a short flag field.
- The broader `binary-schema-flag*-decode` examples check the supported flag
  widths and byte orders.

## Remaining Work

The broader schema declaration and binary schema proposals remain open for
field forms and mappings outside the implemented generated helper slices.
