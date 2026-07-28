# HTTP/2 Flow-Control Numeric Domain Types

Status: implemented

This record preserves the completed flow-control numeric domain-type slice
from the HTTP/2 sans-I/O protocol-core proposal. Current public standard
module behavior is specified by `../../specification/http2.md`, the adjacent
standard tests, and the checked focused executable case
`../../../examples/specification/run/http2-core-flow-control-domains/`.

## Completed Behavior

The public `std::http2::core` facade distinguishes connection window credit,
stream window credit, configured initial window size, and received
`WINDOW_UPDATE` increments with ordinary Veln values backed by `Int`. Focused
constructors and accessors own the role-specific bounds: connection credit and
configured initial sizes range from zero through the HTTP/2 31-bit maximum,
increments range from one through that maximum, and current stream credit also
permits the negative range produced by a peer initial-window reduction.

The public debit and refill helpers return immutable accepted or rejected
credit decisions. Failure decisions preserve the caller's input credit and
increment while exposing the exact domain label, observed value, and accepted
bounds. The broader DATA debit, peer `SETTINGS_INITIAL_WINDOW_SIZE`, and
connection- and stream-level `WINDOW_UPDATE` integration paths still retain
their existing command-facing diagnostics and focused cases while the
remaining protocol-core state machine is migrated.

## Evidence

- `../../../crates/veln-stdlib/veln/http2/core_test.veln` checks accepted and
  rejected construction boundaries for every flow-control role.
- The same adjacent test checks connection and stream debit, connection and
  stream refill, overflow failure data, and input-credit preservation.
- `../../../examples/specification/run/http2-core-flow-control-domains/`
  checks the public facade projections from an external package.
- The remaining DATA and `WINDOW_UPDATE` integration cases keep their existing
  observable output and stable diagnostic ids while the wider state machine
  remains in migration.
