# Test JSON

This page routes implemented JSON output for `veln test --json`.

## Read First

- The envelope uses schema version `veln-test-json/v0`.
- The top-level status is `passed`, `failed`, `blocked`, or `error`.
- Parse and semantic diagnostics block Java compilation and execution.
- JDK setup failures become case errors with `reason: "runner_error"`.
- Doctest `runtime=contract`, `runtime=ensure`, and `runtime=result`
  expectations pass only when the selected runtime failure details match;
  mismatches use
  `reason: "expected_runtime_failure"`.
- Doctest hidden setup uses `> ` inside executable `veln` fences; `#`
  remains visible source comment syntax.
- Doctest expected-output comparison is a separate route; mismatches use
  `reason: "expected_output"` even when a runtime expectation matches.
- Executable specification cases may use `[[result_value_assert]]` against a
  JSON string path that contains a returned result-failure value, such as
  `error.details.value`. The harness wraps that value as the outer `Err` and
  exposes path assertions over the returned value shape, including
  `RuntimeDiagnostic`, `RuntimeByteDiagnostic`, byte offset, field path,
  count/range/reason facts, and optional byte preview fields.

## Read When

- Top-level fields: [test-json-full.md](test-json-full.md#envelope).
- Discovery and explicit target selection:
  [test-json-full.md](test-json-full.md#selection).
- Counts in `summary`: [test-json-full.md](test-json-full.md#summary).
- Case records, doctests, runtime failures, runtime expectation mismatches,
  expected-output mismatches, and captured stdio events:
  [test-json-full.md](test-json-full.md#cases).
- Static gate behavior: [test-json-full.md](test-json-full.md#static-gate).
- Readable doctest runtime JSON coverage:
  `../../examples/specification/test/doctest-runtime-contract-json/`,
  `../../examples/specification/test/doctest-runtime-contract-blocked-json/`,
  `../../examples/specification/test/doctest-runtime-ensure-json/`,
  `../../examples/specification/test/doctest-runtime-ensure-blocked-json/`,
  `../../examples/specification/test/doctest-runtime-result-json/`,
  `../../examples/specification/test/doctest-runtime-result-blocked-json/`.
- Readable coverage for runtime expectation plus output mismatch:
  `../../examples/specification/test/doctest-runtime-output-mismatch-json/`.
- Runtime diagnostic payload value-shape assertion coverage:
  `../../examples/specification/run/runtime-diagnostic-payload-byte-json/`
  and
  `../../examples/specification/run/runtime-diagnostic-payload-hpack-string-length-json/`.
  Additional HPACK fixture payload assertions are checked by
  `../../examples/specification/run/runtime-diagnostic-payload-hpack-raw-string-json/`,
  `../../examples/specification/run/runtime-diagnostic-payload-hpack-huffman-padding-json/`,
  `../../examples/specification/run/runtime-diagnostic-payload-hpack-huffman-eos-json/`,
  and
  `../../examples/specification/run/runtime-diagnostic-payload-hpack-dynamic-index-json/`.
  `RuntimeDiagnostic(...)` HTTP/2 protocol payload projection assertions are
  checked by
  `../../examples/specification/run/http2-protocol-core-ping-length-json/`,
  `../../examples/specification/run/http2-protocol-core-goaway-length-json/`,
  `../../examples/specification/run/http2-protocol-core-settings-ack-length-json/`,
  `../../examples/specification/run/http2-protocol-core-rst-stream-length-json/`,
  `../../examples/specification/run/http2-protocol-core-data-padding-json/`,
  `../../examples/specification/run/http2-protocol-core-flow-control-json/`,
  `../../examples/specification/run/http2-protocol-core-flow-control-connection-json/`,
  `../../examples/specification/run/http2-protocol-core-content-length-early-json/`,
  `../../examples/specification/run/http2-protocol-core-content-length-over-json/`,
  `../../examples/specification/run/http2-protocol-core-stream-after-goaway-json/`,
  and
  `../../examples/specification/run/http2-protocol-core-local-stream-after-goaway-json/`.

## Skip Unless Needed

- Use [json-output.md](json-output.md) first when choosing between check, run,
  and test JSON pages.
- Use [diagnostics-json.md](diagnostics-json.md) for diagnostic object shape.
