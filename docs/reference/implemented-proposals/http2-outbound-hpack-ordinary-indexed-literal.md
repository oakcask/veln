# HTTP/2 Outbound HPACK Ordinary Indexed Literal

Status: implemented

This record preserves the completed outbound HPACK fixture slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and checked by
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
historical aggregate evidence.

## Completed Behavior

The imported HPACK fixture encoder accepts one source-visible ordinary
new-name literal-with-indexing path for visible-ASCII field-name and value
pairs that already pass the outbound ordinary header-name validation boundary.
The checked `x-trace: ok` header list emits HPACK bytes
`0x40 0x07 "x-trace" 0x02 "ok"` and returns an immutable encode state whose
bounded dynamic table contains that entry.

Encoding the same checked header list from that returned state emits dynamic
indexed byte `0xbe`, proving the inserted ordinary field can be reused without
requiring a fixture marker header. The HTTP/2 protocol-core example feeds both
returned encode transitions through outbound HEADERS framing: the first
HEADERS frame carries the literal-with-indexing header block, and the later
HEADERS frame carries the dynamic indexed header block.

Unsupported ordinary names remain outside this slice. The checked invalid
`bad@name: ok` literal-with-indexing header list stays on the HPACK fixture
header-list encode failure path before HEADERS bytes are emitted.

This slice does not implement full HPACK compression, Huffman value selection
for outbound ordinary literals, unbounded dynamic-table behavior, socket
behavior, or production HTTP/2 behavior.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the literal-with-indexing bytes, returned encode-state reuse as `0xbe`, and
  the invalid ordinary-name failure.
- Historical aggregate evidence checks the
  literal and dynamic indexed header blocks after outbound HEADERS framing and
  keeps the invalid ordinary-name encode failure before output bytes.
- `../../specification/execution.md` summarizes the implemented fixture
  boundary and routes readers to the checked examples.
