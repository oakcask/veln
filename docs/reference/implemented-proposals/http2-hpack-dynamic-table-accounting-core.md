# HTTP/2 HPACK Dynamic Table Accounting Core

Status: implemented

This record preserves the completed source-visible HPACK dynamic-table
accounting slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md`,
`../../specification/run-json.md`, `../../specification/commands.md`, and the
checked executable case
`../../../examples/specification/run/hpack-fixture-codec-boundary/`.

## Completed Behavior

The `hpack_dynamic_core` boundary exposes the HPACK dynamic entry size formula
as ordinary source behavior: header-name byte count plus header-value byte
count plus `32`. The checked slice keeps dynamic table state immutable, inserts
new entries newest-first, retains older entries while the supplied table-size
limit allows them, and evicts oldest entries first after insertion or
table-size reduction.

When a table-size reduction leaves room for only the newest retained entry, the
next dynamic indexed read still resolves that newest entry while the next older
index reports the focused dynamic-index out-of-range fact. When insertion of a
new entry would exceed the supplied table-size limit, the new entry is retained
and older entries are evicted oldest-first until the table fits. When the
inserted entry itself is larger than the supplied limit, the carried dynamic
table becomes empty.

This remains a bounded source-visible accounting core. It does not add full
HPACK string decoding, Huffman behavior, integer parsing beyond the existing
dynamic-index boundary, socket I/O, protocol-state transitions, full HPACK
compression, or unbounded dynamic-table behavior.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the `:path: /target` entry size, accepted insertion into a bounded table,
  retained newest and older entries, table-size reduction eviction,
  insertion-caused eviction, and over-limit insertion.
- The same case keeps the existing dynamic indexed decode evidence for
  multiple carried bounded entries, saturated seven-bit indexed representation,
  decode-count advancement, and focused dynamic-index lookup failures.
- `../../specification/execution.md`, `../../specification/run-json.md`, and
  `../../specification/commands.md` summarize the current behavior and route
  readers to the checked executable evidence.
