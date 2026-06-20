# HTTP/2 HPACK Static Indexed Fixture

Status: implemented

This record preserves the completed static indexed fixture slices from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture boundary accepts the one-byte static indexed header
block `0x81`. The decoded header list exposes that fixture as ordinary
header-list data: `HpackHeader(":authority", "")`, followed by the existing
`HpackHeader(":fixture", "static-indexed")` marker.

The same fixture boundary accepts the checked HPACK static table subset through
ordinary header-list data. The supported regular header-name entries include
`0xa4` `expires:`, `0xa5` `from:`, `0xa6` `host:`, the `if-*` names from
`0xa7` through `0xab`, `0xac` `last-modified:`, and the remaining checked
regular names through `0xbd` `www-authenticate:`. The transition advances the
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
  corrected `expires` entry, and the shifted later static indexed entries
  through `www-authenticate`.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  completed HEADERS frame cases named `hpack-static-indexed-authority`,
  `hpack-static-indexed-method-path`, `hpack-static-indexed-expires`,
  `hpack-static-indexed-from`, and the later entries through
  `hpack-static-indexed-www-authenticate`; the checked output emits `8284` for
  the two-header block, `a4` for `expires`, `a5` for `from`, and `bd` for
  `www-authenticate`.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
