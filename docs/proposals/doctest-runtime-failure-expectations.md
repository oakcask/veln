# Doctest Runtime Failure Expectations

Status: proposed

This page records the runtime-failure follow-up that was outside the
expected-error doctest completion target. It is proposal work, not current
behavior.

## Read First

- Current doctest behavior: [../specification/commands.md](../specification/commands.md)
  and [../specification/test-json.md](../specification/test-json.md).
- Current contract runtime failures:
  [../specification/contracts.md](../specification/contracts.md).
- Static expected-error doctest completion notes are summarized below.

## Current Boundary

`veln fail` doctest fences are static negative examples. They are accepted only
when generated source produces an error diagnostic before execution. They do
not describe expected runtime failures.

The completed static target extracts negative doctests as generated private
functions for `check` and `test`, consumes matching top-level error diagnostics,
and reports `doctest.expected_failure_missing` at the `veln fail` fence when no
error diagnostic appears. Hint-only diagnostics remain visible and do not
satisfy the expected failure. Negative doctests do not create expected-output
attachments.

Completion coverage included accepted negative doctests, missing expected
failures for `check --json`, and missing expected failures blocking
`test --json`.

## Target

Define a doctest metadata form for examples that are expected to pass static
checking, execute, and fail at runtime in a specific way.

The first target should cover one narrow runtime failure class before expanding
the surface. Runtime contract failure is the preferred initial class because
the current test JSON schema already has structured contract failure details.

## Non-Goals

- Do not change `veln fail`; it remains a static expected-error facility.
- Do not add broad panic matching, arbitrary stderr matching, or command-status
  assertions in the first target.
- Do not weaken parse, semantic, or checked-core gates before runtime
  execution.

## Acceptance Checks

- A positive static doctest with the new runtime-failure metadata is selected
  and executed.
- The doctest passes only when the selected runtime failure kind and key
  details match.
- If execution succeeds, fails with a different runtime failure, or is blocked
  before execution, the doctest case fails or blocks with structured test JSON.
- Expected-output comparison remains separate from expected runtime failure
  matching.

## Update When

- Add the implemented metadata and test JSON behavior to
  `../specification/commands.md` and `../specification/test-json.md` only after
  code and CLI coverage exist.
