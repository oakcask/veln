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

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  literal-with-indexing insertion, newest and older dynamic indexed reads,
  missing dynamic state, and the reduced-table-size eviction failure path.
- `../../../examples/specification/run/http2-protocol-core/` checks the same
  carried immutable HPACK state across completed HEADERS and final
  CONTINUATION paths.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
