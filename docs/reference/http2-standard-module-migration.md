# HTTP/2 Standard Module Migration

Status: implemented

This record classifies the public symbol move from the implicit prelude to
explicit standard modules.

| Former public name | Current public path | Policy |
| --- | --- | --- |
| `byte_decode_http2_frame` | `http2::frame::decode` | removed |
| `http2_protocol_*` | `http2::diagnostic::protocol_*` | removed |
| `http2_peer_limit_*` | `http2::diagnostic::peer_limit_*` | removed |
| `hpack_fixture_huffman_bytes_label` | none; fixture display is local | removed |
| `hpack_fixture_huffman_label_bytes` | none; fixture parsing is local | removed |
| remaining diagnostic `hpack_fixture_*` names | matching member of `http2::hpack::diagnostic` | removed |

The former diagnostic spellings remain only as private compiler-adapter and
JVM link names, tests that assert the public absence, and historical records
describing the earlier boundary. The two Huffman label spellings and their
adapters are removed entirely. They are not source compatibility aliases.

Current specification examples import the public module explicitly. Private
HPACK responsibility modules are loaded only through the dependency closure of
the public facade and cannot be imported from the `std` package.

## Prefixed-Integer Fixture Retirement

The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
coverage replaces the focused fixture's pure indexed-prefix,
table-size-prefix, literal-length-prefix, unterminated continuation, and three
encoding assertions. It retains the exact indexed, table-size, and
literal-length vectors beside the public codec tests. Their shared invariant
is that one public finite codec preserves representation bits and round-trips
every supported prefix width while rejecting incomplete or out-of-range
continuations. Header-codec helpers remain in the fixture until their stateful
callers move.

## Huffman Fixture Retirement

`http2::hpack::encode_huffman` and `decode_huffman` replace the label facade
with a stateless arbitrary-octet codec. The private standard implementation
owns the complete HPACK static Huffman table. Adjacent standard tests check
canonical encodings, every input octet, recursive multi-octet round trips,
padding boundaries, EOS, malformed padding, truncated codes, and atomic
failure. The focused
`../../examples/specification/run/hpack-huffman-codec/` case records public
encoded bytes, decoded non-visible octets, and representative failures.
