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
The HTTP/2 protocol-core example also inserts raw new-name
literal-with-indexing `x-trace: ok`, carries the returned immutable dynamic
entry state through a later `0xbe` lookup, then reduces the table to `40` so
that ordinary entry is evicted and the next `0xbe` lookup stays unsupported.

Later literal-with-indexing blocks prepend bounded fixture dynamic-table
entries in newest-first order. After `:method: PUT` and `:scheme: https` are
inserted over `:path: /target`, dynamic indexed blocks decode `0xbe` as the
newest `:scheme: https` entry, `0xbf` as the second `:method: PUT` entry, and
`0xc0` as the third retained `:path: /target` entry. The bounded eviction
policy measures each accepted dynamic entry as header name byte count plus
value byte count plus `32` and evicts oldest entries first after insertion or
table-size updates. Reducing the fixture table size to `86` keeps the newest
two entries and evicts the third; reducing it to `42` keeps only the newest
supported `:method: PUT` entry when that entry is followed by
`:path: /target`; the same table size evicts a supported
`:authority: abc.test` entry because that accepted entry is larger than `42`.
Reducing the table size to `30` evicts both supported `:method: PUT` and
`:path: /target` entries, and later dynamic indexed representations for
evicted entries remain unsupported. The checked fixture boundary also covers a
later literal-with-indexing insertion that exceeds the remaining capacity of a
reduced table: inserting `:path: /target` after the reduced table retained
`:scheme: https` and `:method: PUT` keeps the new entry readable at `0xbe` and
evicts the older entries so `0xbf` is unsupported. The HTTP/2 protocol-core
example carries the generalized dynamic-table state, reduced table-size state,
and insertion-caused eviction state through completed HEADERS before decoding
later dynamic indexed blocks, and carries the reduced table-size state through
final CONTINUATION paths.

The fixture also accepts checked dynamic-name literal-with-indexing blocks
after dynamic entries have been inserted. `0x7e 0x06 "/again"` reuses the
newest dynamic entry name `:path`, supplies the visible-ASCII value `/again`,
prepends `:path: /again`, and keeps the older `:path: /target` entry readable
when the bounded table allows it. After three retained dynamic entries exist,
the fixture accepts the continuation-byte indexed-name forms
`0x7f 0x00 0x05 "PATCH"` and `0x7f 0x01 0x06 "/third"` for dynamic indexes
`63` and `64`, respectively. Those forms reuse the retained dynamic entry
name, decode the following visible-ASCII string literal as the replacement
value, and insert the decoded header as the newest dynamic entry without
discarding older entries while the bounded table has room. A deeper bounded
table with retained `:path: /a` entries also accepts dynamic index `127`
through `0x7f 0x40 0x05 "/deep"` for literal-with-indexing, proves the older
retained entry remains addressable through `0xff`, and keeps the next newest
dynamic indexed read pointed at the inserted `:path: /deep` entry. The HTTP/2
protocol-core example carries the same deep dynamic state through a completed
HEADERS block and through a final CONTINUATION block before later dynamic
indexed reads observe the inserted value. It also prints the carried fixture
decode count before and after a split header block, showing that HPACK state
does not advance while the CONTINUATION block is still pending and advances
only after the final accepted header-block decode.

Literal-without-indexing and literal-never-indexed dynamic-name forms reuse
the same dynamic-table name lookup through saturated four-bit indexed-name
prefixes. After `:path: /target` has been inserted, the boundary accepts
`0x0f 0x2f 0x03 "/no"` as `:path: /no` and
`0x1f 0x2f 0x07 "/secret"` as `:path: /secret`. Both forms advance the
immutable fixture decode count without inserting replacement dynamic entries,
so a later `0xbe` lookup from their returned state still reads the previously
inserted `:path: /target` entry. After `:method: PUT` has also been inserted,
the same non-inserting forms accept one continuation byte for dynamic index
`63`: `0x0f 0x30 0x03 "/no"` and
`0x1f 0x30 0x07 "/secret"` both reuse the retained `:path` name, decode the
visible-ASCII value literal, advance only the decode count, and leave later
`0xbe` and `0xbf` reads pointed at the prior `:method: PUT` and
`:path: /target` entries. The deeper table also accepts dynamic index `127`
for both non-inserting forms with `0x0f 0x70 0x05 "/skip"` and
`0x1f 0x70 0x07 "/secret"`; later `0xff` reads from their returned states
still observe the older retained `:path: /a` entry. Missing, malformed,
out-of-range, and unsupported dynamic-name continuations remain on the
unsupported fixture path, including dynamic index `128` for the deep
literal-with-indexing form.
The HTTP/2 protocol-core example continues to cover dynamic HPACK state carry
through completed HEADERS and final CONTINUATION paths. It also checks the
HTTP/2 boundary for a dynamic indexed `0xbe` lookup without any prior dynamic
entry: the unsupported fixture failure leaves the carried decode count
unchanged, and a later accepted literal-with-indexing block inserts
`:path: /target` so the following `0xbe` reads through the returned state.

The same fixture boundary accepts dynamic table-size update bytes `0x3e`,
`0x3f`, `0x3f 0x01`, and the fixture-boundary slice of general multi-byte
HPACK integer continuations with the table-size update prefix, including
`0x3f 0x0b`, `0x3f 0x80 0x01`, `0x3f 0x81 0x01`, and
`0x3f 0x82 0x02`, returning immutable fixture states with table sizes `30`,
`31`, `32`, `42`, `159`, `160`, and `289`. The HTTP/2 protocol-core example
carries accepted updated states at or below its local receive policy through
completed HEADERS and final CONTINUATION paths before later header blocks are
decoded; larger decoded table-size updates are now covered by
[http2-hpack-table-size-policy.md](http2-hpack-table-size-policy.md).
Malformed non-terminating table-size updates and
table-size updates with trailing bytes after a complete integer remain on the
unsupported fixture path.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  literal-with-indexing insertion, newest, second, and third dynamic indexed
  reads, dynamic-name literal-with-indexing insertion, dynamic-name
  literal-without-indexing and literal-never-indexed decode without dynamic
  insertion, dynamic index `127` continuation coverage for all three
  dynamic-name literal forms, malformed and out-of-range dynamic-name
  literals, missing dynamic state, accepted-entry-size eviction, full and
  partial reduced-table-size eviction failure paths, insertion-caused
  eviction, oldest-first eviction after a three-entry table-size reduction,
  and the fixture-boundary table-size update slice.
- `../../../examples/specification/run/http2-protocol-core/` checks the same
  carried immutable HPACK state across completed HEADERS and final
  CONTINUATION paths, including dynamic-name literal-with-indexing,
  continuation-byte dynamic-name literal-with-indexing for retained dynamic
  indexes `63`, `64`, and `127`, dynamic-index `63`
  literal-without-indexing and literal-never-indexed forms without
  replacement insertion, dynamic-index `127` literal-without-indexing and
  literal-never-indexed forms without replacement insertion,
  generalized dynamic indexed lookup, ordinary raw new-name
  literal-with-indexing insertion and dynamic-indexed reuse, oldest-first
  table-size eviction of that ordinary entry,
  insertion-caused eviction, the accepted-entry-size eviction case, pending
  CONTINUATION state not advancing HPACK decode count before final acceptance,
  missing dynamic indexed state not advancing the carried decode count before
  a later accepted insertion and lookup, and the fixture-boundary table-size
  update slice.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented HPACK fixture boundary and route readers to the
  checked examples.
