# Network Effect Integration Boundary

Status: implemented

This record preserves the completed external production socket runtime target.
Current behavior is specified by `../../specification/execution.md`,
`../../specification/names-effects.md`, and `../../specification/run-json.md`.

## Implemented Boundary

`VELN_NET_RUNTIME=external` makes `net::listen` bind a host listener without a
synthetic client and makes `net::connect` connect directly to the requested
host endpoint without consulting the runtime listener registry. Bind and
connection failures return structured transport failures without creating
in-memory handles. Fixture mode and `production-loopback` retain their existing
behavior.

Host sockets stay behind the existing `NetListener` and `NetStream` values.
The source API continues to use the existing endpoint inspection, ordered
read/write, clean-end, half-close, deadline, cancellation, state inspection,
and close calls under the coarse `net`, `time`, and `concurrency` effects.

## Completion Evidence

- JVM backend integration tests pair an external-mode Veln client with a host
  listener not registered by the runtime and an external-mode Veln listener
  with a host client not synthesized by the runtime.
- Those peers check ordered writes, clean stream end, request and response
  half-close, cancellation before a live stream read, and explicit stream and
  listener reclamation.
- The `transport-socket-external-connect-failure-*` and
  `transport-socket-external-listen-failure-*` executable run cases check human
  and JSON structured details. Missing identities and false ownership commit
  facts demonstrate that failures do not produce fallback handles.
- Existing fixture and `production-loopback` cases remain independent
  regression coverage.

## Non-Goals Preserved

The target did not add TLS, ALPN, HTTP routing, protocol-core networking,
fine-grained effect labels, or additional same-shaped lifecycle helpers.
