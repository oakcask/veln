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
For the checked slice, a carried single-entry dynamic table containing
`:path: /target` lets the indexed byte `0xbe` resolve to that header field and
advance the dynamic-core decode count.

The same indexed byte without a carried entry reports the focused
`hpack.fixture.dynamic_index_out_of_range` fact shape with the requested
dynamic index, the bounded dynamic table entry count, the inspected offset,
and the `hpack_dynamic_core` module name. It does not fall back to a generic
unsupported-header-block failure for this checked out-of-range dynamic index.

Unsupported HPACK forms, table-size behavior, dynamic-name continuations,
literal insertion into the broader fixture table, and full HPACK compression
remain outside this core slice.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the accepted source-visible `hpack_dynamic_core` dynamic indexed decode for
  a carried bounded entry and the focused out-of-range dynamic index failure
  facts when no dynamic entry is carried.
- The same case keeps the existing fixture-owned dynamic-table behavior and
  outbound HPACK fixture encoder coverage around the new source-visible core
  boundary.
