# HTTP/2 Unknown Frame Preservation

Status: implemented

This record preserves the completed unknown-frame slice from the HTTP/2
sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable case
historical aggregate evidence.

## Completed Behavior

The ordinary-source protocol core decodes a structurally complete unknown
HTTP/2 frame type into `UnknownFrame(kind, flags, stream_id, payload)` after
the client preface gate and after the frame header and payload length are
structurally valid. The payload is a bounded `ByteView` over exactly the
decoded payload bytes, so the ordinary value preserves frame type, flags,
stream id, payload length, and payload byte contents.

Unknown HTTP/2 frame types do not report `schema.dispatch_unknown_tag`; closed
binary schema dispatch behavior remains unchanged for schema declarations that
use a closed dispatch field. State-machine ownership stays with the protocol
core: when an active continuation sequence requires CONTINUATION next, an
unknown frame is rejected with the existing continuation protocol-state
failure shape.

## Evidence

- Historical aggregate evidence includes an
  accepted unknown frame after the client preface gate, a direct frame-state
  accepted unknown frame that preserves payload bytes `170`, `187`, and `204`,
  and an unknown frame rejected while continuation state is active.
- `../../specification/execution.md` routes the HTTP/2 protocol-core behavior
  and states that structurally complete unknown extension frame types decode
  to ordinary `UnknownFrame` values.
- `../../specification/run-json.md` routes the same protocol diagnostic family
  for command-facing JSON output.
