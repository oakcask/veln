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
