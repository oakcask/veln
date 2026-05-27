# Expected-Error Doctest Completion Review

Status: complete.

This review covers the expected-error doctest target that remained after the
implemented positive doctest decisions for result propagation, explicit
doctest error types, and expected output.

## Completion Check

- Negative static examples are represented by block-local `veln fail` metadata
  on executable doctest fences, keeping the visible example body as ordinary
  Veln source.
- `check` and `test` extract negative doctests as generated private functions,
  so their parse and semantic diagnostics participate in the static gate while
  the examples are not selected as runtime doctest cases.
- A negative doctest is accepted only when its generated source produces at
  least one severity `error` diagnostic. Matching diagnostics are consumed from
  the top-level diagnostic list.
- If the generated source produces no error diagnostic, the command reports
  `doctest.expected_failure_missing` at the `veln fail` fence. Hint-only
  diagnostics remain visible and do not satisfy the expected failure.
- The structured CLI cases cover accepted negative doctests, missing expected
  failures for `check --json`, and missing expected failures blocking
  `test --json`.
- Negative doctests do not create expected-output attachments, so output
  comparison remains limited to positive executable doctests.
- The current reference pages describe the behavior in
  `../reference/language/source-surface.md`,
  `../reference/language/commands.md`,
  `../reference/language/test-json.md`, and
  `../reference/language/diagnostics-json.md`.

## Residual Scope

Expected runtime failures remain outside this target. The implemented
`veln fail` mode is a static negative-example facility: it verifies that an
example is rejected during checking, not that a selected doctest fails at
runtime in a particular way.

## Verification

- `cargo test -p veln-test doctest`
- `cargo test -p veln-cli --test check_json negative_doctest`
- `cargo test -p veln-cli --test toolchain_harness`
