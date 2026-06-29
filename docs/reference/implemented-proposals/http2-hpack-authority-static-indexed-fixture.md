# HTTP/2 HPACK Static Indexed Fixture

Status: implemented

This record preserves the completed static indexed fixture slices from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/`,
`../../../examples/specification/run/hpack-fixture-codec-json/`, and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture boundary accepts the one-byte static indexed header
block `0x81`. The decoded header list exposes that fixture as ordinary
header-list data: `HpackHeader(":authority", "")`, followed by the existing
`HpackHeader(":fixture", "static-indexed")` marker.

The same fixture boundary accepts every HPACK static indexed table entry from
`0x81` through `0xbd` through ordinary header-list data, including the
request pseudo-header entries, response `:status` entries, and all regular
header-name entries through `www-authenticate:`. The transition advances the
immutable `HpackFixtureState` through the same path for each static indexed
fixture.

The same fixture boundary also accepts the two-byte static indexed block
`0x82 0x84` as two consecutive supported static indexed representations. The
decoded header list preserves `HpackHeader(":method", "GET")` followed by
`HpackHeader(":path", "/")` as ordinary header-list data instead of replacing
the second slot with the fixture marker. Unsupported bytes and unsupported
multi-byte indexed forms outside the checked two-header static indexed slice
still project through
`hpack.fixture.unsupported_header_block`.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the focused `authority` decode, the two-header `method-path` decode, the
  complete static indexed table from `0x81` through `0xbd`, and the later
  dynamic-table interactions after those decodes.
- `../../../examples/specification/run/hpack-fixture-codec-json/` checks
  direct JSON-command decode of static indexed `0x85` as
  `:path: /index.html` while preserving the unsupported header-block
  diagnostic projection.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  completed HEADERS frame cases named `hpack-static-indexed-authority`,
  `hpack-static-indexed-method-path`, the full static table through
  `hpack-static-indexed-www-authenticate`, and a final CONTINUATION path for
  `hpack-static-indexed-path-index-continuation`; the checked output emits
  `8284` for the two-header block, `85` for the continuation block, and `bd`
  for `www-authenticate`.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
