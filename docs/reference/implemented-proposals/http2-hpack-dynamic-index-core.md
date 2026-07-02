# HTTP/2 HPACK Dynamic Index Core

Status: implemented

This record preserves the completed source-visible dynamic indexed HPACK
decode slice from the HTTP/2 sans-I/O protocol-core proposal. Current behavior
is specified by `../../specification/execution.md`,
`../../specification/run-json.md`, `../../specification/commands.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The `hpack_dynamic_core` boundary owns the narrow dynamic indexed
header-field decode path for a bounded dynamic table supplied by the caller.
For the checked slice, a carried bounded dynamic table can contain multiple
entries. The indexed byte `0xbe` resolves to the newest carried entry,
`0xbf` resolves to the next older carried entry, and each accepted decode
advances the dynamic-core decode count. The boundary also accepts saturated
seven-bit indexed representation `0xff 0x00` as HPACK index `127`, resolving
dynamic table index `65` when the supplied bounded table carries that retained
entry.

An indexed byte that asks past the carried table reports the focused
`hpack.fixture.dynamic_index_out_of_range` fact shape with the requested
dynamic index, the bounded dynamic table entry count, the inspected offset,
and the `hpack_dynamic_core` module name without advancing state. It does not
fall back to a generic unsupported-header-block failure for this checked
out-of-range dynamic index.

Unsupported HPACK forms, dynamic-name continuations, literal insertion into
the broader fixture table, and full HPACK compression remain outside this
dynamic-index slice. Later source-visible dynamic-table accounting behavior is
recorded in
[HTTP/2 HPACK Dynamic Table Accounting Core](http2-hpack-dynamic-table-accounting-core.md).

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the accepted source-visible `hpack_dynamic_core` dynamic indexed decode for
  multiple carried bounded entries, the saturated seven-bit `0xff 0x00`
  indexed representation, decode-count advancement after each accepted decode,
  and the focused out-of-range dynamic index failure facts without state
  advancement when the requested entry is not carried.
- The same case keeps the existing fixture-owned dynamic-table behavior and
  outbound HPACK fixture encoder coverage around the new source-visible core
  boundary.
