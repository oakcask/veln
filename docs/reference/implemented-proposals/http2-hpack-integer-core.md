# HTTP/2 HPACK Integer Core

Status: implemented

This record preserves the completed HPACK integer core slice from the HTTP/2
sans-I/O protocol-core proposal. Current public behavior is specified by
`../../specification/http2.md` and the checked executable case
`../../../examples/specification/run/hpack-prefixed-integer-codec/`.

## Completed Behavior

The original `hpack_dynamic_core` boundary established a bounded HPACK integer
decoder and encoder for the prefix widths used by the checked protocol slice.
That work proved saturated prefix values and continuation bytes for indexed
header fields, dynamic table-size updates, and string literal lengths before
the reusable codec moved behind the public `http2::hpack` facade.

The fixture copies no longer expose the standalone integer facade or its
encode-only helpers. Their private prefixed-integer decoder remains in place
only where the fixture header codec needs it. Public callers use
`http2::hpack::encode_integer` and `http2::hpack::decode_integer`, which share
the one-through-eight-bit width contract and reject incomplete or
out-of-range encodings through ordinary `Result` failures.

This remains a deterministic specification slice. It does not add full HPACK
compression, unbounded integer growth, unbounded dynamic-table behavior, or
new protocol stream-state rules.

## Evidence

- `../../../crates/veln-stdlib/veln/http2/hpack_test.veln` checks direct and
  saturated-prefix values, multi-octet values, every supported prefix width,
  round trips, representation bits, and rejected boundaries.
- `../../../examples/specification/run/hpack-prefixed-integer-codec/` checks
  the public facade with the canonical multi-octet encoding, preserved
  representation bits, round trips, and the indexed-field, table-size, and
  literal-length shapes that motivated the historical slice.
- `../../../examples/specification/run/hpack-fixture-codec-boundary/` and
  historical aggregate evidence retain coverage
  for fixture-internal prefixed-integer decoding as part of header codec
  behavior, without exposing a second integer API.
