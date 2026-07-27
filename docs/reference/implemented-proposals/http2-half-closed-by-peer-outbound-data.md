# HTTP/2 Half-Closed-By-Peer Outbound DATA

Status: implemented

This record preserves the completed half-closed-by-peer outbound DATA
send-intent slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md` and the checked
historical aggregate evidence.

## Completed Behavior

After the ordinary receive core accepts DATA with `END_STREAM` on an open
stream, the tracked stream moves to the closed-by-peer receive state. That
state rejects later inbound DATA and stream-level inbound `WINDOW_UPDATE`
frames, but it does not close the local sending side.

The outbound DATA send-intent path can still send DATA on that stream while
local state remains open. It uses the peer-advertised outbound stream credit
from `SETTINGS_INITIAL_WINDOW_SIZE` and the peer-advertised maximum frame size
from `SETTINGS_MAX_FRAME_SIZE`, so payloads may still be split into multiple
DATA frames and charged against outbound connection and stream credit.

When outbound DATA from that state carries local `END_STREAM`, the outbound
receive-credit state records the stream as closed for later local
send-intents. Later outbound DATA and stream-level outbound `WINDOW_UPDATE`
for that stream use the existing closed stream-state rejection boundary.

## Evidence

- Historical aggregate evidence receives DATA
  with `END_STREAM`, prints the closed-by-peer stream state, successfully
  sends outbound DATA on that stream, and checks the split DATA frame bytes.
- The same checked case sends outbound DATA with local `END_STREAM` from that
  state, then rejects later outbound DATA and stream-level outbound
  `WINDOW_UPDATE` through the closed stream-state route.
- `../../specification/execution.md` summarizes the half-closed-by-peer DATA
  send-intent behavior and routes readers to the checked executable example.
