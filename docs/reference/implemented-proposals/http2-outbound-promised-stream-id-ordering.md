# HTTP/2 Outbound Promised Stream ID Ordering

Status: implemented

This record preserves the completed server-side outbound `PUSH_PROMISE`
stream-identifier ordering slice. Current behavior is specified by
`../../specification/execution.md`, `../../specification/commands.md`,
`../../specification/run-json.md`, and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

The outbound `PUSH_PROMISE` connection state retains the greatest local
server-initiated promised stream id whose complete send intent was accepted.
The first and increasing ids advance that value. Repeated and lower ids use
`http2.protocol.peer_stream_id_not_increasing` with the attempted id, previous
high-water value, server endpoint role, active state, and rule provenance.

Stream-id domain, associated-stream lifecycle, peer
`SETTINGS_ENABLE_PUSH`, GOAWAY, HPACK, frame-size, and generated encoding
failures retain their focused paths. Ordering state advances only after those
checks and complete single-frame or split `PUSH_PROMISE`/CONTINUATION encoding
succeed. Rejection emits no output chunk and preserves HPACK, receive-credit,
peer-settings, shutdown, associated-stream, promised-stream lifecycle, and
ordering state. A rejected higher id remains eligible for a corrected retry.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks first,
  increasing, repeated, and lower ids; retention after close and reset;
  validation precedence; retry after rejection; HPACK rollback; split output;
  and full connection-state preservation.
- `../../../examples/specification/run/http2-protocol-core-outbound-promised-stream-id-ordering-human/`
  checks the focused primary message and related server notes.
- `../../../examples/specification/run/http2-protocol-core-outbound-promised-stream-id-ordering-json/`
  checks the source-visible runtime value and structured JSON fields.

## Non-Goals

Automatic stream-id allocation, transport behavior, full HPACK compression,
unbounded dynamic-table behavior, and a generic stream-id allocator remain
outside this completed slice.
