# Binary Schema Repeat Helper Bindings

Status: implemented

This record preserves the completed bounded repeat generated helper binding
slice for binary schemas from
`../../proposals/schema-declaration-surface.md` and
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode and encode helpers accept
`Repeat(count_field, Payload)`,
`Repeat(left_count - right_count, Payload)`,
`Repeat(left_count + right_count, Payload)`,
`Repeat(left_count * right_count, Payload)`, and
`Repeat(left_count / right_count, Payload)` when each count operand names an
earlier visible `Int` field. Payloads may be implemented byte-aligned unsigned
primitives, `ByteView(length_field)` with an earlier visible `Int` length
field, `ByteView(left_length + right_length)` with both operands naming
earlier visible `Int` fields, or eligible same-module or public imported
nested binary schemas resolved through written schema-aware paths.

Decode exposes repeated primitive fields as `List<Int>`, repeated
`ByteView(length_field)` and `ByteView(left_length + right_length)` fields as
`List<ByteView>`, and repeated nested schema fields as a list of the nested
schema's decoded record shape. Encode accepts the same list shapes and writes
each element through the generated helper path. Runtime failures keep the
repeated field path, append the repeated element index when the failed element
is known, and then append the nested schema field path for nested payload
failures.

## Evidence

- `../../../examples/specification/run/binary-schema-repeat-decode/` checks
  primitive repeat decode.
- `../../../examples/specification/run/binary-schema-repeat-add-decode/`,
  `../../../examples/specification/run/binary-schema-repeat-subtract-decode/`,
  `../../../examples/specification/run/binary-schema-repeat-product-decode/`,
  and `../../../examples/specification/run/binary-schema-repeat-quotient-decode/`
  check arithmetic count decode.
- `../../../examples/specification/run/binary-schema-repeat-byteview-decode/`
  checks repeated bounded `ByteView` decode.
- `../../../examples/specification/run/binary-schema-repeat-byteview-add-decode/`
  checks repeated additive-length `ByteView` decode.
- `../../../examples/specification/run/binary-schema-repeat-nested-decode/`
  checks same-module nested repeat decode.
- `../../../examples/specification/run/binary-schema-imported-repeat-nested-decode/`
  checks imported public nested repeat decode.
- `../../../examples/specification/run/binary-schema-repeat-truncated-json/`,
  `../../../examples/specification/run/binary-schema-repeat-truncated-human/`,
  `../../../examples/specification/run/binary-schema-repeat-byteview-truncated-json/`,
  `../../../examples/specification/run/binary-schema-repeat-subtract-negative-json/`,
  `../../../examples/specification/run/binary-schema-repeat-product-negative-json/`,
  and
  `../../../examples/specification/run/binary-schema-repeat-quotient-division-by-zero-json/`
  check repeat runtime failure diagnostics.
- `../../../examples/specification/run/binary-schema-repeat-nested-truncated-json/`
  checks same-module nested repeat truncation diagnostics.
- `../../../examples/specification/run/binary-schema-imported-repeat-nested-truncated-json/`
  checks imported nested repeat truncation diagnostics.
- `../../../examples/specification/run/binary-schema-repeat-encode/`,
  `../../../examples/specification/run/binary-schema-repeat-add-encode/`,
  `../../../examples/specification/run/binary-schema-repeat-subtract-encode/`,
  `../../../examples/specification/run/binary-schema-repeat-product-encode/`,
  and `../../../examples/specification/run/binary-schema-repeat-quotient-encode/`
  check primitive and arithmetic count repeat encode.
- `../../../examples/specification/run/binary-schema-repeat-byteview-encode/`
  checks repeated bounded `ByteView` encode.
- `../../../examples/specification/run/binary-schema-repeat-byteview-add-encode/`
  checks repeated additive-length `ByteView` encode.
- `../../../examples/specification/run/binary-schema-repeat-nested-encode/`
  checks same-module nested repeat encode.
- `../../../examples/specification/run/binary-schema-imported-repeat-nested-encode/`
  checks imported public nested repeat encode.
- `../../../examples/specification/run/binary-schema-repeat-encode-out-of-range/`,
  `../../../examples/specification/run/binary-schema-repeat-encode-count-mismatch/`,
  `../../../examples/specification/run/binary-schema-repeat-add-encode-count-mismatch/`,
  `../../../examples/specification/run/binary-schema-repeat-subtract-encode-count-mismatch/`,
  `../../../examples/specification/run/binary-schema-repeat-product-encode-count-mismatch/`,
  `../../../examples/specification/run/binary-schema-repeat-quotient-encode-count-mismatch/`,
  `../../../examples/specification/run/binary-schema-repeat-byteview-encode-length-mismatch/`,
  and
  `../../../examples/specification/run/binary-schema-repeat-byteview-add-encode-length-mismatch/`
  check repeat encode failures.
- `../../../examples/specification/run/binary-schema-repeat-nested-encode-failure/`
  checks nested repeat encode field paths for representation failures.

## Remaining Work

The broader binary schema primitives and dispatch proposal remains open for
recursive repeated nested schemas outside the existing helper eligibility
checks and mapping behavior outside the implemented structural slices.
