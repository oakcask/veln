# Schema-Owned Dispatch Value Diagnostics

Status: implemented

This record preserves the completed generated schema dispatch diagnostic
reclassification slice from the schema binary pattern boundary proposal.
Current behavior is specified by `../../specification/execution.md`,
`../../specification/commands.md`, `../../specification/run-json.md`, and the
checked executable examples under `../../../examples/specification/run/`.

## Completed Behavior

Generated binary schema dispatch encode failures now use schema-owned ids when
the failed fact belongs to the schema representation:

- `schema.dispatch_unknown_tag` for a closed dispatch tag value with no matching
  payload case.
- `schema.dispatch_length_mismatch` for an extension dispatch payload whose
  encoded byte count differs from the schema-local length field.
- `schema.dispatch_mismatch` for an extension dispatch tag and supplied
  `SchemaDispatchPayload` variant that do not describe the same case.

The reclassification keeps the source-visible `EncodeError(id, field_path,
reason)` value shape, existing schema-local field path text, and focused human
and JSON command projection. Compatibility-only hand-written `EncodeError(...)`
values may still use the corresponding `codec.dispatch_*` ids when the error is
not produced by generated schema representation logic.

## Evidence

- `../../../examples/specification/run/binary-schema-dispatch-unknown-tag-encode-diagnostic-json/`
  and `../../../examples/specification/run/binary-schema-dispatch-unknown-tag-encode-diagnostic-human/`
  cover command-facing JSON and human projection for closed-dispatch unknown
  tags.
- `../../../examples/specification/run/binary-schema-dispatch-length-encode-diagnostic-json/`
  and `../../../examples/specification/run/binary-schema-dispatch-length-encode-diagnostic-human/`
  cover extension-dispatch payload length mismatches.
- `../../../examples/specification/run/binary-schema-dispatch-mismatch-encode-diagnostic-json/`
  and `../../../examples/specification/run/binary-schema-dispatch-mismatch-encode-diagnostic-human/`
  cover extension-dispatch tag/payload mismatches.
- `../../../examples/specification/run/binary-schema-closed-dispatch-encode-unknown-tag/`,
  `../../../examples/specification/run/binary-schema-extension-dispatch-encode-length-mismatch/`,
  `../../../examples/specification/run/binary-schema-extension-dispatch-encode-mismatch/`,
  and `../../../examples/specification/run/binary-schema-extension-dispatch-encode-tag-mismatch/`
  cover the source-visible `EncodeError` values produced by generated schema
  encode helpers.
- `../../../examples/specification/run/codec-dispatch-encode-result-compat-json/`
  covers compatibility-only command projection for hand-written
  `codec.dispatch_*` values.
