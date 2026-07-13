# HTTP/2 Peer-Created Stream ID Ordering

Status: implemented

This record preserves the completed server-side peer-created stream id
ordering slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md` and
`../../specification/run-json.md`.

## Completed Behavior

The receive connection state retains the greatest accepted client-initiated
stream id independently of the set of currently open streams. Accepting a
HEADERS frame that creates a new peer-created stream advances this value.
Closing or resetting that stream does not reduce it.

A later HEADERS frame for an untracked peer-created stream must use an id
greater than the retained value. Reuse of a tracked open, closed-by-peer, or
reset stream id bypasses the new-stream ordering check and follows the existing
stream lifecycle decision.

Frame-size, stream-id-domain, payload, HPACK, and completed header-list
validation precede ordering. For an otherwise valid untracked stream, ordering
precedes concurrent-stream and GOAWAY admission. A rejection preserves the
retained value and the stream, flow-control, HPACK, and shutdown states apart
from ordinary input-consumption semantics.

Ordering rejection uses
`http2.protocol.peer_stream_id_not_increasing`. Its structured facts include
the attempted stream id, previous greatest peer-created stream id, endpoint
role, active state, rule provenance, frame-header byte offset, and bounded
frame-header preview. Human output keeps the failed ordering fact primary and
places endpoint context, preview, state, provenance, and the repair direction
in related notes.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks first and
  increasing admission, tracked reuse, lower idle rejection before and after
  close or reset, repeated rejection, validation and admission precedence, and
  state preservation after rejection.
- `../../../examples/specification/run/http2-protocol-core-peer-stream-id-monotonicity-human/`
  checks the focused primary message and related human notes.
- `../../../examples/specification/run/http2-protocol-core-peer-stream-id-monotonicity-json/`
  checks the source-visible runtime detail and projected JSON fields.

## Remaining Scope

Local stream-id allocation, client receive ordering for promised streams, and
protocol behavior outside the checked server receive boundary remain in the
active HTTP/2 proposal.
