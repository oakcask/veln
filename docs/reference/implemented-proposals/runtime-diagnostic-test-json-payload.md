# Runtime Diagnostic Test JSON Payload

Status: implemented

This record preserves the completed top-level test JSON payload slice from the
runtime diagnostic payload proposal. Current behavior is specified by
`../../specification/test-json.md` and the checked executable case under
`../../../examples/specification/test/`.

## Completed Behavior

Top-level tests that return `Err(RuntimeDiagnostic(...))` keep the rendered
runtime diagnostic value in `cases.*.failure.details.value` and project the
contained detail into the same structured diagnostic objects used by
`veln run --json`.

The implemented evidence covers returned
`RuntimeDiagnostic(..., RuntimeByteDiagnostic(...))` and
`RuntimeDiagnostic(..., RuntimeValueDiagnostic(...))` values. The test report
keeps the rendered result value for result-value assertions. It carries byte
diagnostic ids, byte offsets, field paths, byte-count facts, readiness, and
bounded byte previews as structured JSON details. It also carries value
diagnostic ids, field paths, field-path displays, and reasons as structured
JSON details.

## Evidence

- `../../../examples/specification/test/runtime-diagnostic-payload-json/`
  checks that `veln test --json` preserves the rendered
  `RuntimeDiagnostic(...)` value and projects `details.byte_diagnostic` and
  `details.value_diagnostic` from returned values.
- `../../specification/test-json.md` routes the implemented test JSON behavior
  and names the executable evidence.
