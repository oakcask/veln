# HTTP/2 Outbound DATA Flow Control

Status: implemented

This record preserves the completed outbound DATA send-window accounting slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md` and the checked executable case
historical aggregate evidence.

## Completed Behavior

Outbound DATA send-intents use peer-advertised limits for frames this endpoint
sends. `SETTINGS_MAX_FRAME_SIZE` provides the maximum DATA frame payload size,
and `SETTINGS_INITIAL_WINDOW_SIZE` provides the peer-owned stream send-window
credit. The executable protocol core tracks outbound connection credit and the
target stream credit separately from inbound receive-window state.

A DATA intent is accepted only when its full encoded DATA payload fits both
the connection send window and the target stream send window. Accepted DATA
emits one immutable output chunk containing one or more DATA frames, split by
the peer-advertised maximum frame size, and then consumes both send windows by
the full encoded DATA payload length. The checked boundary cases include
intents that exactly consume the connection window or exactly consume the
stream window.

If the connection send window has no available credit, or the target stream
send window has no available credit, the send-intent returns a stable
source-visible rejection and emits no output bytes. Larger over-window DATA
intents follow the same no-output rejection shape for the limiting window.
Valid received peer `WINDOW_UPDATE` frames refill the separate outbound
connection or stream send credit for later DATA send-intents. The refill path
is separate from local outbound `WINDOW_UPDATE` send-intents, which add
receive credit and do not make a previously over-window DATA intent fit.
Received peer `SETTINGS_INITIAL_WINDOW_SIZE` changes apply the delta from the
previous advertised value to the tracked open outbound stream send credit. A
lower value can make existing send credit negative and reject the same DATA
intent through the existing stream send-window flow-control shape before any
output bytes or credit changes. A later stream-level peer `WINDOW_UPDATE` can
restore enough credit for that DATA intent to emit bytes again, and a higher
advertised value raises existing send credit by the same delta.

PADDED DATA uses the same outbound credit accounting but counts the pad-length
byte and padding bytes as part of the encoded DATA payload. Padding that cannot
fit in the selected frame payload is rejected before bytes or credit changes.

## Evidence

- Historical aggregate evidence checks accepted
  DATA output and accepted split DATA output.
- The same checked case pins exact-boundary accepted cases for connection and
  stream send windows, then verifies the remaining credit reported by ordinary
  source output.
- The same checked case pins zero-credit connection and stream cases as
  rejected no-output outcomes.
- The same checked case sends valid peer connection-level and stream-level
  `WINDOW_UPDATE` frames after no-output DATA rejections, then verifies the
  matching later DATA intent emits bytes after send credit is restored.
- The same checked case sends a lower peer `SETTINGS_INITIAL_WINDOW_SIZE`
  after accepted outbound DATA, verifies the tracked open-stream send credit
  becomes negative, verifies the same DATA intent emits no bytes through the
  existing stream send-window rejection, then restores credit with a stream
  `WINDOW_UPDATE` and verifies the same DATA bytes are emitted again.
- The same checked case sends a higher peer `SETTINGS_INITIAL_WINDOW_SIZE`
  and verifies the tracked open-stream send credit increases by that delta.
- The same checked case applies the same SETTINGS transition to absent,
  closed, and reset outbound stream credit and verifies those shapes keep
  their existing later-DATA rejection behavior.
- The same checked case verifies a local outbound `WINDOW_UPDATE`
  receive-credit intent does not refill outbound DATA send credit.
- The same checked case keeps the existing PADDED DATA and split DATA
  send-intent coverage passing with the same output bytes.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the current behavior and route readers to the checked executable
  example.
