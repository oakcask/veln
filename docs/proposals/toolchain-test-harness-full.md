# Toolchain Test Harness Full

Status: implemented
Implementation: initial harness implemented in `crates/veln-cli/tests/`

This proposal record defines the small, structured integration test harness
for the Veln command-line toolchain. It is not a source for command behavior;
use `../reference/language/` for implemented command, diagnostic, and JSON
output rules.

## Goal

Add a case-based test harness that verifies the toolchain as a connected
system:

- source files and project discovery
- command parsing and command-specific gates
- parse and semantic diagnostics
- backend generation and external tool invocation
- stdout, stderr, JSON output, and exit status

The harness should standardize integration tests early, before ad hoc CLI
fixtures become expensive to migrate.

## Non-Goals

- Replace unit tests in compiler crates.
- Treat proposal text as implemented language behavior.
- Add a free-form shell scripting language for tests.
- Snapshot every byte of JSON output by default.
- Infer language semantics from expected files.

## Case Layout

Each integration test case should live in its own directory:

```text
toolchain_cases/
  check/
    type-mismatch/
      case.toml
      main.veln
  run/
    entry-args/
      case.toml
      main.veln
  test/
    doctest-output/
      case.toml
      main.veln
```

`case.toml` is the entry point. All other files in the case directory are
copied into a temporary project before the `veln` binary is executed. The case
directory is the logical working tree for relative paths in expectations.

## Case Manifest

The manifest should describe command invocation and assertions declaratively:

```toml
command = ["check", "--json", "main.veln"]
exit = 1

[stdout]
format = "json"

[[json_assert]]
path = "status"
equals = "error"

[[json_assert]]
path = "summary.diagnostic_count"
equals = 1

[[diagnostics]]
id = "type.mismatch"
severity = "error"
kind = "type"
message = "expected `Int`, but found `String`"

[diagnostics.span]
file = "main.veln"
line = 2
column = 3
```

The initial manifest surface should stay small:

- `command`: required argv list passed after the `veln` executable.
- `exit`: required process exit code.
- `stdout.format`: `empty`, `text`, or `json`.
- `stdout.contains`: ordered or unordered text fragments for human output.
- `stderr.format`: `empty` or `text`.
- `stderr.contains`: ordered or unordered text fragments for human errors.
- `json_assert`: JSON path assertions for stable top-level fields.
- `diagnostics`: selector-based assertions for diagnostic JSON.
- `requires`: optional environment requirements such as a real JDK.
- `skip`: optional platform or toolchain skip rules.

## JSON Assertion Policy

JSON output should be parsed and checked semantically. Full JSON string equality
should be reserved for schema smoke tests where every byte is intentionally
part of the contract.

Prefer stable assertions for:

- schema version
- command name or report kind
- status
- exit code representation
- diagnostic `id`, `severity`, `kind`, and primary `message`
- stable `details` fields documented by the current reference
- test case status and failure reason
- run JSON error kind

Avoid default assertions on:

- full diagnostic array ordering
- every summary key
- full span objects when only the reported file or line matters
- related note ordering unless the ordering is specified
- backend or host-specific stderr
- tool metadata that is unrelated to the case purpose

Diagnostic assertions should select by `id` and then check only the requested
fields. If more than one diagnostic has the same `id`, the manifest may add
`message`, `span.file`, or `span.line` to disambiguate.

## Harness Responsibilities

The harness should:

- discover case directories containing `case.toml`
- parse manifests into typed Rust structs
- copy fixtures into a temporary project
- run `veln` with the declared argv and working directory
- capture stdout, stderr, and status
- parse JSON when requested
- report assertion failures with the case path and failing field
- keep temporary project paths out of expected output where possible

The harness should not:

- execute arbitrary shell commands from manifests
- encode compiler semantics outside assertions
- mutate expected files during normal test runs
- hide stderr or stdout when a command fails unexpectedly
- require a real JDK unless the case declares that requirement

## Initial Test Groups

The first useful groups are:

### Check

- valid source with empty diagnostics
- parse or type error with diagnostic JSON assertions
- empty input discovery across nested directories
- ignored discovery under build output directories
- manifest and source module name drift

### Run

- entry resolution failure
- entry argument count failure
- primitive command-line argument conversion
- unsupported entry argument type
- missing `javac`
- missing `java`
- JSON report for runtime or contract failure

### Test

- no discovered tests
- explicit target selection
- source-to-test convention selection
- static gate blocks test execution
- passing runtime test
- failing contract test
- expected stdout comparison
- doctest source extraction

### Cache

- repeated generated Java uses the cache
- stale or incomplete cache entries are rebuilt
- cache behavior does not change user-visible output

## Dependency Policy

The harness may use development-only dependencies for structured parsing and
assertions. `serde`, `toml`, and `serde_json` are appropriate because the
manifest and command output are structured formats. The harness should avoid a
large general-purpose snapshot framework until the case format proves it needs
one.

## Migration Plan

1. Add the harness and one representative case for `check --json`.
2. Move or mirror the broadest existing CLI integration tests into case
   directories when they naturally fit the manifest model.
3. Keep bespoke Rust tests for behavior that needs custom process setup, such
   as fake JDK counters, until the manifest supports that setup cleanly.
4. Add diagnostic selector assertions before adding more JSON cases.
5. Add cache and external-tool cases after the basic command groups are stable.

## Open Questions

- Should `stdout.contains` and `stderr.contains` be ordered by default?
- Should JSON path syntax support arrays beyond simple indexes?
- Should fake JDK setup be a manifest feature or remain a Rust helper?
- Should expectation update support be added, or should changes remain manual?
- Should case names appear in test output as one Rust test or as subtests under
  a single runner?
