# Schema-Owned Encode Value Diagnostics

Status: implemented

This record preserves the completed encode-value diagnostic reclassification
slice from the schema binary pattern boundary proposal. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/commands.md`, `../../specification/run-json.md`, and
checked executable examples under `../../../examples/specification/run/`.

## Completed Behavior

Generated binary schema encode failures that reject a schema-local value,
repeat count, nested dispatch payload value, or length-bounded `ByteView` now
use `schema.encode_value_unrepresentable`. The reclassification keeps the
source-visible `EncodeError(id, field_path, reason)` shape, existing field path
text, count and length mismatch facts, and human and JSON command projection
details.

Codec-owned encode ids remain supported for hand-written codec values and
compatibility projections that are not produced by generated schema
representation logic. Dispatch ownership was completed by
[Schema-Owned Dispatch Value Diagnostics](schema-owned-dispatch-value-diagnostics.md).

## Evidence

- `../../../examples/specification/run/schema-encode-expression-unrepresentable-json/`
  and `../../../examples/specification/run/schema-encode-expression-unrepresentable-human/`
  cover direct explicit schema encode expressions.
- `../../../examples/specification/run/binary-schema-encode-value-diagnostic-json/`
  and `../../../examples/specification/run/binary-schema-byteview-encode-diagnostic-human/`
  cover generated helper and command-facing JSON and human projection.
- `../../../examples/specification/run/derived-codec-encode-boundary/`,
  `../../../examples/specification/run/derived-codec-byteview-product-boundary/`,
  and `../../../examples/specification/run/binary-schema-dispatch-nested-encode-failure/`
  cover derived codec wrappers, length-bounded `ByteView` facts, and nested
  dispatch payload encode failures.
