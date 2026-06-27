# Runtime Diagnostic Payloads

Status: implemented

This record preserves the completed runtime diagnostic payload migration.
Current behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`,
`../../specification/test-json.md`, and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

Runtime diagnostic details for command-facing fixture, byte, value, and
protocol failures are carried by the failing result value. Source-visible
diagnostics use ordinary `Err(RuntimeDiagnostic(...))` values. JVM runtime
helpers that preserve legacy public error values, such as plain `String` or
`EncodeError(...)` failures, carry their command-facing details with the
returned error value instead of registering those details in a message-keyed
side table.

The command recorder projects `details.fixture_hex`,
`details.byte_diagnostic`, `details.value_diagnostic`, and
`details.protocol_diagnostic` from the returned failure value. Plain
`Err(value)` failures that do not carry runtime diagnostic details remain
ordinary result failures. Public human output, JSON field names, diagnostic
ids, and result-value rendering stay stable.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-payload-byte-human/`,
  `../../../examples/specification/run/runtime-diagnostic-payload-byte-json/`,
  and `../../../examples/specification/run/runtime-diagnostic-payload-plain-json/`
  check source-visible byte payloads and plain result failures.
- The HPACK fixture and HTTP/2 protocol examples under
  `../../../examples/specification/run/` check source-visible
  `RuntimeDiagnostic(...)` payload projection for fixture and protocol
  families.
- Generated schema and byte/value helper cases under
  `../../../examples/specification/run/` check stable command output for
  legacy public error values whose diagnostic details are carried with the
  returned value.
- `../../../crates/veln-backend-jvm/src/tests.rs` checks direct JVM runtime
  result diagnostic recording without pre-registering message-keyed details.

## Archived Slices

Earlier incremental slices are archived alongside this record, including the
HPACK fixture payloads, generated encode value payload, generated schema
fixed-field payload, HTTP/2 payload/helper migrations, HTTP/2 side-table
cleanup, test JSON payload projection, and result-value trace projection
records.
