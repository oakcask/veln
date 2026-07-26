# HTTP/2 Standard Modules

HTTP/2 support is opt-in. Source files import the required public module from
the toolchain-owned `std` package; no HTTP/2 function is part of the implicit
prelude.

```veln
use http2::frame from "std"
use http2::diagnostic from "std"
use http2::hpack from "std"
use http2::hpack::diagnostic from "std"
use http2::core from "std"
```

The public routes are:

- `http2::frame`: frame decoding and validated frame-header encoding.
- `http2::diagnostic`: protocol and peer-limit diagnostic constructors.
- `http2::hpack`: prefixed-integer and HPACK Huffman codecs, static entries,
  immutable dynamic-table state, and indexed and literal header-field decoding.
- `http2::hpack::diagnostic`: HPACK diagnostic constructors.
- `http2::core`: connection and role-specific stream-id domains.

Nested implementation modules below `http2::hpack` are not package exports.
The JVM adapter keeps its intrinsic link names private; source code calls only
the module-qualified API. Diagnostic ids, human rendering, and
`details.protocol_diagnostic` projections remain stable.

`http2::hpack::encode_integer(value, prefix_bits, representation_bits)` accepts
a non-negative `Int` and a prefix width from one through eight. It preserves
the caller-supplied high representation bits in the first octet and returns
the finite HPACK continuation encoding as a `ByteChunk`.
`http2::hpack::decode_integer(input, prefix_bits)` uses the same width contract
and reports the decoded value plus the consumed octet count. Empty input,
invalid widths, incomplete continuations, and encodings beyond the `Int` range
are rejected. The canonical multi-octet encoding and representation-bit
behavior are checked by
`../../examples/specification/run/hpack-prefixed-integer-codec/`.

`http2::hpack::encode_huffman(bytes)` encodes arbitrary `ByteChunk` octets
with the HPACK static Huffman table and the required EOS-prefix padding.
`decode_huffman(input)` returns the exact decoded `ByteChunk`, including
non-visible octets. It rejects EOS as a payload symbol, invalid or overlong
padding, and truncated or invalid code sequences. A failure returns no partial
decoded value. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks canonical vectors, every single octet, recursive input, padding
boundaries, and rejection paths. The focused
[`hpack-huffman-codec`](../../examples/specification/run/hpack-huffman-codec/)
case records the public facade's encoded and decoded octets and representative
failures.

`http2::hpack::static_entry(index)` exposes every one-based HPACK static-table
entry from 1 through 61; `static_entry_name(entry)` and
`static_entry_value(entry)` project its exact fields.
`static_entry_index(name, value)` returns the exact entry index, while
`static_name_index(name)` returns the first index with the exact name. Indices
outside the table and unknown names or values return `None`. The complete
forward and reverse contract is checked by the adjacent standard-library
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln).

`http2::hpack::empty_dynamic_table(capacity)` creates an immutable empty table
and rejects a negative capacity. `insert_dynamic_table_entry(table, name,
value)` inserts at index one, keeps entries in newest-to-oldest order, and
accounts for each entry as the name octet count plus the value octet count plus
32. It evicts the oldest entries until the result fits; an entry larger than
the active capacity clears the result table. Header values are `ByteChunk`
values and preserve arbitrary octets.

`dynamic_table_with_capacity(table, capacity)` returns a new table, evicting
after a shrink and retaining entries after a grow, and rejects a negative
capacity. Successful and failed transitions leave the input table unchanged.
The capacity, current size, and entry count have dedicated projections.
`dynamic_table_entry(table, index)` performs one-based lookup and returns
`None` for non-positive or unavailable indices; `dynamic_entry_name` and
`dynamic_entry_value` project a found entry. The adjacent standard-library
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks exact size accounting, insertion order, eviction, capacity changes,
lookup boundaries, arbitrary-octet values, and input-state preservation. The
focused
[`hpack-dynamic-table-state`](../../examples/specification/run/hpack-dynamic-table-state/)
case checks the same facade from an external package and records its projected
state and octet values through command output.

`http2::hpack::decode_indexed_header_field(input, table)` decodes one HPACK
indexed header-field representation with the full seven-bit-prefixed integer.
Indices 1 through 61 resolve through the static table. Larger indices resolve
through the supplied immutable dynamic table, where index 62 selects its
newest entry. The transition reports the consumed octet count, decoded name
and exact value `ByteChunk`, and the unchanged dynamic table.

The typed `IndexedDecodeFailure` distinguishes malformed and incomplete
integers, index zero, unavailable static entries, and unavailable dynamic
entries. Failure projections expose a stable failure kind and the requested
table coordinates where applicable; a failure contains neither a decoded
header nor a next table. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks every static entry, single- and multi-octet dynamic indices, arbitrary
value octets, all reachable failure classes, and state preservation. The
focused
[`hpack-indexed-header-field`](../../examples/specification/run/hpack-indexed-header-field/)
case checks the public facade from an external package.

`http2::hpack::decode_literal_header_field(input, table)` decodes one literal
header field with incremental indexing, without indexing, or marked never
indexed. A zero name index reads a raw or Huffman string name; a nonzero name
index resolves through the static or immutable dynamic table with the
representation's full prefixed integer. The value is another raw or Huffman
string and is returned as an exact `ByteChunk`, including non-visible octets.
The transition identifies the representation and reports its decoded field,
consumed octet count, and next table. Incremental indexing inserts the field
into the next table; the other representations return the unchanged table.

The typed `LiteralDecodeFailure` distinguishes malformed and incomplete name
indices, unavailable indexed names, malformed and incomplete name or value
lengths, truncated raw name or value octets, invalid name octets, and name or
value Huffman failures. Truncation projections report the expected and
available octet counts, while unavailable dynamic-name projections report the
requested table coordinates. Every failure contains neither a decoded field
nor a next table and leaves the input table unchanged. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks all representations, direct and indexed names, raw and Huffman strings,
multi-octet indices and lengths, exact value octets, insertion, and failure
preservation. The focused
[`hpack-literal-header-field`](../../examples/specification/run/hpack-literal-header-field/)
case records public raw result values and representative typed failures.

Additional executable evidence lives in the adjacent standard-library
`*_test.veln` files and in the focused HTTP/2 cases under
`../../examples/specification/`.
The broad protocol-core case remains coverage for state transitions and output
chunk projections while focused cases retain human and JSON diagnostic
coverage.
