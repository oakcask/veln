---
review-when: The documented test JSON behavior or executable test JSON evidence changes.
---

# Test JSON

This page routes implemented JSON output for `veln test --json`.

## Read First

- The envelope uses schema version `veln-test-json/v0`.
- The top-level status is `passed`, `failed`, `blocked`, or `error`.
- Parse and semantic diagnostics block Java compilation and execution.
- Targetless discovery includes valid `.test.veln` companions, and explicit
  selection of `X.veln` includes existing `X.test.veln` and `X_test.veln`
  peers. Explicit selection of `X.test.veln` includes `X.veln` in analysis.
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
  count/range/fixed-value/reason facts, and optional byte preview fields.
- Runtime result failures in top-level tests keep returned
  `RuntimeDiagnostic(...)` payloads in `cases.*.failure.details.value` and
  project contained byte and value payload fields into the same structured
  diagnostic details used by `run --json`.
- `test --json` keeps `cases`, captured `events`, `summary`, top-level
  `status`, diagnostics, and failures in discovered-case order for serial
  `--jobs 1`, explicit bounded `--jobs <JOBS>`, and automatic job modes.

## Read When

- Top-level fields: [test-json-full.md](test-json-full.md#envelope).
- Discovery and explicit target selection:
  [test-json-full.md](test-json-full.md#selection).
- Companion discovery and explicit target selection:
  `../../examples/specification/test/companion-discovery/`,
  `../../examples/specification/test/companion-explicit-target-selection/`,
  and
  `../../examples/specification/test/companion-explicit-companion-selection/`.
- Companion private-function execution, private source ADT execution, private
  nominal effect operation execution, and established target effect
  propagation:
  `../../examples/specification/test/companion-private-function-access/`,
  `../../examples/specification/test/companion-private-source-adt-access/`,
  `../../examples/specification/test/companion-private-effect-operation/`, and
  `../../examples/specification/test/companion-private-function-established-effects/`.
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
- Ordered parallel-job coverage:
  `../../examples/specification/test/parallel-jobs-one-json/`,
  `../../examples/specification/test/parallel-jobs-two-json/`, and
  `../../examples/specification/test/parallel-jobs-auto-json/`.
- Runtime diagnostic payload and helper-returned value-shape assertion
  coverage:
  `../../examples/specification/test/runtime-diagnostic-payload-json/`,
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
  `../../examples/specification/run/http2-protocol-core-settings-item-length-json/`,
  `../../examples/specification/run/http2-protocol-core-settings-enable-push-role-json/`,
  `../../examples/specification/run/http2-protocol-core-rst-stream-length-json/`,
  `../../examples/specification/run/http2-protocol-core-data-padding-json/`,
  `../../examples/specification/run/http2-protocol-core-flow-control-json/`,
  `../../examples/specification/run/http2-protocol-core-flow-control-connection-json/`,
  `../../examples/specification/run/http2-protocol-core-content-length-early-json/`,
  `../../examples/specification/run/http2-protocol-core-content-length-over-json/`,
  `../../examples/specification/run/http2-protocol-core-invalid-stream-id-json/`,
  `../../examples/specification/run/http2-protocol-core-invalid-stream-reference-json/`,
  `../../examples/specification/run/http2-protocol-core-push-promise-json/`,
  `../../examples/specification/run/runtime-diagnostic-http2-closed-helper-json/`,
  `../../examples/specification/run/http2-protocol-core-stream-after-goaway-json/`,
  and
  `../../examples/specification/run/http2-protocol-core-local-stream-after-goaway-json/`.

## Skip Unless Needed

- Use [json-output.md](json-output.md) first when choosing between check, run,
  and test JSON pages.
- Use [diagnostics-json.md](diagnostics-json.md) for diagnostic object shape.
