# HTTP/2 Outbound PUSH_PROMISE Enable-Push Setting

Status: implemented

This record preserves the completed server-side outbound `PUSH_PROMISE`
peer-setting slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md` and the checked
executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The outbound `PUSH_PROMISE` send-intent continues to accept a currently open
client-created associated stream, a server-initiated promised stream id, and
already-encoded header-block bytes when the peer has not disabled push. The
checked accepted case still emits one `PUSH_PROMISE` output chunk containing
the frame header, the generated promised-stream payload, and the header
block.

After the peer-advertised settings state contains `SETTINGS_ENABLE_PUSH = 0`,
the same otherwise valid send-intent is rejected before output chunks are
emitted. The rejection exposes the structured reason
`settings_enable_push_disabled` and active state `peer-settings`, so the
failed fact is tied to the peer setting rather than to stream id, HPACK, frame
size, flow control, or generated payload encoding.

This slice does not implement broader push lifecycle behavior, transport,
socket, TLS, ALPN, platform networking, full HPACK compression, or unbounded
dynamic-table behavior.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  `PUSH_PROMISE` output with no peer disable-push setting, rejected
  `PUSH_PROMISE` after peer `SETTINGS_ENABLE_PUSH = 0`, the empty output
  chunk list for that rejected case, and the structured rejection reason.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented behavior and route readers to the checked
  example.
