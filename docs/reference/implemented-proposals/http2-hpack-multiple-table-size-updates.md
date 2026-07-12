# HTTP/2 HPACK Multiple Table Size Updates

Status: implemented

This record preserves the completed consecutive dynamic table-size update
slice from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/execution.md` and the checked executable
cases under `../../../examples/specification/run/`.

## Completed Behavior

The source-visible HPACK fixture boundary accepts up to two dynamic table-size
updates at the start of one header block. It decodes both updates with the
existing HPACK integer boundary, applies them in wire order to the immutable
dynamic-table state, and decodes a following supported header field with the
final capacity and updated table contents. The checked boundary
retains an entry that fits the final capacity and resolves its dynamic index,
while a shrink followed by an expansion proves that wire-order eviction is
not reversed by the later update.

The HTTP/2 completed HEADERS and final CONTINUATION paths validate both
leading updates against the active local header-table receive limit before a
next HPACK state is installed. The first excessive update uses
`http2.peer_limit.header_table_size_exceeded`. Non-terminating integers and
updates after a header field retain the focused
`hpack.fixture.table_size_update_malformed` and
`hpack.fixture.table_size_update_not_at_start` diagnostics. A failed block
does not install any earlier or later state from the update sequence. A third
leading update is outside this bounded fixture vocabulary and uses the same
focused placement diagnostic. The existing single-update behavior is
preserved, and another same-shaped update-count extension is not a follow-up
target.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  two ordered updates, retained and evicted dynamic-index lookups under the
  installed capacities, the bounded third-update rejection, and
  malformed-sequence state preservation at the standalone fixture boundary.
- `../../../examples/specification/run/http2-protocol-core/` checks the same
  capacity-dependent behavior through completed HEADERS and final
  CONTINUATION, plus excessive updates at both checked positions, malformed
  and misplaced updates including a third leading update, and unchanged state
  after failure.
