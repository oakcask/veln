# Network Listener Address Metadata

Status: implemented

This record preserves the completed listener endpoint text slice from
`../../proposals/network-effect-integration-boundary.md`. Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-socket-listener-address/`,
`../../../examples/specification/run/transport-socket-production-listener-address/`,
`../../../examples/specification/run/transport-socket-listener-address-failure-json/`,
and
`../../../examples/specification/check/transport-socket-listener-address-effects/`.

## Outcome

The completed slice adds `net::listener_local_addr(listener)` as a
source-visible standard-library operation over a `NetListener`. The call
returns endpoint text as `String`, uses the existing coarse `net` effect
label, and leaves listener ownership unchanged. It does not expose host socket
handles, structured address records, integer ports, stream handles, or close
behavior.

Fixture-backed listeners report the listener address text. Production-loopback
listeners report the bound listener endpoint text when a host socket is
available and the configured listener address on the deterministic in-memory
loopback path. The helper can be called before accept work; later accepts and
accepted streams keep their existing ownership and lifecycle.

The checked run cases pin both fixture-backed and production-loopback paths by
reading the listener endpoint before accept work, then confirming accepted
stream endpoint inspection, stream reads, stream close, and listener close
still use the same ownership path. The failure case pins listener endpoint
metadata lookup failure as a runtime transport failure in run JSON. The checked
effect case pins the required `net` effect and descriptor provenance.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, richer stream routing, richer deadline and cancellation APIs,
channel and task ownership beyond the checked adapter slices, and HTTP/2
transport-adapter work.

## Read When

- Auditing why listener endpoint text inspection is no longer active proposal
  work.
- Checking completion evidence before changing `net::listener_local_addr`,
  listener ownership, or production-loopback listener endpoint reporting.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
