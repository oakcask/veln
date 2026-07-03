# Network Client Connect Boundary

Status: implemented

This record preserves the completed source-visible client connect slice from
`../../proposals/network-effect-integration-boundary.md`. Current behavior is
specified by [names-effects.md](../../specification/names-effects.md),
[execution.md](../../specification/execution.md),
[examples.md](../../specification/examples.md), and the checked executable
examples named `transport-socket-connect-effects`,
`transport-socket-connect-boundary`,
`transport-socket-connect-failure-json`, and
`transport-socket-production-connect-lifecycle`.

## Outcome

The completed slice adds `net::connect(address)` as a source-visible
standard-library operation over a client-side address string. The call returns
an owned `NetStream`, uses the existing coarse `net` effect label, and reuses
the same endpoint inspection, read, write, write-side shutdown, and close
lifecycle as other stream handles.

Fixture-backed connect records a deterministic connection event, exposes the
requested address as peer endpoint text, and keeps read and write behavior on
the existing fixture stream path. Production-loopback connect returns a
deterministic client-side loopback stream, records production network events,
and captures client writes on stream close.

The static check case pins descriptor-backed `net` effect inference and
provenance for `net::connect`. Forced connection failure remains a runtime
transport failure. The boundary does not add TLS, DNS policy, structured
address records, finer-grained network effect labels, a service interface, or
a general async connect API.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, richer stream routing, richer deadline and cancellation APIs,
channel and task ownership beyond the checked adapter slices, and HTTP/2
transport-adapter work.

## Read When

- Auditing why source-visible client connect is no longer active proposal
  work.
- Checking completion evidence before changing `net::connect`, connected
  stream ownership, or production-loopback client stream behavior.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
