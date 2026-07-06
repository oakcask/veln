# HTTP/2 HPACK Huffman Encode Boundary

Status: implemented

This record preserves the completed source-visible HPACK Huffman encode
boundary slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md`,
`../../specification/commands.md`, and `../../specification/run-json.md`, and
checked by executable examples under `../../../examples/specification/run/`.

## Completed Behavior

The HTTP/2 protocol-core HPACK fixture now exposes
`encode_hpack_huffman_payload` and `encode_hpack_huffman_payload_bytes`.
The string helper maps supported source-visible strings to bytes, encodes
those bytes through the HPACK static Huffman table, applies EOS padding, and
returns only the encoded Huffman payload bytes. The byte helper accepts
bounded `ByteChunk` input and returns the same payload-only byte shape.

`encode_hpack_huffman_string_literal` reuses the payload helper and still adds
the Huffman string length prefix for fixture header-list encoding.

Unsupported string input returns `Err(HpackFixtureFailure)` on the existing
fixture raw string encoding failure path. It does not use a runtime diagnostic
path.

This slice does not add full HPACK compression, compression strategy
selection, unbounded dynamic-table behavior, a general header-list compressor,
or frame splitting.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks
  payload-only encoding for `test` as `49 50 9f`.
- `../../../examples/specification/run/http2-protocol-core/` checks bounded
  raw byte input `00 ff` as `ff c7 ff ff dd`.
- `../../../examples/specification/run/http2-protocol-core/` checks an
  unsupported source-visible string returning a fixture encode failure.
- The same protocol-core case keeps the existing Huffman-marked fixture string
  literal examples that include the HPACK string length prefix.
