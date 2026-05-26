# Toolchain Test Harness

Status: open-proposal
Implementation: not implemented

This is the routing page for a proposed structured integration test harness
for the Veln command-line toolchain. It is not a source for current behavior;
use `../reference/language/` for implemented command, diagnostic, and JSON
output rules.

## Read First

- Current command behavior:
  [../reference/language/commands.md](../reference/language/commands.md).
- Current JSON output behavior:
  [../reference/language/json-output.md](../reference/language/json-output.md).
- Full harness proposal:
  [toolchain-test-harness-full.md](toolchain-test-harness-full.md).

## Proposal Summary

Add a case-based harness that verifies the toolchain as a connected system:

- source files and project discovery
- command parsing and command-specific gates
- parse and semantic diagnostics
- backend generation and external tool invocation
- stdout, stderr, JSON output, and exit status

The proposal standardizes test cases around a `case.toml` manifest, fixture
files copied into a temporary project, and semantic JSON assertions instead of
default full-output equality.

## Read When

- Use this page when deciding how to organize command-line integration tests.
- Open [toolchain-test-harness-full.md](toolchain-test-harness-full.md) when
  implementing the harness, adding manifest fields, or migrating existing CLI
  tests.
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

- Do not treat this proposal as implemented behavior unless the reference also
  states it.
- Do not open the full proposal when the current reference already answers the
  command behavior question.
