# HTTP/2 HPACK Static-Name Huffman Literals

Status: implemented

This record preserves the completed source-visible `hpack_static`
static-name Huffman literal promotion from the HTTP/2 sans-I/O protocol-core
proposal. Current behavior is specified by
`../../specification/execution.md` and checked by executable examples under
`../../../examples/specification/run/`.

## Completed Behavior

The standalone source-visible HPACK static decoder accepts bounded
literal-without-indexing, literal-with-indexing, and literal-never-indexed
header fields whose names resolve through the HPACK static table metadata and
whose values are Huffman-marked string literals. The value decoder now walks
the HPACK static Huffman table instead of matching only a fixed decoded-value
allowlist.

Accepted decoded values include visible ASCII, line feed, single-byte
non-visible labels such as `hpack-byte-ff`, and deterministic multi-byte
labels such as `hpack-bytes-00-ff`. Malformed Huffman padding, EOS-as-symbol,
malformed string lengths, and dynamic-table behavior remain outside this
source-visible static decoder slice and continue to use the existing fixture
or fallback boundaries.

## Evidence

- `../../../examples/specification/run/hpack-static-codec-boundary/` checks
  visible ASCII, line feed, `hpack-byte-ff`, and `hpack-bytes-00-ff` through
  the static-name Huffman path.
- The same case checks Huffman-marked values across
  literal-without-indexing, literal-with-indexing, and
  literal-never-indexed static-name forms.
- `../../../examples/specification/run/hpack-static-codec-boundary/` also
  preserves malformed raw-length fallback coverage so the promoted Huffman
  path does not absorb out-of-scope diagnostics.
