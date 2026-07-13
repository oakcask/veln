# HTTP/2 Automatic Outbound HPACK Huffman Selection

Status: implemented

This record preserves the completed automatic literal-string selection slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md` and the checked executable case under
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The production ordered header-list encoder builds both supported HPACK string
literal encodings and compares their complete byte counts, including the
length prefix. It selects Huffman only when that complete literal is smaller;
ties keep raw encoding for deterministic output.

The decision is independent for static-name and dynamic-name literal values
and for both the name and value of new-name literals. Exact static-indexed and
exact dynamic-indexed fields retain their existing priority. A
literal-with-indexing entry stores the decoded name/value pair, so later
blocks can reuse it through dynamic indexing regardless of the selected wire
encoding.

Encoding remains transactional across the complete header list and outbound
framing. A rejected string or later field exposes no partial header block,
frame output, dynamic-table update, or protocol-state update.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks
  Huffman-smaller and raw-tie values, independent mixed name/value choices,
  Huffman selection for a carried dynamic name, exact dynamic reuse, HEADERS
  framing, and state reuse after a later unsupported string.
- `../../specification/execution.md` summarizes the selection and transaction
  rules and routes readers to the executable evidence.
- The underlying ordered encoder is recorded in
  [http2-production-outbound-hpack-header-list-encoding.md](http2-production-outbound-hpack-header-list-encoding.md).
