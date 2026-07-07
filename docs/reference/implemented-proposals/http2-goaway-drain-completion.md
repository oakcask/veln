# HTTP/2 GOAWAY Drain Completion

Status: implemented

This record preserves the completed GOAWAY drain completion slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

Receiving or sending GOAWAY records the peer-visible or local last-stream
boundary and exposes graceful shutdown while an already-admitted stream at or
below that boundary is still active.

Once every tracked in-boundary stream is terminal, ordinary source exposes
`drained_shutdown` with the recorded last stream id and error code. Received
GOAWAY drain completion is covered after received HEADERS `END_STREAM`, DATA
`END_STREAM`, trailer HEADERS, and `RST_STREAM`. Locally sent GOAWAY drain
completion is covered after local outbound DATA `END_STREAM` and outbound
HEADERS `END_STREAM`.

When local GOAWAY narrows an already received boundary, the drain decision
uses that stricter boundary, so an active stream above the narrowed boundary
does not keep the connection in graceful shutdown.

After drain completion, a later peer-created HEADERS frame that would create
a new stream remains rejected with `http2.protocol.stream_after_goaway`
instead of reopening the graceful-shutdown state.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks that
  received GOAWAY with an active in-boundary stream stays in
  `graceful_shutdown`, then reaches `drained_shutdown` after received
  HEADERS `END_STREAM`, DATA `END_STREAM`, trailer HEADERS, or `RST_STREAM`.
- The same checked case verifies that locally sent GOAWAY stays in
  `graceful_shutdown` while the in-boundary stream is active, then reaches
  `drained_shutdown` after outbound DATA or HEADERS with `END_STREAM`.
- The same checked case verifies a received GOAWAY boundary narrowed by a
  local GOAWAY and exposes `drained_shutdown` from the stricter boundary.
- The same checked case rejects a later peer-created HEADERS stream after
  drain completion with `http2.protocol.stream_after_goaway`.
