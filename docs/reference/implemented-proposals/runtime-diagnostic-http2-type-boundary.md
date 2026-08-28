---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Runtime Diagnostic HTTP/2 Type Boundary

This record preserves the completed diagnostic-type slice of the HTTP/2
standard-library completion proposal. Current behavior is specified by
`../../specification/names-effects.md`,
`../../specification/names-effects.md`, and
`../../specification/run-json.md` and is checked by focused cases under
`../../../examples/specification/run/`.

## Completed Behavior

The private `std::diagnostic` module owns the generic runtime diagnostic
envelope, field-path types, and byte/value diagnostic types. Public prelude
type aliases preserve the established `RuntimeDiagnostic*` source spelling.

HTTP/2 and HPACK details are owned by `Http2DiagnosticDetail` and
`HpackDiagnosticDetail`. `RuntimeDiagnosticDetail` carries those inner values
through `RuntimeHttp2Diagnostic(...)` and
`RuntimeHttp2HpackDiagnostic(...)`. Raw result values expose this nesting;
human diagnostics and `details.protocol_diagnostic` retain their established
shape and precedence. Existing private JVM intrinsic link names remain
implementation details.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-header-table-helper-json/`
  checks the established outer aliases, an explicitly typed HTTP/2 inner
  detail, the nested raw value, and the unchanged structured projection.
- `../../../examples/specification/run/runtime-diagnostic-payload-hpack-dynamic-index-json/`
  checks the same boundary for HPACK details.
- Focused `runtime-diagnostic-http2-*-human/` and
  `runtime-diagnostic-payload-hpack-*-human/` cases retain primary messages and
  related context.
- CLI result-value parser tests distinguish both nested envelope constructors
  and preserve named access to inner detail fields.

## Related Cleanup Scope

HPACK codec completion, sans-I/O core migration, and monolithic fixture
retirement have moved to standard-owned HTTP/2 modules and focused executable
cases. The completed cleanup and item-level replacement evidence are recorded
by
[`http2-sans-io-protocol-core.md`](http2-sans-io-protocol-core.md); the retired
aggregate fixture is not current executable evidence.
