# HTTP/2 GOAWAY Receive Lifecycle

Status: implemented

This record preserves the completed GOAWAY receive lifecycle slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable case `../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

Receiving GOAWAY on the connection stream with a valid eight-byte payload
records graceful-shutdown state with the peer-sent last stream id and error
code. The receive path keeps already-admitted peer-created streams whose ids
are less than or equal to that recorded last stream id on their ordinary
stream-state path.

For an already-admitted stream after received GOAWAY, DATA still consumes
connection and stream receive-window credit. Trailer HEADERS with
`END_STREAM` still complete HPACK fixture decoding and transition the stream
to closed-by-peer. Those accepted frames are ordinary stdout facts in the
checked `run --json` example, not `protocol_diagnostic` failures.

A later peer-created HEADERS frame that tries to create a stream above the
recorded last stream id keeps using the existing
`http2.protocol.stream_after_goaway` boundary with byte offset, stream id,
last stream id, graceful-shutdown state, endpoint role, and rule provenance.
Stream id domain failures and existing stream-state failures stay on their
narrower diagnostic routes.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` opens a
  peer-created stream, receives GOAWAY, then accepts DATA and trailer HEADERS
  on that already-admitted stream.
- The same checked case repeats that accepted path for an already-admitted
  stream below the recorded last stream id, not only at the boundary.
- The same checked case verifies receive-window accounting, emitted trailer
  header-block bytes, HPACK fixture decode output, and the closed-by-peer
  lifecycle state after the accepted trailers.
- The same checked case rejects a peer-created HEADERS stream above the
  recorded last stream id with `http2.protocol.stream_after_goaway`, including
  while graceful shutdown still has an active in-boundary stream.
- `../../specification/execution.md`, `../../specification/commands.md`, and
  `../../specification/run-json.md` summarize the current behavior and route
  readers to the checked executable example.
