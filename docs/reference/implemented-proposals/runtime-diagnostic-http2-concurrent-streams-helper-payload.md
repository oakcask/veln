# Runtime Diagnostic HTTP/2 Concurrent-Streams Helper Payload

Status: implemented

This record preserves the completed HTTP/2 concurrent-stream receive-limit
standard helper runtime diagnostic payload slice from the runtime diagnostic
payload proposal. Current behavior is specified by
`../../specification/run-json.md`, `../../specification/commands.md`,
`../../specification/execution.md`,
`../../specification/names-effects-full.md`, and checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

`http2_peer_limit_concurrent_streams_exceeded(...)` now returns
`Result<(), RuntimeDiagnostic>`. On failure it returns
`Err(RuntimeDiagnostic(...))` with
`RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(...)` carrying the byte
offset, stream id, attempted concurrent stream count, allowed concurrent
stream count, endpoint role, active state, receive-limit provenance, and rule
provenance, plus the inspected HEADERS frame-header bytes.

Command recording projects the HTTP/2 `details.protocol_diagnostic` JSON
object from the returned `RuntimeDiagnostic(...)` value. Human output uses the
same focused concurrent-stream receive-limit diagnostic and related notes as
historical aggregate evidence, including the nearby-byte preview note for
the inspected frame header. The helper no longer needs to register this
diagnostic through the message-keyed backend side-table bridge. The legacy
bridge remains available for unrelated helpers that are outside this slice.

## Evidence

- `../../../examples/specification/run/runtime-diagnostic-http2-concurrent-streams-helper-json/`
  checks that a direct call to
  `http2_peer_limit_concurrent_streams_exceeded(...)` returns a rendered
  `RuntimeDiagnostic(...)` result value and structured
  `details.protocol_diagnostic` fields.
- `../../../examples/specification/run/runtime-diagnostic-http2-concurrent-streams-helper-human/`
  checks that the direct helper payload keeps the focused human diagnostic and
  related notes.
- `../../../examples/specification/run/http2-protocol-core-concurrent-streams-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-concurrent-streams-json/`
  keep the existing public human and JSON command output stable.
- `../../specification/run-json.md`,
  `../../specification/commands.md`,
  `../../specification/execution.md`, and
  `../../specification/names-effects-full.md` summarize the implemented
  behavior and route readers to executable evidence.
