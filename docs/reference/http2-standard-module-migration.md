# HTTP/2 Standard Module Migration

Status: implemented

This record classifies the public symbol move from the implicit prelude to
explicit standard modules.

| Former public name | Current public path | Policy |
| --- | --- | --- |
| `byte_decode_http2_frame` | `http2::frame::decode` | removed |
| `http2_protocol_*` | `http2::diagnostic::protocol_*` | removed |
| `http2_peer_limit_*` | `http2::diagnostic::peer_limit_*` | removed |
| `hpack_fixture_huffman_bytes_label` | `http2::hpack::huffman_bytes_label` | removed |
| `hpack_fixture_huffman_label_bytes` | `http2::hpack::huffman_label_bytes` | removed |
| remaining diagnostic `hpack_fixture_*` names | matching member of `http2::hpack::diagnostic` | removed |

The former spellings remain only as private compiler-adapter and JVM link
names, tests that assert the public absence, and historical records describing
the earlier boundary. They are not source compatibility aliases.

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
