# Runtime Diagnostic HTTP/2 Side-Table Cleanup

Status: implemented

This record preserves the completed cleanup of obsolete HTTP/2 runtime
diagnostic side-table registrations from the runtime diagnostic payload
proposal. Current behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, `../../specification/execution.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

HTTP/2 protocol and peer-limit command diagnostics are projected from returned
`Err(RuntimeDiagnostic(...))` values. The JVM runtime keeps the HTTP/2
`RuntimeDiagnosticDetail` projection code that derives
`details.protocol_diagnostic` from the returned value, and no longer keeps the
obsolete HTTP/2 message-keyed registration helpers that wrote equivalent
details into the legacy protocol side table.

The compatibility bridge remains available for unrelated fixture, value, and
generated-schema helpers that have not yet moved to returned runtime diagnostic
payloads.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-closed-helper-json/`
  and the other `runtime-diagnostic-http2-*-helper-json/` cases check direct
  standard helper payload projection.
- The HTTP/2 protocol-core JSON cases under
  `../../../examples/specification/run/` check value-carried
  `RuntimeHttp2Protocol...` and `RuntimeHttp2PeerLimit...` details with named
  result-value assertions.
- `../../specification/run-json.md`, `../../specification/commands.md`, and
  `../../specification/execution.md` summarize the implemented value-carried
  HTTP/2 diagnostic behavior and route readers to executable evidence.
