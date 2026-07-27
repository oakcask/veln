# HTTP/2 Outbound HPACK Static-Name Literal

Status: implemented

This record preserves the completed outbound HPACK fixture slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The HPACK fixture encoder exposes a source-visible static-name
literal-without-indexing helper. It resolves header names through the finite
HPACK static table metadata and emits literal-without-indexing bytes for raw
visible-ASCII values. Static-name indexes beyond the four-bit prefix boundary
use the same HPACK integer continuation shape as the checked decoder boundary.

The helper is intentionally bounded to static-table names and raw
visible-ASCII values. It does not choose between indexed and literal forms, it
does not add dynamic-table behavior, and it does not add Huffman encoding.
Names absent from the HPACK static table remain HPACK fixture encode failures
at this helper boundary.

The existing exact static-indexed encoder behavior is preserved: exact
name/value pairs with fixed static-table values still emit indexed-field bytes
before this literal helper is relevant. The protocol-core header-list encoder
continues to preserve the existing ordinary new-name literal path for outbound
HEADERS and `PUSH_PROMISE`; the static-name helper is used when a
literal-without-indexing fixture header list names an HPACK static-table
entry.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-json/` checks the
  focused source-visible helper boundary. It covers non-exact
  `:method: PUT`, ordinary static-name `server: ok` beyond the earlier
  checked outbound subset, the existing `:path: /target` subset, and a
  non-static name that remains an HPACK fixture encode failure.
- Historical aggregate evidence remains the
  aggregate HTTP/2 outbound HEADERS and `PUSH_PROMISE` regression route for
  fixture encoder changes.
