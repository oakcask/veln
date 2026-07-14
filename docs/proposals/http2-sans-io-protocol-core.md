# HTTP/2 Sans-I/O Protocol Core

Status: proposed

This proposal defines the remaining HTTP/2 protocol-core work used by the
binary schema design driver. Current behavior belongs under
`../specification/`; completed proposal history belongs under
`../reference/implemented-proposals/`.

## Problem

The design driver needs a concrete protocol target without becoming a
production web server. The remaining work should continue to exercise pure
state transitions over explicit byte input and output while avoiding another
inventory of already implemented slices.

## Scope

Planned work is limited to gaps not already specified by the executable HTTP/2
cases and current specification:

- remaining SETTINGS interactions and connection settings
- remaining DATA frame behavior and flow-control interactions
- remaining stream-identifier and stream-lifecycle rules
- remaining graceful-shutdown interactions
- typed protocol errors for uncovered frame, connection, and stream rules
- HPACK behavior beyond the current production ordered-list, octet-value,
  bounded dynamic-table, static-table, integer, and Huffman boundaries

Before selecting a slice from this list, compare the executable cases under
`../../examples/specification/run/http2-protocol-core/` with
`../specification/execution.md` and the implemented-proposal reference index.
Do not reselect behavior already present there.

## Design Constraints

### Limit Placement

Schema validation owns representation-local facts available from the current
decoded fields. Runtime settings own negotiated or configured peer limits.
Contracts own implementation invariants and must not replace peer protocol
errors for invalid incoming frames.

### Protocol Error Reporting

Pure transitions return typed protocol errors. Diagnostic conversion supplies
stable ids, byte offsets, focused primary messages, and structured related
context for frame, stream, state, setting, limit, and rule provenance.

### HPACK Boundary

Frame decoding keeps header blocks opaque until an explicit HPACK library
boundary consumes them with immutable state. Schema declarations must not
grow an HPACK-specific special case.

### Peer Limit Diagnostics

Peer-visible limit failures use the `http2.peer_limit.*` namespace. Malformed
representation facts keep their narrower schema or codec diagnostics when
they fail before the protocol core can evaluate negotiated state.

### SETTINGS State And Provenance

Locally enforced receive limits remain separate from settings advertised by
the peer for outbound behavior. Each active receive limit retains protocol,
configuration, or local-SETTINGS provenance. Received peer SETTINGS retain
their own item provenance and must not be cited as this endpoint's inbound
limit.

## Selection Stop Rule

A future target must name one uncovered protocol fact and its bounded state
transition. Extending a fixture count, stream-list width, table-update count,
or other same-shaped sequence is not a target; use the existing recursive or
list-backed abstraction instead.

## Non-Goals

- TLS, ALPN, socket listeners, or platform networking
- production throughput optimization
- encoding all protocol state rules inside schema declarations
- duplicating implemented behavior or completion history in this proposal

## Completion Criteria

- Executable cases cover valid input, invalid input, and state preservation
  for the selected uncovered rule.
- Pure transitions retain only undecoded suffix bytes after consumption.
- Protocol failures are typed and diagnostically structured.
- Current behavior is promoted to the smallest matching specification page.
- Completed history is archived under the implemented-proposal reference
  index and removed from this active proposal.
