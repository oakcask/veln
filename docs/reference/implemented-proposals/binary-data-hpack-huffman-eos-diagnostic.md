# Binary Data HPACK Huffman EOS Diagnostic

Status: implemented

This record preserves the completed HPACK Huffman EOS protocol-facing byte
diagnostic slice from the binary data standard-library proposal. Current
behavior is specified by `../../specification/run-json.md`,
`../../specification/commands.md`, and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

`hpack.fixture.huffman_eos_symbol` is a protocol-facing byte diagnostic for
HPACK fixture input where the Huffman EOS code appears as a decoded symbol.
The diagnostic remains at the HPACK fixture boundary and keeps the same byte
detail shape as the surrounding protocol byte diagnostics: header-block byte
offset, observed header-block size, observed first byte, expected fixture,
codec module, and a bounded byte preview.

The direct HPACK fixture path and the HTTP/2 protocol-core path both project
the diagnostic through human output and command JSON. Runtime diagnostic
payload projection also carries the same protocol diagnostic fields when this
failure is returned as a source-visible `RuntimeDiagnostic(...)` value.

This slice does not add full HPACK compression, general Huffman acceptance,
dynamic-table behavior, socket behavior, or production HTTP/2 behavior.

## Evidence

- `../../../examples/specification/run/hpack-fixture-huffman-eos-human/` and
  `../../../examples/specification/run/hpack-fixture-huffman-eos-json/` check
  the direct HPACK fixture human and JSON projections.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-json/`
  check the HTTP/2 protocol-core human and JSON projections.
- `../../../examples/specification/run/runtime-diagnostic-payload-hpack-huffman-eos-json/`
  checks runtime diagnostic payload projection for the HPACK Huffman EOS
  protocol diagnostic fields.
- `../../specification/run-json.md` and `../../specification/commands.md`
  summarize the command-facing diagnostic projection that these examples pin.
