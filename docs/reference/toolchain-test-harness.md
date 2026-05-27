# Toolchain Test Harness

This page specifies the implemented CLI integration test harness. It is a
reference for test organization, not a source for command behavior.

## Read First

- Command behavior belongs in
  [../specification/commands.md](../specification/commands.md).
- JSON output behavior belongs in
  [../specification/json-output.md](../specification/json-output.md).
- The completion review records verification evidence:
  [toolchain-test-harness-completion.md](../reviews/toolchain-test-harness-completion.md).

## Read When

- Add a case under `toolchain_cases/` when behavior must be checked through the
  public CLI.
- Change this harness when a manifest needs a reusable assertion shape, command
  environment, repeated invocation, or fixture setup rule.
- JVM backend fixtures exercise the implemented bytecode path by default. Use
  the JVM bytecode proposal review for migration cleanup:
  [../reviews/jvm-bytecode-backend-completion.md](../reviews/jvm-bytecode-backend-completion.md).

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

Treat `command`, `[env]`, and `repeat` as invocation settings. Treat `exit`,
`[stdout]`, `[stderr]`, `[[json_assert]]`, and `[[diagnostics]]` as expected
observable results.

Use `[env]` for fixed environment variables that belong to the fixture. Use
`repeat` when one isolated project should run the same command more than once,
for example to compare command-visible behavior across cache misses and cache
hits.

JSON output should be parsed and checked semantically by default. Full JSON
equality is reserved for schema smoke tests where exact envelope shape is the
behavior under test.

## Boundaries

The harness standardizes CLI integration tests. It does not replace parser,
checker, runtime, or formatter unit tests in compiler crates.

Use the language specification when a case needs to decide whether command,
diagnostic, JSON, runtime, or source behavior is correct. Use this page only
for harness organization and assertion policy.
