# HTTP/2 Fixture Marker Classification

Status: proposed

This artifact classifies the focused Veln files that still contain
`HpackFixtureState`, `hpack_fixture::`, or
`hpack.fixture.unsupported_header_block` after the retired aggregate
`../../examples/specification/run/http2-protocol-core/` implementation was
removed.

Validation command:

```sh
rg -l "HpackFixtureState|hpack_fixture::|hpack\\.fixture\\.unsupported_header_block" examples/specification -g "*.veln" | sort
```

The command currently returns 28 files. None are standard-library HTTP/2
source files. They are focused executable examples that intentionally preserve
historical fixture diagnostics or compatibility projections while production
HTTP/2 core receive and send paths use `std::http2::hpack`.

## Classified Focused Files

| File | Classification | Evidence role |
| --- | --- | --- |
| `../../examples/specification/run/hpack-fixture-codec-boundary/hpack_dynamic_core.veln` | retained focused fixture evidence | legacy fixture dynamic-table projection, not production core |
| `../../examples/specification/run/hpack-fixture-codec-boundary/hpack_fixture.veln` | retained focused fixture evidence | legacy fixture codec facade for fixture-boundary output |
| `../../examples/specification/run/hpack-fixture-codec-boundary/main.veln` | retained focused fixture evidence | legacy fixture codec result and stdout projection |
| `../../examples/specification/run/hpack-fixture-codec-human/hpack_fixture.veln` | retained diagnostic fixture evidence | human diagnostic projection for fixture codec fallback |
| `../../examples/specification/run/hpack-fixture-codec-human/main.veln` | retained diagnostic fixture evidence | human diagnostic projection for fixture codec fallback |
| `../../examples/specification/run/hpack-fixture-codec-json/hpack_fixture.veln` | retained diagnostic fixture evidence | JSON diagnostic projection for fixture codec fallback |
| `../../examples/specification/run/hpack-fixture-codec-json/main.veln` | retained diagnostic fixture evidence | JSON diagnostic projection for fixture codec fallback |
| `../../examples/specification/run/hpack-fixture-dynamic-index-human/hpack_fixture.veln` | retained diagnostic fixture evidence | human diagnostic projection for dynamic-index fixture failure |
| `../../examples/specification/run/hpack-fixture-dynamic-index-human/main.veln` | retained diagnostic fixture evidence | human diagnostic projection for dynamic-index fixture failure |
| `../../examples/specification/run/hpack-fixture-dynamic-index-json/hpack_fixture.veln` | retained diagnostic fixture evidence | JSON diagnostic projection for dynamic-index fixture failure |
| `../../examples/specification/run/hpack-fixture-dynamic-index-json/main.veln` | retained diagnostic fixture evidence | JSON diagnostic projection for dynamic-index fixture failure |
| `../../examples/specification/run/hpack-fixture-dynamic-name-continuation-json/hpack_fixture.veln` | retained diagnostic fixture evidence | JSON diagnostic projection for dynamic-name continuation fallback |
| `../../examples/specification/run/hpack-static-codec-boundary/hpack_static.veln` | retained focused fixture evidence | static-table compatibility helper for fixture-boundary output |
| `../../examples/specification/run/hpack-static-codec-boundary/main.veln` | retained focused fixture evidence | static-table fixture-boundary result projection |
| `../../examples/specification/run/hpack-static-index-unsupported-human/hpack_fixture.veln` | retained diagnostic fixture evidence | human diagnostic projection for unsupported static index |
| `../../examples/specification/run/hpack-static-index-unsupported-json/hpack_fixture.veln` | retained diagnostic fixture evidence | JSON diagnostic projection for unsupported static index |
| `../../examples/specification/run/http2-protocol-core-content-length-body/hpack_fixture.veln` | retained historical diagnostic support | local support for focused content-length body projection |
| `../../examples/specification/run/http2-protocol-core-content-length-body/main.veln` | retained focused HTTP/2 evidence | focused content-length body projection with historical prefix |
| `../../examples/specification/run/http2-protocol-core-content-length-early-human/hpack_fixture.veln` | retained historical diagnostic support | local support for early END_STREAM human diagnostic |
| `../../examples/specification/run/http2-protocol-core-content-length-early-human/main.veln` | retained diagnostic fixture evidence | early END_STREAM human diagnostic projection |
| `../../examples/specification/run/http2-protocol-core-content-length-early-json/hpack_fixture.veln` | retained historical diagnostic support | local support for early END_STREAM JSON diagnostic |
| `../../examples/specification/run/http2-protocol-core-content-length-early-json/main.veln` | retained diagnostic fixture evidence | early END_STREAM JSON diagnostic projection |
| `../../examples/specification/run/http2-protocol-core-content-length-over-human/hpack_fixture.veln` | retained historical diagnostic support | local support for over-length DATA human diagnostic |
| `../../examples/specification/run/http2-protocol-core-content-length-over-human/main.veln` | retained diagnostic fixture evidence | over-length DATA human diagnostic projection |
| `../../examples/specification/run/http2-protocol-core-content-length-over-json/hpack_fixture.veln` | retained historical diagnostic support | local support for over-length DATA JSON diagnostic |
| `../../examples/specification/run/http2-protocol-core-content-length-over-json/main.veln` | retained diagnostic fixture evidence | over-length DATA JSON diagnostic projection |
| `../../examples/specification/run/http2-protocol-core-outbound-hpack-table-size-update-json/hpack_fixture.veln` | retained historical diagnostic support | local support for outbound HPACK table-size JSON projection |
| `../../examples/specification/run/http2-protocol-core-outbound-hpack-table-size-update-json/main.veln` | retained diagnostic fixture evidence | outbound HPACK table-size JSON diagnostic projection |

## Completion Impact

This classification closes only the focused fixture-marker inventory. It does
not classify the removed aggregate helper invocations, exact stdout lines, or
output-chunk tables. Those rows remain part of
`http2-sans-io-protocol-core.md`.
