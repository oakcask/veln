# HTTP/2 Initial Peer SETTINGS Gate

Status: implemented

## Scope

The sans-I/O receive transition now represents waiting for the first peer
SETTINGS frame separately from an established connection. The server path
enters this gate after the client connection preface, and the role-aware client
path exposes the same transition without requiring a client preface.

A complete non-ACK SETTINGS frame establishes the connection only after the
existing stream-id, payload-shape, item-range, role, and state-update checks all
succeed. A different first frame or SETTINGS ACK produces the typed
`http2.protocol.initial_peer_settings_required` failure. Incomplete or rejected
frames retain pending input and all connection state so callers may diagnose or
retry without observing a partial SETTINGS update.

## Evidence

- Integrated transition and state-preservation assertions:
  `examples/specification/run/http2-protocol-core/`.
- Structured diagnostic projection:
  `examples/specification/run/http2-initial-peer-settings-gate-json/`.
- Human diagnostic projection:
  `examples/specification/run/http2-initial-peer-settings-gate-human/`.
- Current behavior summary: `docs/specification/execution.md`.

## Non-Goals

This slice does not send the local connection preface or local initial SETTINGS
batch, and it does not change SETTINGS ranges, duplicate-item handling, ACK
coalescing, flow-control updates, TLS, ALPN, HPACK, or socket lifecycle.
