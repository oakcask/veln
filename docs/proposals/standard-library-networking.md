---
role: proposal
update-when: The planned network duplex adapter, public handler package-documentation support, or legacy compiler-known network compatibility removal changes.
---

# Remaining standard-library networking integration

## Outcome

The public `net::IO` contract and its host `net::system()` implementation are
current behavior in the
[standard-library networking specification](../specification/standard-library-networking.md).
This proposal now covers three independent follow-ups:

- adapt one caller-owned `net::Stream` to `transport::DuplexStream`;
- remove the legacy compiler-known `net::...` compatibility surface after
  checked sources use the exported standard module;
- publish public handler declarations through package documentation so
  `net::system()` is discoverable with the rest of the exported module.

None of these follow-ups changes the implemented system handler's ownership or
TCP resource lifecycle.

## Duplex transport adapter

The standard library will add a handler that captures one `net::Stream` and
implements `transport::DuplexStream` through `net::IO`. Reads will map
`net::ReadOutcome` into the transport read contract. Writes will use
`net::write_all` and map `NetError` through a separately decided transport
failure contract.

The adapter must not resolve, listen, accept, connect, shut down, or close the
captured stream. The caller retains lifecycle ownership. The failure mapping
must be decided before implementation because the current transport effect
does not expose a typed failure result.

## Compatibility removal

The compiler still recognizes legacy `net::...` spellings when no imported
source module owns `net`. An explicit `use net from "std"` already selects the
standard module and its nominal `net::IO` boundary.

Removal must first migrate checked standard-library and executable-example
sources. The compiler will then stop resolving the fallback spelling and emit
one migration diagnostic that identifies the standard import, `net::IO`, and
`net::system()` boundary. Compatibility is not a requirement for this
experimental project.

## Package documentation

The current [package-documentation catalog](../specification/package-documentation.md#published-boundary)
publishes public types, constructors, schemas, effects, aliases, and functions,
but excludes public handler declarations. It will add a handler declaration
kind with a stable semantic identity, canonical signature, documentation, and
`veln-doc:` resource. The checked standard-library bundle must then expose
`net::system()` from the exported `net` module without publishing its private
`prelude_builtin::net_system_*` adapters.

## Non-goals

This proposal does not add UDP, Unix-domain sockets, TLS, HTTP, proxies, packet
APIs, file-descriptor conversion, or platform-specific socket options. Those
features require separate contracts.

## Related work

The current [effects specification](../specification/effects.md) defines
nominal handler composition. Its `transport::DuplexStream` section demonstrates
the existing coarse host-network adapter but does not decide the typed
`NetError` mapping required here. Go's
[`net.Conn` contract](https://go.dev/src/net/net.go) informs the separation of
stream I/O from protocol code, but Veln must preserve explicit effect handling
and caller-owned lifecycle rather than copy Go's method model.

## Acceptance model

| Concern | Input or event | Required observation | Intended evidence |
| --- | --- | --- | --- |
| Duplex adapter | A captured `net::Stream` is handled as `transport::DuplexStream` and protocol code reads and writes | The adapter uses `net::IO` and `write_all`, maps failures through the decided transport contract, and does not close or take ownership of the stream | deterministic handler conformance test |
| Compatibility removal | Source uses a legacy compiler-known `net::...` spelling after checked-source migration | The fallback no longer resolves, and the migration diagnostic identifies the current import, effect, and handler boundary | check and diagnostic cases |
| Package documentation | The exported `std::net` module is rendered as package documentation | The catalog and MCP declaration resource include `net::system()` and exclude its private host adapters | package-documentation gate and checked standard-library resource bundle |

Add executable evidence before promoting any row into current specification.
Remove this page and its catalog entry after all three rows are implemented.
