---
review-when: The CLI integration harness assertion model or source-error guard evidence changes.
---

# Toolchain Test Harness

This page specifies the implemented CLI integration test harness. It is a
reference for test organization, not a source for command behavior.

## Read First

- Command behavior belongs in
  [../specification/commands.md](../specification/commands.md).
- JSON output behavior belongs in
  [../specification/json-output.md](../specification/json-output.md).

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
  `[help]`, `[[json_assert]]`, `[[result_value_assert]]`,
  `[[diagnostics]]`, `[[file_assert]]`, `[[binary_fixture]]`, and
  `[[output_chunk_list]]`.
- External tool setup: `[tools] java = "missing"`, `"fake-success"`, or
  `"real"`.

## Output Cases

Use `exit`, `[stdout]`, and `[stderr]` for command-visible output. Stream
sections accept `format = "empty"`, `"text"`, or `"json"` where JSON is valid
for stdout, plus `contains` fragments for stable text checks. Use
`[[json_assert]]`, `[[result_value_assert]]`, and `[[diagnostics]]` for
semantic checks inside JSON stdout. `[[result_value_assert]]` reads a rendered
result-failure value string from `value_path`, wraps it as the outer `Err`,
and then checks a parsed value path with either `equals` or `missing = true`.

Use `[help]` for command help output. It checks a help stream, defaulting to
stdout, through stable help fragments instead of full-output equality. Its
fields are `stream`, `summary`, `usage`, `commands`, `arguments`, `options`,
and `contains`. `stream` is `"stdout"` or `"stderr"`. `summary` checks the
first help line, `usage` checks the `Usage:` line, and the list fields check
that the matching section heading and listed entries appear. Help cases should
still use `[stdout]` and `[stderr]` for stream format and emptiness, and should
point behavior questions to the command specification.

Use `[[binary_fixture]]` and `[[output_chunk_list]]` only for test-owned binary
fixture evidence. Binary fixture records compare named program-output lines
against complete lowercase hex, decoded counts, optional consumed counts,
stable fixture errors, and byte diagnostic metadata for truncation or invalid
field checks. Output chunk lists compare a named, ordered sequence of complete
lowercase hex chunks against consecutive program-output lines, including empty
lists and zero-length chunks.

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

Test harness-owned tool setup with harness or runner unit tests. Do not add CLI
cases solely to prove Java launcher setup, because JVM availability and wrapper
mechanics are not Veln command behavior.

JSON output should be parsed and checked semantically by default. Full JSON
equality is reserved for schema smoke tests where exact envelope shape is the
behavior under test.

## Source-Error Guard

Specification examples reject unexpected source diagnostics unless the manifest
sets `source_errors = "expected"` or the command expectation intentionally
checks a source diagnostic. The failure message includes the diagnostic
locations and identifiers needed to clean the example or mark the source error
as intentional.

For normal `check`, `run`, and `test` cases, the harness does not run an
independent whole-project analysis before the CLI invocation. It asks the real
command process to write a harness-owned source diagnostic artifact for the
copied project and run. The artifact is internal test evidence; it does not
change stdout, stderr, exit status, JSON output, generated files, or command
semantics.

The artifact contains checked diagnostics for the copied project root, not only
for the command-selected source inputs. This preserves the source-error guard
for explicit-input `run` and `test` cases that otherwise analyze only a
selected file before execution. Each repeated invocation writes a distinct
artifact path, so one run or copied project cannot satisfy another run's
guard.

Cases with `source_errors = "expected"` keep the independent guard because some
examples intentionally contain source errors outside the command-selected
slice. `doc`, `fmt`, `lsp`, and `repair` also keep the independent guard until
those commands expose equivalent checked evidence to the harness.

## Analysis Cost Evidence

The duplicate source-error analysis removed from normal `check`, `run`, and
`test` cases was the harness-owned `checked_project_diagnostics` call before
the CLI invocation. The real command now produces the checked diagnostic
artifact that the guard reads. Harness boundary tests verify that a clean
copied project does not satisfy a later dirty copied project, and that a
repeated invocation reads the artifact generated for that invocation after the
copied project changes.

Controlled measurements used the same prebuilt debug toolchain for the direct
CLI invocation and the harness case. Each value below is the median of five
measured runs. The previous measurements recorded the small schema direct run
at 0.43 seconds and the HTTP/2 core toolchain case at 13.56 seconds.

After the artifact guard change, representative debug-toolchain observations
were:

| Workload | Direct CLI | Harness case | Ratio |
| --- | ---: | ---: | ---: |
| Binary schema decode step | 0.40 seconds | 0.47 seconds | 1.17 |
| HTTP/2 protocol core closed JSON | 4.22 seconds | 4.32 seconds | 1.02 |

These observations are local review evidence, not CI failure thresholds.

## Toolchain Analysis Benchmark

Use `scripts/benchmark-toolchain-analysis compare BASELINE_BINARY NEW_BINARY`
when reviewing the bounded toolchain-analysis proposal's controlled benchmark.
The command expects prebuilt CLI binaries. It does not build either binary
during measured runs.

The benchmark covers the small schema, HPACK static codec, HTTP/2 core, HTTP/2
connection, and three generated fully annotated module-graph workloads. It
keeps generated projects in temporary storage. When `--output PATH` is
supplied, it writes deterministic JSON with the exact binary path and command
used for every workload.

The optional toolchain-case overhead comparison needs an explicit command
because it is not part of the CLI binary interface. Set
`VELN_TOOLCHAIN_CASE_COMMAND` to include that workload in a comparison run.
Without that environment variable, the benchmark reports the comparison as
skipped.

## Boundaries

The harness standardizes CLI integration tests. It does not replace parser,
checker, runtime, or formatter unit tests in compiler crates.

Use the language specification when a case needs to decide whether command,
diagnostic, JSON, runtime, or source behavior is correct. Use this page only
for harness organization and assertion policy.
