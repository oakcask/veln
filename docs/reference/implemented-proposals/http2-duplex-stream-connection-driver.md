# HTTP/2 Duplex Stream Connection Driver

Status: implemented

This record preserves the completed one-connection duplex-stream driver
proposal. Current behavior is specified by `../../specification/http2.md`,
`../../specification/names-effects.md`, `../../specification/execution.md`,
`../../specification/examples.md`, and the checked executable examples under
`../../../examples/specification/`.

## Completed Behavior

The standard `http2::connection` module exposes one server driver and one
client driver for a caller-owned `transport::DuplexStream`.

`drive_server(state)` sends the initial server SETTINGS bytes, then delegates
received chunks to the existing `http2::core` receive boundary. `drive_client`
writes the HTTP/2 client connection preface, writes the initial client
SETTINGS bytes produced by the existing local SETTINGS transition, then uses
the same receive loop and typed `Http2ConnectionFailure` result boundary.

Both drivers return a supplied closed lifecycle state without performing a
duplex-stream read, write, or protocol transition. Neither driver connects,
listens, accepts, closes, shuts down, retries, spawns, or converts host
transport failures into protocol failures. Installing
`transport::net::net_stream(stream)` around either driver replaces only the
nominal `transport::DuplexStream` effect with `net`; the caller retains
transport lifecycle ownership.

## Evidence

- `../../../examples/specification/run/http2-connection-client-initial-output/`
  checks that the client driver writes the client preface before the initial
  client SETTINGS bytes and before later core-produced output.
- `../../../examples/specification/run/http2-connection-tcp-loopback-client/`
  checks the client driver through the implemented
  `transport::net::net_stream` production-loopback handler and covers the
  client closed-entry no-effect boundary.
- `../../../examples/specification/run/http2-connection-server-split-preface/`,
  `../../../examples/specification/run/http2-connection-settings-ack/`,
  `../../../examples/specification/run/http2-connection-partial-frame/`,
  `../../../examples/specification/run/http2-connection-clean-end/`,
  `../../../examples/specification/run/http2-connection-truncated-end-json/`,
  `../../../examples/specification/run/http2-connection-protocol-failure-json/`,
  and `../../../examples/specification/run/http2-connection-closed-entry/`
  preserve the existing server-driver behavior.
- `../../../examples/specification/check/http2-connection-transport-handler-effects/`
  `../../../examples/specification/run/http2-connection-transport-handler-loopback/`,
  `../../../examples/specification/run/http2-connection-transport-handler-read-failure-json/`,
  and
  `../../../examples/specification/run/http2-connection-transport-handler-write-failure-json/`
  preserve the nominal duplex-stream handler effect and transport failure
  boundaries.

## Non-Goals Preserved

The completed driver does not add TLS, ALPN, URI discovery, authority
selection, connection pooling, retry policy, listeners, accept loops, task
spawning, deadlines, cancellation, high-level request routing, or new HTTP/2
frame, HPACK, stream, or flow-control semantics.
