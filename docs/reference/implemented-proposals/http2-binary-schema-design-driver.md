# HTTP/2 Binary Schema Design Driver

Status: implemented

## Outcome

The HTTP/2 design driver established and exercised the separation between
binary layout, incremental codec readiness, pure protocol state, diagnostic
projection, and effectful transport integration. Implemented behavior is
specified under `../../specification/` and checked by the HTTP/2, HPACK,
binary-schema, codec, and transport cases under
`../../../examples/specification/`.

## Continuing Routes

The broad driver is complete. Remaining work is deliberately split into
bounded proposal routes:

- [HTTP/2 Sans-I/O Protocol Core](../../proposals/http2-sans-io-protocol-core.md)
- [Schema Declaration Surface](../../proposals/schema-declaration-surface.md)
- [Network Effect Integration Boundary](../../proposals/network-effect-integration-boundary.md)

Future binary data, schema primitive, or diagnostic work needs a concrete
consumer and a new bounded proposal instead of reopening this design driver.

## Evidence

The implemented-proposal index routes to focused completion records for the
individual schema, codec, HPACK, HTTP/2 state, transport, and diagnostic
slices. Those records preserve rationale and completion evidence; they are not
the source of current behavior.
