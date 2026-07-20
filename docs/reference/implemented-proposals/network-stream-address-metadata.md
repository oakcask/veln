# Network Stream Address Metadata

Status: implemented

This record preserves the completed accepted-stream endpoint text slice from
[external production socket runtime record](network-effect-integration-boundary.md).
Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-socket-stream-addresses/` and
`../../../examples/specification/run/transport-socket-production-stream-addresses/`.
Runtime failure classification is checked by
`../../../examples/specification/run/transport-socket-stream-address-failure-json/`.

## Outcome

The completed slice adds `net::stream_local_addr(stream)` and
`net::stream_peer_addr(stream)` as source-visible standard-library operations
over an accepted `NetStream`. Both calls return endpoint text as `String`, use
the existing coarse `net` effect label, and leave stream ownership unchanged.
They do not expose host socket handles, structured address records, or integer
ports.

Fixture-backed streams report the listener address as local endpoint text and
the accepted stream id as peer endpoint text. Production-loopback streams
capture accepted socket metadata at accept time and report local and peer
socket endpoint text through the same public helpers. Runtime metadata lookup
failures remain transport failures rather than protocol diagnostics.

The checked run cases pin both paths. The fixture-backed case accepts a stream,
prints local and peer endpoint text, and verifies the deterministic network
event log. The production-loopback case accepts a stream, prints both endpoint
texts, reads configured client bytes to prove the stream remains usable, and
verifies the production network event log.

The failure case forces endpoint metadata lookup failure after accept. The run
JSON surface reports a runtime failure with the transport address message, and
the event log stops after listen and accept.

## Read When

- Auditing why accepted-stream endpoint text inspection is no longer active
  proposal work.
- Checking completion evidence before changing stream address metadata,
  accepted stream ownership, or production-loopback endpoint reporting.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
