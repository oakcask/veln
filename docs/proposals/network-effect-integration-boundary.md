# Network Effect Integration Boundary

Status: proposed

This proposal tracks the remaining external production socket runtime beyond
the checked loopback boundary. Current network behavior, including structured
transport failures, is specified by `../specification/names-effects.md`,
`../specification/execution.md`, and `../specification/run-json.md`.

## Target: External Production Socket Runtime

Add an explicit production runtime mode that uses host sockets independently of
the deterministic loopback client and in-memory fallback.

- `net::listen` binds the requested endpoint or returns a transport failure; it
  does not start a synthetic client and does not silently replace a failed bind
  with an in-memory listener.
- `net::connect` may connect to an external endpoint that was not registered by
  the current process; it does not synthesize an in-memory stream for an
  unknown endpoint.
- Existing `NetListener` and `NetStream` ownership, clean-end, half-close,
  deadline, cancellation, address inspection, stale-handle, and structured
  transport-failure rules apply without exposing host socket objects to source
  code.
- The deterministic fixture and production-loopback modes remain available for
  executable specification cases and do not change semantics.

## Design Constraints

- Pure protocol functions do not perform `net`, `time`, or `concurrency`
  effects.
- Adapter functions declare the coarse effects required by their host calls.
- Clean stream end is an ordinary adapter-observable outcome; host I/O failure
  remains a structured runtime transport failure.
- A closed stream may cause the pure core to report an incomplete protocol
  fact, but transport context remains related context rather than replacing
  the schema, codec, HPACK, or HTTP/2 protocol diagnostic.

## Non-Goals

- TLS or ALPN
- moving socket or clock access into the pure protocol core
- finer-grained effect labels without an API that requires them
- HTTP application routing
- more helper arities, route-count examples, or same-shaped lifecycle fixtures

## Completion Criteria

- A checked external client can connect without an in-process listener binding,
  and a checked external listener can accept without a synthetic loopback
  client.
- Bind and connect failures do not fall back to in-memory transport.
- Executable cases cover ownership, ordered output, clean end, half-close,
  cancellation, reclamation, and structured failures through the external
  path.
- Existing source calls and coarse effects are reused unless implementation
  demonstrates a concrete missing capability.
