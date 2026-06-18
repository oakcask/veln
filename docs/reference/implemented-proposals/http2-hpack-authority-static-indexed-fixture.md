# HTTP/2 HPACK Authority Static Indexed Fixture

Status: implemented

This record preserves the completed `:authority` static indexed fixture slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md`, `../../specification/examples.md`, and
the checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture boundary accepts the one-byte static indexed header
block `0x81`. The decoded header list exposes that fixture as ordinary
header-list data:
`HpackHeader(":authority", "")`, followed by the existing
`HpackHeader(":fixture", "static-indexed")` marker.

The transition advances the immutable `HpackFixtureState` through the same
path as the other static indexed fixtures. Unsupported bytes and unsupported
multi-byte indexed forms still project through
`hpack.fixture.unsupported_header_block`.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the focused `authority` decode, including the empty decoded value and the
  one-byte wire size.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  completed HEADERS frame case named `hpack-static-indexed-authority`, emits
  the header-block byte `81`, and prints the decoded `:authority` empty
  value.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
