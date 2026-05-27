# Toolchain Test Harness Completion Review

Status: complete.

This review covers the `toolchain-test-harness` proposal after the remaining
case-directory and manifest-driven assertion work. The proposal remains the
organization record for CLI integration tests; command behavior still routes
through `../reference/language/`.

## Completion Check

- Case discovery is implemented by walking `tests/toolchain_cases/` for
  `case.toml` files and running each case in a temporary project.
- The manifest supports the required command, exit status, stdout, stderr,
  JSON path assertions, diagnostic selectors, JDK requirements, and platform
  skips described by the proposal.
- Fixture copying treats the case directory as the project tree and excludes
  only `case.toml`, which matches the proposed case layout.
- JSON output is parsed and checked semantically. Cases assert stable paths
  and diagnostic fields instead of relying on full JSON string equality.
- Check cases cover valid JSON output, human diagnostics, type diagnostics,
  nested discovery, ignored build output, and manifest module drift.
- Run cases cover entry resolution, entry argument count, primitive argument
  conversion failure, unsupported entry argument types, JSON success, and JSON
  contract failure.
- Test cases cover no discovered tests, source-to-test convention selection,
  static gate blocking, doctest expected output, and runtime stdio capture.

## Residual Scope

The remaining bespoke CLI integration tests are not blocking this proposal.
They exercise custom setup that the manifest deliberately does not represent
yet, such as fake JDK tools, cache counters, command help, formatter mutation,
and broad diagnostic detail checks. The proposal explicitly keeps that kind of
process setup in Rust tests until the manifest has a clean declarative feature
for it.

Cache behavior also remains covered by bespoke tests instead of declarative
cases. That is consistent with the migration boundary because the current case
manifest cannot install fake tools or inspect generated-cache side effects.

## Verification

- `cargo test -p veln-cli --test toolchain_harness`
