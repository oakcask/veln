# HTTP/2 HPACK Malformed String Diagnostics

Status: implemented

The HTTP/2 protocol-core HPACK fixture boundary now splits malformed string
literal inputs that used to share the broad unsupported-header-block route.

Malformed HPACK string-length encodings, including non-terminating
string-length continuations, project through
`hpack.fixture.malformed_string_length`. Malformed raw string values for
supported literal-name forms, including malformed raw `:status` values and
non-visible raw bytes, project through
`hpack.fixture.malformed_raw_string_value`.

Both diagnostics keep the HPACK fixture diagnostic shape: header-block byte
offset, observed header-block size, observed first byte, expected fixture,
codec module, and bounded byte preview. These ids remain fixture-owned and do
not broaden protocol-owned HTTP/2 diagnostics or accepted header validation.

## Evidence

- Historical aggregate evidence checks completed
  HEADERS and final CONTINUATION paths for malformed string lengths and
  malformed raw string values.
- `../../../examples/specification/run/http2-protocol-core-hpack-string-length-json/`
  and `../../../examples/specification/run/http2-protocol-core-hpack-string-length-human/`
  check the JSON and human diagnostic projection for malformed string lengths.
- `../../../examples/specification/run/http2-protocol-core-hpack-raw-string-json/`
  and `../../../examples/specification/run/http2-protocol-core-hpack-raw-string-human/`
  check the JSON and human diagnostic projection for malformed raw string
  values.
- `../../specification/execution.md`, `../../specification/examples.md`, and
  `../../specification/run-json.md` summarize the implemented diagnostic
  boundary and route readers to the checked examples.
