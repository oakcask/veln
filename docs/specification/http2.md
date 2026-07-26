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
- `http2::hpack`: prefixed-integer encoding and decoding, Huffman byte labels,
  static entries, and immutable initial dynamic-table state.
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

`http2::hpack::static_entry(index)` exposes every one-based HPACK static-table
entry from 1 through 61; `static_entry_name(entry)` and
`static_entry_value(entry)` project its exact fields.
`static_entry_index(name, value)` returns the exact entry index, while
`static_name_index(name)` returns the first index with the exact name. Indices
outside the table and unknown names or values return `None`. The complete
forward and reverse contract is checked by the adjacent standard-library
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln).

Additional executable evidence lives in the adjacent standard-library
`*_test.veln` files and in the focused HTTP/2 cases under
`../../examples/specification/`.
The broad protocol-core case remains coverage for state transitions and output
chunk projections while focused cases retain human and JSON diagnostic
coverage.
