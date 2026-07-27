# HTTP/2 SETTINGS Initial-Window Overflow

Status: implemented

This record preserves the completed initial-window overflow slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable case under
historical aggregate evidence.

## Completed Behavior

Applying a received `SETTINGS_INITIAL_WINDOW_SIZE` delta first checks every
open outbound stream send window. A result exactly at the HTTP/2 maximum is
accepted. If any result would exceed the maximum, the update returns the typed
`http2.peer_limit.flow_control_window_exceeded` failure and preserves the
original outbound send-credit state for the whole SETTINGS batch.

The diagnostic identifies the SETTINGS item offset, SETTINGS frame kind,
affected stream, attempted delta, remaining allowed credit, open-stream state,
and `settings_initial_window_size_stream_window` rule. Rejection preserves the
connection window, every stream window and lifecycle, and expected and observed
content-length accounting. Closed and reset streams remain unaffected.

## Evidence

- Historical aggregate evidence accepts an update that lands exactly at
  the maximum and rejects a one-step overflow.
- A multi-stream case places an adjustable content-length stream before a
  later overflowing stream and checks that neither stream is committed after
  rejection.
- The same case checks preserved connection credit, content-length accounting,
  closed-stream identity, reset-stream behavior, typed diagnostic id, item
  offset, frame kind, stream id, attempted delta, allowed credit, and rule.
