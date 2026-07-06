# HTTP/2 Outbound HPACK Dynamic Table Eviction

Status: implemented

This record preserves the completed outbound HPACK fixture encoder slice for
dynamic-table eviction after a reduced outbound table size. Current behavior
is specified by `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

The stateful outbound HPACK fixture encoder applies the current dynamic-table
capacity when a later literal-with-indexing encode inserts a fresh entry. A
table-size update first evicts any existing entries that no longer fit. A
subsequent literal-with-indexing encode then either keeps or discards its new
entry according to the reduced capacity, before the returned fixture state is
reused by later outbound HEADERS encoding.

The checked finite boundary uses `:method: PUT`, whose HPACK dynamic entry size
is exactly `42`. After a table-size update to zero, the fixture emits the
valid table-size update byte, clears retained entries, and later emits the
literal-with-indexing bytes for `:method: PUT` without retaining the entry;
encoding the same header list again emits the literal bytes again instead of a
dynamic indexed byte. After a table-size update to `30`, the same
literal-again behavior holds because the entry does not fit the reduced
capacity. After a table-size update to `42`, the same literal-with-indexing
encode retains the entry, and the following matching encode emits `0xbe`.

The HTTP/2 outbound HEADERS path consumes those returned fixture states before
frame splitting. The aggregate protocol-core case therefore observes the
zero-capacity literal-again behavior, the literal-again behavior at table size
`30`, and the dynamic-index reuse at table size `42` as encoded HEADERS output
chunks.

This remains fixture-scoped behavior. It does not implement full HPACK
compression, an unbounded dynamic table, or production header validation
beyond the checked fixture boundary.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  direct fixture encoder transitions for table size zero, table size `30`,
  and table size `42`, including the second encode that proves whether
  `:method: PUT` was retained.
- `../../../examples/specification/run/http2-protocol-core/` routes the same
  state transitions through outbound HEADERS, including the zero table-size
  update path, and checks the emitted frame chunks for the literal and
  dynamic-indexed outcomes.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize the implemented current behavior and route readers to the checked
  executable examples.
