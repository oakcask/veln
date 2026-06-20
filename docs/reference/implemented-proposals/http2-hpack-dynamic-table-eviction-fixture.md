# HTTP/2 HPACK Dynamic Table Fixture

Status: implemented

This record preserves the completed HPACK fixture dynamic-table slice from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The fixture HPACK boundary keeps HPACK state as an immutable ordinary source
value. A literal-with-indexing `:path: /target` block returns a next
`HpackFixtureState` with that dynamic entry, and a later `0xbe` indexed block
decodes the carried entry through that state. The same indexed block without a
prior dynamic entry remains unsupported.

Later literal-with-indexing blocks prepend bounded fixture dynamic-table
entries in newest-first order. After `:method: PUT` and `:scheme: https` are
inserted over `:path: /target`, dynamic indexed blocks decode `0xbe` as the
newest `:scheme: https` entry, `0xbf` as the second `:method: PUT` entry, and
`0xc0` as the third retained `:path: /target` entry. The bounded eviction
policy measures each accepted dynamic entry as header name byte count plus
value byte count plus `32` and evicts oldest entries first after insertion or
table-size updates. Reducing the fixture table size to `86` keeps the newest
two entries and evicts the third; reducing it to `42` keeps only the newest
supported `:method: PUT` entry when that entry is followed by
`:path: /target`; the same table size evicts a supported
`:authority: abc.test` entry because that accepted entry is larger than `42`.
Reducing the table size to `30` evicts both supported `:method: PUT` and
`:path: /target` entries, and later dynamic indexed representations for
evicted entries remain unsupported. The HTTP/2 protocol-core example carries
the generalized dynamic-table state and reduced table-size state through both
completed HEADERS and final CONTINUATION paths before decoding later dynamic
indexed blocks.

The same fixture boundary accepts dynamic table-size update bytes `0x3e`,
`0x3f`, `0x3f 0x01`, and the fixture-boundary slice of general multi-byte
HPACK integer continuations with the table-size update prefix, including
`0x3f 0x0b`, `0x3f 0x80 0x01`, `0x3f 0x81 0x01`, and
`0x3f 0x82 0x02`, returning immutable fixture states with table sizes `30`,
`31`, `32`, `42`, `159`, `160`, and `289`. The HTTP/2 protocol-core example
carries those updated states through completed HEADERS and final CONTINUATION
paths before later header blocks are decoded. Malformed non-terminating
table-size updates and
table-size updates with trailing bytes after a complete integer remain on the
unsupported fixture path.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  literal-with-indexing insertion, newest, second, and third dynamic indexed
  reads, missing dynamic state, accepted-entry-size eviction, full and partial
  reduced-table-size eviction failure paths, oldest-first eviction after a
  three-entry table-size reduction, and the fixture-boundary table-size update
  slice.
- `../../../examples/specification/run/http2-protocol-core/` checks the same
  carried immutable HPACK state across completed HEADERS and final
  CONTINUATION paths, including generalized dynamic indexed lookup,
  oldest-first table-size eviction, the accepted-entry-size eviction case, and
  the fixture-boundary table-size update slice.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
