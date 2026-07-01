# HTTP/2 SETTINGS ACK Send State

Status: implemented

This record preserves the completed outbound SETTINGS ACK send-state slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md` and the checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

After the pure connection transition receives a structurally valid non-ACK
peer SETTINGS frame with payload items, the connection state records one
pending outbound SETTINGS ACK intent. The pending state is independent from
the outstanding local SETTINGS batches that wait for peer ACKs.

Multiple valid peer SETTINGS frames received before the ACK intent is consumed
coalesce to one pending ACK. Consuming the intent returns an empty SETTINGS
frame with the ACK flag set and clears the pending ACK state. The peer
advertised settings values remain unchanged by ACK consumption.

This slice does not add transport I/O, socket effects, local SETTINGS ACK
correlation beyond the existing outstanding local batch queue, new SETTINGS
identifiers, HPACK behavior, stream lifecycle behavior, or flow-control
behavior.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks the
  pending ACK state after receiving peer SETTINGS, the emitted empty SETTINGS
  ACK frame, the cleared state after consumption, unchanged peer-advertised
  settings after consumption, and coalescing across multiple peer SETTINGS
  frames.
- `../../specification/execution.md` summarizes the current behavior and
  routes readers to the checked example.
