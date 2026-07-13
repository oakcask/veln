# HTTP/2 Client Promised Stream ID Ordering

Status: implemented

This record preserves the completed client receive ordering slice for
server-initiated promised stream ids. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The client receive connection state retains the greatest accepted promised
stream id independently of whether that promised stream is reserved, open,
closed, or reset. A valid `PUSH_PROMISE` that would reserve an untracked
promised stream must use an id greater than the retained value. Accepted first
and increasing promises advance the value; tracked reuse keeps its existing
promised-stream lifecycle failure.

Frame-size, stream-id-domain, payload-length, local disable-push, associated
stream state, HPACK, and promised request-header validation keep their focused
precedence. Ordering runs at the new promised-stream admission boundary before
reservation. Rejection uses
`http2.protocol.peer_stream_id_not_increasing` with the attempted promised id,
previous high-water value, client endpoint role, active state, rule provenance,
byte offset, and bounded frame-header preview.

Ordering rejection preserves promised-stream lifecycle, flow-control, HPACK,
settings, shutdown, and retained high-water state apart from ordinary
consumed-input state. A higher id rejected by header validation does not
advance the high-water value and remains eligible for a later valid retry.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks first and
  increasing single-frame and continued promises, lower untracked and repeated
  tracked ids, open, closed, and reset lifecycle retention, validation
  precedence, retry, and rejection-state preservation.
- `../../../examples/specification/run/http2-protocol-core-client-promised-stream-id-ordering-human/`
  checks the focused primary message and client receive related notes.
- `../../../examples/specification/run/http2-protocol-core-client-promised-stream-id-ordering-json/`
  checks the source-visible runtime value and projected JSON fields.

## Remaining Scope

Local stream-id allocation and automatic promised-stream-id selection remain
outside this completed receive boundary.
