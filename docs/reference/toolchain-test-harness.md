# Toolchain Test Harness

This page specifies the implemented CLI integration test harness. It is a
reference for test organization, not a source for command behavior.

## Read First

- Command behavior belongs in
  [../specification/commands.md](../specification/commands.md).
- JSON output behavior belongs in
  [../specification/json-output.md](../specification/json-output.md).
- This page owns implemented manifest fields. Planned manifest extensions
  belong in
  [../proposals/toolchain-test-harness-extensions.md](../proposals/toolchain-test-harness-extensions.md).

## Read When

- Add a case under `toolchain_cases/` when behavior must be checked through the
  public CLI.
- Change this harness when a manifest needs a reusable assertion shape, command
  environment, repeated invocation, or fixture setup rule.
- JVM backend fixtures exercise the implemented bytecode path by default. Use
  [implemented-proposals/jvm-bytecode-backend.md](implemented-proposals/jvm-bytecode-backend.md)
  for the source-backend cleanup result.

## Case Layout

The CLI harness discovers case directories that contain `case.toml` under
`tests/toolchain_cases/` and `examples/specification/`. Each case is copied
into a temporary project before the command runs, so fixtures stay isolated
from the repository checkout.

Cases are grouped by command or behavior area. The harness owns command
execution, fixture copying, exit-status checks, stream checks, JSON
assertions, diagnostic selectors, and file content assertions.

## Manifest Fields

- Invocation and fixture setup: `command`, `stdin`, `repeat`, `[env]`,
  `[tools]`, `[requires]`, and `[skip]`.
- Observable command results: `exit`, `[stdout]`, `[stderr]`,
  `[help]`, `[[json_assert]]`, `[[diagnostics]]`, and `[[file_assert]]`.
- External tool setup: `[tools] java = "missing"`, `"fake-success"`, or
  `"real"`.

## Output Cases

Use `exit`, `[stdout]`, and `[stderr]` for command-visible output. Stream
sections accept `format = "empty"`, `"text"`, or `"json"` where JSON is valid
for stdout, plus `contains` fragments for stable text checks. Use
`[[json_assert]]` and `[[diagnostics]]` for semantic checks inside JSON stdout.

Use `[help]` for command help output. It checks a help stream, defaulting to
stdout, through stable help fragments instead of full-output equality. Its
fields are `stream`, `summary`, `usage`, `commands`, `arguments`, `options`,
and `contains`. `stream` is `"stdout"` or `"stderr"`. `summary` checks the
first help line, `usage` checks the `Usage:` line, and the list fields check
that the matching section heading and listed entries appear. Help cases should
still use `[stdout]` and `[stderr]` for stream format and emptiness, and should
point behavior questions to the command specification.

## Manifest Policy

Case manifests are declarative. They should describe the command, expected exit
status, expected stdout or stderr fragments, and structured JSON expectations.
They must not execute arbitrary shell commands.

Use `stdin` only for protocol-style command input that is part of the fixture,
such as LSP exchanges. Use `[requires]` for host capabilities the case needs,
and `[skip]` for platform-specific exclusions with an explicit reason.

Use `[env]` for fixed environment variables that belong to the fixture. Use
`repeat` when one isolated project should run the same command more than once.
Repeated invocations can check stable stdout, stderr, exit status, JSON, file
results, and other command-visible state changes.

Use `[tools]` for controlled external tool availability owned by the harness.
The implemented key is `java`, with values `"missing"`, `"fake-success"`, and
`"real"`. `"missing"` runs the command with an isolated tool path that contains
no Java launcher. `"fake-success"` installs a harness-owned Java wrapper that
exits successfully without running arbitrary manifest code. `"real"` exposes
the host Java launcher under the isolated tool path; cases that use it should
also declare `[requires] jdk = true`.

JSON output should be parsed and checked semantically by default. Full JSON
equality is reserved for schema smoke tests where exact envelope shape is the
behavior under test.

## Boundaries

The harness standardizes CLI integration tests. It does not replace parser,
checker, runtime, or formatter unit tests in compiler crates.

Use the language specification when a case needs to decide whether command,
diagnostic, JSON, runtime, or source behavior is correct. Use this page only
for harness organization and assertion policy.
