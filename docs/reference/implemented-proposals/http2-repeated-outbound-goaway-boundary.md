# HTTP/2 Repeated Outbound GOAWAY Boundary

Status: implemented

This record preserves the completed repeated outbound GOAWAY send-intent
boundary from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

Outbound GOAWAY send-intents first validate the schema-declared eight-byte
GOAWAY payload fields. On an open connection, a valid intent emits the
nine-byte frame header plus payload and records local graceful-shutdown state.

After local graceful shutdown is already recorded, a repeated outbound GOAWAY
is accepted only when the requested last stream id preserves or narrows the
recorded local boundary. Accepted repeated GOAWAY send-intents emit ordinary
GOAWAY frame bytes and update the recorded local shutdown state to the sent
boundary.

A repeated outbound GOAWAY that would widen the recorded boundary is rejected
with `http2.protocol.stream_after_goaway` using local endpoint context. The
rejection emits no output bytes. Later local outbound HEADERS, DATA,
`PRIORITY`, stream-level `WINDOW_UPDATE`, and server-side `PUSH_PROMISE`
send-intents continue to use the recorded local GOAWAY boundary.

Generated schema encode-helper representation failures for the last stream id
or error-code payload remain encode errors before accepted bytes are produced.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks a first
  outbound GOAWAY send-intent, then repeated same-boundary and narrowed
  outbound GOAWAY send-intents with emitted frame bytes.
- The same checked case rejects a repeated outbound GOAWAY that would widen
  the recorded boundary as a no-output `http2.protocol.stream_after_goaway`
  failure.
- The same checked case verifies that later peer-created HEADERS plus local
  outbound HEADERS, DATA, PRIORITY, and server-side PUSH_PROMISE use the
  narrowed local GOAWAY boundary.
- `../../specification/execution.md`, `../../specification/commands.md`, and
  `../../specification/run-json.md` summarize the current behavior and route
  readers to the checked executable example.
