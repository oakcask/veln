# HTTP/2 Half-Closed-Local PRIORITY Receive

Status: implemented

This record preserves the completed half-closed-local PRIORITY receive slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md` and the checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

After local outbound DATA with `END_STREAM`, the receive core tracks that
stream as half-closed-local. A valid inbound PRIORITY frame for the same
stream is accepted rather than rejected by the half-closed-local DATA-only
stream-state route.

The accepted PRIORITY frame decodes the dependency stream id, exclusive flag,
and weight through the same source-visible frame shape used by the open-stream
and idle-stream PRIORITY receive paths. The tracked stream records those
priority facts while remaining half-closed-local, so a later inbound DATA frame
continues to use the existing half-closed-local DATA receive behavior.

The existing invalid PRIORITY boundaries are unchanged for stream id zero,
wrong payload length, self-dependency, closed-by-peer streams, reset streams,
and unrelated idle streams.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` records local
  `END_STREAM`, receives a valid PRIORITY frame on the resulting
  half-closed-local stream, and prints the decoded dependency stream id,
  exclusive flag, and weight.
- The same checked case prints the tracked priority facts after that frame,
  then accepts a later inbound DATA frame on the same stream with
  half-closed-local receive-window accounting.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the current behavior and route readers to the checked executable
  example.
