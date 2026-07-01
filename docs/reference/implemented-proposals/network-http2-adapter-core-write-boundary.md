# Network HTTP/2 Adapter Core Write Boundary

Status: implemented

This record preserves the completed HTTP/2 adapter/core write boundary slice
from `../../proposals/network-effect-integration-boundary.md`. Current
behavior is specified by `../../specification/execution.md`,
`../../specification/names-effects.md`, `../../specification/run-json.md`, and
the checked examples under
`../../../examples/specification/run/http2-adapter-core-write-boundary/` and
`../../../examples/specification/check/http2-adapter-core-write-boundary-effects/`.

## Outcome

The completed slice keeps application behavior as ordinary source values. A
pure handler returns HEADERS and DATA response actions without receiving a
`NetStream`, calling `net`, or owning transport writes. Adapter-owned code
translates those actions into the pure HTTP/2 core send-intent path and
accumulates only accepted output chunks.

The checked run case fixes the observable transport boundary. The adapter
accepts a fixture stream, sends response HEADERS through the core's HEADERS
framing path, sends DATA through the outbound credit and frame-size path that
splits the payload into ordered DATA frames, and writes the accepted chunks in
that order with `net::write_chunks`. A later DATA action after local
end-stream is rejected as an ordinary protocol decision and does not produce a
transport write for that rejected action.

The effect check keeps ownership explicit. Handler and pure core helper code
remain free of transport effects, while the adapter entry point that projects
accepted core chunks to `net::write_chunks` must declare the existing coarse
`net` effect. The slice adds no effect label, production socket API, schema
primitive, HPACK behavior, or binary primitive behavior.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs beyond the checked deterministic fixture and loopback adapter
shapes.

## Read When

- Auditing why HTTP/2 frame ordering, outbound DATA splitting, and transport
  write projection are no longer active proposal work for the current adapter
  boundary.
- Checking completion evidence before changing HTTP/2 send-intent adapter
  projection or `net::write_chunks` ownership.

## Skip Unless Needed

- Do not read this page for ordinary current HTTP/2 or transport behavior.
- Use the specification pages and checked examples for current behavior.
