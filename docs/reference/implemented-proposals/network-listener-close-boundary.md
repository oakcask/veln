# Network Listener Close Boundary

Status: implemented

This record preserves the completed explicit listener close boundary slice from
`../../proposals/network-effect-integration-boundary.md`. Current behavior is
specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-socket-listener-close-boundary/`,
`../../../examples/specification/run/transport-socket-production-listener-close/`,
`../../../examples/specification/run/transport-socket-listener-close-accept-until-failure-json/`,
`../../../examples/specification/run/transport-socket-listener-close-accept-until-cancellable-failure-json/`,
`../../../examples/specification/run/transport-socket-listener-close-failure-json/`,
`../../../examples/specification/run/transport-socket-listener-close-record-failure-json/`,
and
`../../../examples/specification/check/transport-socket-listener-close-effects/`.

## Outcome

The completed slice adds `net::close_listener(listener)` as a source-visible
standard-library operation over an adapter-owned `NetListener`. The call uses
the existing coarse `net` effect label, returns `()`, and records a listener
close event through the same runtime event log used by other transport
boundaries. It does not add an effect label, TLS, ALPN, application routing, or
HTTP/2 protocol-core state transitions.

Fixture-backed close records the listener close event in order. Already
accepted streams remain owned by their `NetStream` handles, so adapter code can
continue stream reads and stream cleanup after closing the listener. Later
`net::accept`, `net::accept_or_end`, `net::accept_until`, and
`net::accept_until_cancellable` calls on that listener fail through the runtime
transport boundary.

Production loopback close releases the owned listener resource, including the
host socket path or in-memory loopback listener state, without closing already
accepted streams. Accepted streams keep their independent read, write, and
close lifecycle.

Forced listener close failures and listener close event-recording failures
remain runtime transport failures. Their human-facing messages name listener
close as the failed transport action and include the host fixture reason.

The effect check keeps ownership explicit: source that closes a `NetListener`
must declare `net`, while pure handlers still receive ordinary event, state,
and action values without transport handles.

## Remaining Work

The broader network integration proposal remains open for richer production
socket APIs, richer stream routing, richer deadline and cancellation APIs,
channel and task ownership beyond the checked adapter slices, and HTTP/2
transport-adapter work.

## Read When

- Auditing why explicit adapter-owned listener close is no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
