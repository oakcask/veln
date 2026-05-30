# Test JSON

This page routes implemented JSON output for `veln test --json`.

## Read First

- The envelope uses schema version `veln-test-json/v0`.
- The top-level status is `passed`, `failed`, `blocked`, or `error`.
- Parse and semantic diagnostics block Java compilation and execution.
- JDK setup failures become case errors with `reason: "runner_error"`.

## Read When

- Top-level fields: [test-json-full.md](test-json-full.md#envelope).
- Discovery and explicit target selection:
  [test-json-full.md](test-json-full.md#selection).
- Counts in `summary`: [test-json-full.md](test-json-full.md#summary).
- Case records, doctests, runtime failures, runtime expectation mismatches,
  expected-output mismatches, and captured stdio events:
  [test-json-full.md](test-json-full.md#cases).
- Static gate behavior: [test-json-full.md](test-json-full.md#static-gate).
- Readable doctest runtime contract JSON coverage:
  `../../examples/specification/test/doctest-runtime-contract-json/` and
  `../../examples/specification/test/doctest-runtime-contract-blocked-json/`.

## Skip Unless Needed

- Use [json-output.md](json-output.md) first when choosing between check, run,
  and test JSON pages.
- Use [diagnostics-json.md](diagnostics-json.md) for diagnostic object shape.
