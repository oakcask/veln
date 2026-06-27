# HTTP/2 HPACK Dynamic Name Continuation Diagnostics

Status: implemented

This record preserves the completed focused diagnostic slice for HPACK
dynamic-name continuation failures in the HTTP/2 sans-I/O protocol-core
proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The HPACK fixture boundary keeps bounded dynamic-name literal support inside
the fixture model. When a literal-with-indexing, literal-without-indexing, or
literal-never-indexed header block uses a saturated dynamic-name prefix and
then fails at the continuation boundary, the failure no longer collapses to
`hpack.fixture.unsupported_header_block`.

Missing dynamic-name continuation table entries report
`hpack.fixture.dynamic_name_continuation_missing`. Malformed continuation
integers report `hpack.fixture.dynamic_name_continuation_malformed`. Decoded
continuation indexes outside the bounded fixture table report
`hpack.fixture.dynamic_name_continuation_out_of_range`.

Each diagnostic preserves the header-block byte offset, observed size,
observed first byte, requested dynamic index when available, current bounded
dynamic table entry count, codec module, expected fixture, and bounded
header-block byte preview. The HTTP/2 protocol core projects those same
details when HPACK decode fails after a completed HEADERS block or after the
final CONTINUATION block of an assembled header block.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the direct fixture decode boundary for missing, malformed, and out-of-range
  dynamic-name continuation failures.
- `../../../examples/specification/run/hpack-fixture-dynamic-name-continuation-json/`
  checks the focused structured `run --json` projection for the missing
  dynamic-name continuation case.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  completed HEADERS path for all three focused ids and checks the final
  CONTINUATION path through the same projection helper.
- `../../../examples/specification/run/http2-protocol-core-hpack-dynamic-name-continuation-human/`
  checks the focused human diagnostic text and related context for the HPACK
  dynamic-name continuation projection used by the protocol-core boundary.
