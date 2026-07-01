# HTTP/2 Outbound HPACK Dynamic-Name Indexed Literal

Status: implemented

This record preserves the completed outbound HPACK fixture slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture encoder accepts one fixture-scoped outbound
literal-with-indexing header list whose name is selected from the bounded
dynamic table and whose value is supplied as a fresh raw visible-ASCII literal.
The checked path first encodes a literal-with-indexing `:path: /target`
header list to insert a dynamic entry. A later `:path: /again` fixture header
list encoded from that returned state emits the HPACK dynamic-name
literal-with-indexing bytes `0x7e 0x06 "/again"` and returns another immutable
encode state that inserts `:path: /again` as the newest bounded dynamic entry.

Encoding `:path: /again` from that returned state emits the dynamic indexed
byte `0xbe`. Encoding the older `:path: /target` entry from the same returned
state emits `0xbf`, proving the previous entry remains retained under the
existing bounded table-size rules for this fixture.

The HTTP/2 protocol-core example feeds the returned HPACK encode state into
outbound HEADERS framing: the dynamic-name literal-with-indexing header block
is emitted as HEADERS bytes, the newly inserted `/again` entry is reused as
`0xbe`, and the retained older `/target` entry is reused as `0xbf`. The same
stateful helper path is also checked for server-side `PUSH_PROMISE`, where the
promised header block bytes enter the existing `PUSH_PROMISE` framing path.

Encoding the dynamic-name literal fixture without a matching dynamic-table
name remains a focused HPACK fixture failure. If the dynamic-table name is
present but the fresh value is not supported by the raw string fixture encoder,
the failure remains the focused `fixture raw string encoding` case.

This slice does not implement full HPACK compression, general dynamic-table
indexing, Huffman expansion, socket behavior, or production HTTP/2 behavior.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the stateful insertion, `0x7e 0x06 "/again"` dynamic-name literal bytes,
  returned encode-state reuse as `0xbe`, retained older-entry reuse as `0xbf`,
  and the unsupported fresh value failure.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  returned state through outbound HEADERS and server-side `PUSH_PROMISE`
  framing paths.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize the implemented fixture boundary and route readers to the checked
  examples.
