# HTTP/2 Duplex Stream Connection Driver

Status: proposed

## Summary

Add one standard-library driver for one HTTP/2 server connection and one for
one HTTP/2 client connection. Each driver performs the proposed
`transport::DuplexStream` operations and delegates protocol transitions to the
existing pure `http2::core` modules.

The first executable adapter installs a `transport::DuplexStream` handler over
an existing TCP `NetStream`. It uses the existing production-loopback runtime
path. Unix domain sockets are not required by this proposal.

## Dependencies

This proposal depends on [Lexical Operation Handlers](../reference/implemented-proposals/lexical-operation-handlers.md).
It does not change the current HTTP/2 core while that dependency is absent.

Current network and HTTP/2 behavior remains specified by
`../specification/names-effects.md`, `../specification/execution.md`, and
`../specification/http2.md`.

## Module Boundary

The toolchain-owned `std` package adds these opt-in routes:

- `transport`: the nominal `DuplexStream` effect.
- `transport::net`: a handler declaration backed by one owned `NetStream`.
- `http2::connection`: the single-connection client and server drivers.

The proposed public shape is:

```veln
pub effect DuplexStream
	read_chunk() -> Option<ByteChunk>
	write_chunks(chunks: List<ByteChunk>) -> ()
end
```

```veln
pub fn drive_server(state: CoreConnectionState) -> Result<CoreConnectionState, Http2ConnectionFailure> effects [transport::DuplexStream]
	# Standard-library body omitted.
end

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
underlying `NetStream` lifecycle.

- The driver may perform stream reads and ordered chunk-list writes.
- The driver does not listen, connect, accept, or close a `NetStream`.
- The driver does not shut down either side of a `NetStream`.
- The caller installs `transport::net::net_stream(stream)` around one driver
  call.
- The caller closes the stream after the handled expression returns.
- A runtime transport failure retains the current abrupt runtime boundary. This
  proposal does not claim that source cleanup continues after that failure.
- The first slice runs one connection without spawning a task.

This boundary prevents a protocol failure from silently changing socket
ownership and keeps cleanup visible in the TCP adapter.

## Driver State Transitions

The transition table is the authoritative behavioral model. `state` always
means the immutable `http2::core` connection state supplied to the row.

| Current condition | Duplex-stream event | Core decision | Required output and next state |
| --- | --- | --- | --- |
| Client start | Driver entry | Initial client bytes accepted | Write client preface and initial SETTINGS in core-defined order; retain the returned state |
| Server start | Driver entry | Initial server bytes accepted | Write initial server SETTINGS in core-defined order; retain the returned state |
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

Current `net` operations report host read and write failures as runtime
transport failures. The `transport::net` handler preserves that behavior.
`Http2ConnectionFailure` represents protocol-owned ordinary failures only.

This proposal does not catch a host failure, convert it into a protocol
failure, or promise resumability after a partial host write. A later transport
error proposal may add value-returning host operations if concrete service
code requires recovery.

## Effect Boundary

The standard connection drivers expose only
`effects [transport::DuplexStream]`. Installing the TCP handler replaces that
nominal effect with `net`.

```veln
pub fn run_server_connection(stream: NetStream, state: CoreConnectionState) -> Result<CoreConnectionState, Http2ConnectionFailure> effects [net]
	handle http2::connection::drive_server(state) with transport::net::net_stream(stream)
end
```

The HTTP/2 core functions remain effect-free. Listener loops, deadlines,
cancellation, and concurrency remain outside this first connection boundary.

## Acceptance Model

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| Server split preface | Arbitrary chunk splits produce the same accepted core state as one complete chunk | `run/http2-connection-server-split-preface` |
| Client initial exchange | Client preface and SETTINGS precede later frame bytes | `run/http2-connection-client-initial-output` |
| SETTINGS acknowledgement | A valid initial peer SETTINGS produces one ordered ACK | `run/http2-connection-settings-ack` |
| Partial frame | No output is emitted until the core accepts the transition | `run/http2-connection-partial-frame` |
| Clean end | EOF with no pending protocol input returns the final core state | `run/http2-connection-clean-end` |
| Truncated end | EOF with pending input returns the existing narrow protocol failure facts | `run/http2-connection-truncated-end-json` |
| Protocol rejection | Failure preserves the input state and emits no output from the rejected transition | `run/http2-connection-protocol-failure-json` |
| TCP loopback server | A source-owned client and accepted server stream complete the initial exchange through the handler | `run/http2-connection-tcp-loopback-server` |
| TCP loopback client | A connected client stream consumes server initial bytes and emits client bytes through the handler | `run/http2-connection-tcp-loopback-client` |
| Effect replacement | The driver requires `DuplexStream`; the handled adapter requires `net` and not `DuplexStream` | `check/http2-connection-transport-handler-effects` |
| Runtime read failure | The command reports a transport runtime failure, not an HTTP/2 protocol failure | `run/http2-connection-tcp-read-failure-json` |
| Runtime write failure | The command reports a transport runtime failure and preserves prior accepted writes | `run/http2-connection-tcp-write-failure-json` |
| Closed entry | A supplied closed core state returns without invoking either duplex-stream operation | `run/http2-connection-closed-entry` |

The relative paths are planned directories below `examples/specification/`.
The pure split-input cases should compare public projections of core state and
output rather than backend object identity.

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

This proposal is complete when one server connection and one client connection
can run through the same nominal duplex-stream boundary, the TCP handler
replaces that effect with `net`, the transition rows have checked evidence,
and current behavior is promoted to the HTTP/2, effect, execution, and example
specification routes.
