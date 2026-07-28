# HTTP/2 Multi-Stream Outbound Flow Control

Status: implemented

This record preserves the completed multi-stream outbound flow-control slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md` and the checked executable case
historical aggregate evidence.

## Completed Behavior

The executable protocol core stores outbound stream send-credit states in a
list beside one shared connection send window. The representation can carry
any fixture-bounded number of active streams without adding fixed-arity
constructors. Each stream retains its own send window, open, closed, or reset
lifecycle state, and optional expected and observed `content-length` values.

Accepted DATA consumes shared connection credit and only the selected stream's
credit. It updates body accounting only for the selected stream and preserves
every unrelated stream fact. A received connection-level `WINDOW_UPDATE`
refills shared credit without changing stream facts; a stream-level update
refills only its matching stream. A received
`SETTINGS_INITIAL_WINDOW_SIZE` change applies its delta to every tracked open
stream, can leave individual stream credit negative, and leaves closed and
reset states unchanged. A later stream-level update can restore one negative
stream independently.

Zero increments, connection and stream overflow, unknown streams, closed
streams, and reset streams retain their focused rejection paths. Rejected
updates and DATA intents do not return changed credit.

## Evidence

- Historical aggregate evidence constructs three
  simultaneous outbound stream states through the list-backed API.
- The same checked case sends DATA independently on two streams, including one
  with `content-length` accounting, and checks the shared debit plus unchanged
  facts for the other streams.
- It refills one stream, refills the connection, applies an initial-window
  reduction to every open stream, preserves negative credit, and restores only
  the selected negative stream with a later update.
- It checks zero-increment, overflow, unknown-stream, closed-stream, and
  reset-stream rejection against the multi-stream representation while
  preserving unrelated stream facts.
