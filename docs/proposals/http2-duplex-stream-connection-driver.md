# HTTP/2 Duplex Stream Connection Driver

Status: proposed

## Summary

Add one standard-library driver for one HTTP/2 client connection. The driver
performs the implemented `transport::DuplexStream` operations and delegates
protocol transitions to the existing pure `http2::core` modules.

The reusable duplex-stream effect, the `transport::net::net_stream` handler,
and the server-side `http2::connection::drive_server` driver are current
behavior. This proposal now covers only the still-planned client driver that
uses that boundary.

## Dependencies

This proposal depends on [Lexical Operation Handlers](../reference/implemented-proposals/lexical-operation-handlers.md).
It builds on the implemented lexical handler boundary and does not change the
current HTTP/2 core.

Current network, duplex-stream, handler, HTTP/2 core, and server connection
driver behavior remains specified by `../specification/names-effects.md`,
`../specification/execution.md`, and `../specification/http2.md`.

## Module Boundary

The toolchain-owned `std` package still needs the client half of this opt-in
route:

The proposed public shape is:

```veln
pub fn drive_client(state: CoreConnectionState) -> Result<CoreConnectionState, Http2ConnectionFailure> effects [transport::DuplexStream]
	# Standard-library body omitted.
end
```

The snippets state the source-visible boundary. Final type names may reuse
existing public HTTP/2 core decision types when they already represent every
required outcome. The implementation must not add a second state model that
can disagree with `http2::core`.

## Ownership Boundary

The connection driver owns HTTP/2 protocol progress. The caller owns the
underlying transport lifecycle.

- The driver may perform stream reads and ordered chunk-list writes.
- The driver does not listen, connect, accept, or close a `NetStream`.
- The driver does not shut down either side of a `NetStream`.
- A TCP caller installs the implemented `transport::net::net_stream(stream)`
  handler around one driver call.
- A TCP caller closes the stream after the handled expression returns.
- A runtime transport failure retains the current abrupt runtime boundary. This
  proposal does not claim that source cleanup continues after that failure.
- The first slice runs one connection without spawning a task.

This boundary prevents a protocol failure from silently changing transport
ownership and keeps cleanup visible in the adapter.

## Driver State Transitions

The transition table is the authoritative behavioral model. `state` always
means the immutable `http2::core` connection state supplied to the row.

| Current condition | Duplex-stream event | Core decision | Required output and next state |
| --- | --- | --- | --- |
| Client start | Driver entry | Initial client bytes accepted | Write client preface and initial SETTINGS in core-defined order; retain the returned state |
| Open connection | `Some(chunk)` | Receive accepted | Write every core output chunk in order; continue with the accepted state |
| Open connection | `Some(chunk)` | Need more bytes | Write no bytes; continue with buffered state |
| Open connection | `Some(chunk)` | Protocol failure | Return the typed failure; write only bytes already accepted before the failing transition |
| Open connection | `None` | Core close accepted | Return the accepted final state without another read |
| Open connection | `None` | Pending preface, frame, or header block | Return a typed incomplete-input protocol failure and expose no successful next state |
| Draining connection | `Some(chunk)` | Receive accepted | Preserve core GOAWAY and stream-admission rules; write accepted output in order |
| Closed connection | Driver entry | No core decision requested | Return the supplied closed state without performing a duplex-stream read, write, or protocol transition |

The driver must use the existing core transitions for preface, initial peer
SETTINGS, frame buffering, HPACK state, stream state, flow control, GOAWAY,
and pending header blocks. The proposal does not redefine those rules.

## Transport Failure Boundary

Current `net` operations and the `transport::net` handler report host read and
write failures as runtime transport failures. `Http2ConnectionFailure`
represents protocol-owned ordinary failures only.

This proposal does not catch a host failure, convert it into a protocol
failure, or promise resumability after a partial host write. A later transport
error proposal may add value-returning host operations if concrete service
code requires recovery.

## Effect Boundary

The remaining standard client connection driver exposes only
`effects [transport::DuplexStream]`. Installing the implemented TCP handler
replaces that nominal effect with `net`.

```veln
pub fn run_client_connection(stream: NetStream, state: CoreConnectionState) -> Result<CoreConnectionState, Http2ConnectionFailure> effects [net]
	handle http2::connection::drive_client(state) with transport::net::net_stream(stream)
end
```

The HTTP/2 core functions remain effect-free. Listener loops, deadlines,
cancellation, and concurrency remain outside this first connection boundary.

## Acceptance Model

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| Client initial exchange | Client preface and SETTINGS precede later frame bytes | `run/http2-connection-client-initial-output` |
| TCP loopback client | A connected client stream consumes server initial bytes and emits client bytes through the handler | `run/http2-connection-tcp-loopback-client` |

The relative paths are planned directories below `examples/specification/`.

The implemented server-driver evidence lives in
`examples/specification/run/http2-connection-server-split-preface/`,
`examples/specification/run/http2-connection-settings-ack/`,
`examples/specification/run/http2-connection-partial-frame/`,
`examples/specification/run/http2-connection-clean-end/`,
`examples/specification/run/http2-connection-truncated-end-json/`,
`examples/specification/run/http2-connection-protocol-failure-json/`, and
`examples/specification/run/http2-connection-closed-entry/`. The implemented
transport-boundary evidence lives in
`examples/specification/check/http2-connection-transport-handler-effects/`,
`examples/specification/run/http2-connection-transport-handler-loopback/`,
`examples/specification/run/http2-connection-transport-handler-read-failure-json/`,
and
`examples/specification/run/http2-connection-transport-handler-write-failure-json/`.

## Non-Goals

- Do not add Unix domain socket calls.
- Do not add TLS, ALPN, URI discovery, or connection authority rules.
- Do not add an accept loop, connection pool, retry policy, or task scheduler.
- Do not add deadline or cancellation operations to `DuplexStream`.
- Do not expose `NetStream` to `http2::core` or to a pure application handler.
- Do not add request routing or a public high-level server/client facade.
- Do not change HTTP/2 frame, HPACK, stream, or flow-control behavior merely
  to fit the driver.

## Completion Boundary

This proposal is complete when one client connection can run through the
implemented nominal duplex-stream boundary and current client-driver behavior
is promoted to the HTTP/2, effect, execution, and example specification routes.
