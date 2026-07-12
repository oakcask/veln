# HTTP/2 Flow-Control Numeric Domain Types

Status: implemented

This record preserves the completed flow-control numeric domain-type slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md` and the checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The executable core distinguishes connection window credit, stream window
credit, configured initial window size, and received `WINDOW_UPDATE`
increments with ordinary Veln values backed by `Int`. Focused constructors and
accessors own the role-specific bounds: connection credit and configured
initial sizes range from zero through the HTTP/2 31-bit maximum, increments
range from one through that maximum, and current stream credit also permits
the negative range produced by a peer initial-window reduction.

The existing checked DATA debit and blocking paths, peer
`SETTINGS_INITIAL_WINDOW_SIZE` deltas, and connection- and stream-level
`WINDOW_UPDATE` refill and overflow paths use those domains. The transition
keeps command-facing output and diagnostic ids unchanged. Negative stream
credit continues to reject DATA through the focused stream-window fact, zero
increments keep their increment failure, and overflow keeps the matching
connection or stream window failure.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` directly checks
  accepted and rejected construction boundaries for every flow-control role.
- The same case applies an initial-window reduction that makes tracked stream
  credit negative, rejects DATA without output while credit is negative, and
  accepts the DATA after a stream-level refill.
- Connection- and stream-level received and outbound `WINDOW_UPDATE` cases pin
  successful refill and overflow rejection at the HTTP/2 maximum.
- The aggregate stdout assertion preserves the existing observable output and
  stable diagnostic ids across the domain-value transition.
