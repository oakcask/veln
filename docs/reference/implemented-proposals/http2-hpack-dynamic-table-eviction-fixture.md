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

A later literal-with-indexing `:method: PUT` block becomes the newest bounded
fixture dynamic-table entry while the older `:path: /target` entry remains
addressable when the table has room. A following `0xbe` indexed block decodes
the newest `:method: PUT` entry, and `0xbf` decodes the older retained
`:path: /target` entry. Reducing the fixture table size below those entries
evicts them, and a later dynamic indexed representation remains unsupported.
The HTTP/2 protocol-core example carries the two-entry state through both
completed HEADERS and final CONTINUATION paths before decoding later dynamic
indexed blocks.

The same fixture boundary accepts dynamic table-size update bytes `0x3e`,
`0x3f`, `0x3f 0x01`, and the explicit multi-continuation fixture
`0x3f 0x81 0x01`, returning immutable fixture states with table sizes `30`,
`31`, `32`, and `160`. The HTTP/2 protocol-core example carries those updated
states through completed HEADERS and final CONTINUATION paths before later
header blocks are decoded.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  literal-with-indexing insertion, newest and older dynamic indexed reads,
  missing dynamic state, the reduced-table-size eviction failure path, and the
  explicit table-size update fixtures.
- `../../../examples/specification/run/http2-protocol-core/` checks the same
  carried immutable HPACK state across completed HEADERS and final
  CONTINUATION paths, including the explicit table-size update fixtures.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
