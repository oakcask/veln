# Toolchain Test Harness

Status: implemented
Implementation: initial harness implemented in `crates/veln-cli/tests/`

This is the routing page for the structured integration test harness for the
Veln command-line toolchain. It is not a source for command behavior; use
`../reference/language/` for implemented command, diagnostic, and JSON output
rules.

## Read First

- Current command behavior:
  [../reference/language/commands.md](../reference/language/commands.md).
- Current JSON output behavior:
  [../reference/language/json-output.md](../reference/language/json-output.md).
- Full harness proposal:
  [toolchain-test-harness-full.md](toolchain-test-harness-full.md).

## Proposal Summary

The case-based harness verifies the toolchain as a connected system:

- source files and project discovery
- command parsing and command-specific gates
- parse and semantic diagnostics
- backend generation and external tool invocation
- stdout, stderr, JSON output, and exit status

The harness standardizes test cases around a `case.toml` manifest, fixture
files copied into a temporary project, and semantic JSON assertions instead of
default full-output equality.

## Read When

- Use this page when deciding how to organize command-line integration tests.
- Open [toolchain-test-harness-full.md](toolchain-test-harness-full.md) when
  adding manifest fields or migrating existing CLI tests.
- Use the current reference pages when checking whether an expected command
  behavior is already implemented.

## Key Boundaries

- The harness should standardize integration tests, not replace compiler crate
  unit tests.
- JSON output should be parsed and checked semantically by default.
- Full JSON equality should be reserved for schema smoke tests.
- Manifests should stay declarative and should not execute arbitrary shell
  commands.

## Skip Unless Needed

- Do not use this proposal page as the command behavior contract.
- Do not open the full proposal when the current reference already answers the
  command behavior question.
