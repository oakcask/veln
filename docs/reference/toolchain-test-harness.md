# Toolchain Test Harness

This page specifies the implemented CLI integration test harness. It is a
reference for test organization, not a source for command behavior.

## Read First

- Command behavior belongs in [language/commands.md](language/commands.md).
- JSON output behavior belongs in [language/json-output.md](language/json-output.md).
- The completion review records verification evidence:
  [../reviews/toolchain-test-harness-completion.md](../reviews/toolchain-test-harness-completion.md).

## Case Layout

The CLI harness discovers case directories that contain `case.toml`. Each case
is copied into a temporary project before the command runs, so fixtures stay
isolated from the repository checkout.

Cases are grouped by command or behavior area. The harness owns command
execution, fixture copying, exit-status checks, stream checks, and JSON
assertions.

## Manifest Policy

Case manifests are declarative. They should describe the command, expected exit
status, expected stdout or stderr fragments, and structured JSON expectations.
They must not execute arbitrary shell commands.

JSON output should be parsed and checked semantically by default. Full JSON
equality is reserved for schema smoke tests where exact envelope shape is the
behavior under test.

## Boundaries

The harness standardizes CLI integration tests. It does not replace parser,
checker, runtime, or formatter unit tests in compiler crates.

Use the language reference when a case needs to decide whether command,
diagnostic, JSON, runtime, or source behavior is correct. Use this page only
for harness organization and assertion policy.
