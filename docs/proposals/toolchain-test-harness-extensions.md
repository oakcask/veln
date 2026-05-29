# Toolchain Test Harness Extensions

Status: proposed

This page records the declarative harness follow-up left outside the completed
structured CLI integration test harness target. It is proposal work, not
current test-harness behavior. The fake external tool setup slice is
implemented in the reference harness documentation.

## Read First

- Implemented harness organization:
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
- Current command and JSON behavior:
  [../specification/commands.md](../specification/commands.md) and
  [../specification/json-output.md](../specification/json-output.md).
- Use this page only for manifest capabilities that the reference page does
  not yet list as implemented.

## Implemented Baseline

The implemented case manifest already covers command invocation, stdin, fixed
environment variables, repeated invocations inside one isolated project,
fixture copying, exit status, stream fragments, semantic JSON assertions,
diagnostic selectors, file content assertions, JDK requirements, and platform
skips. Keep those details in
[../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).

Bespoke Rust integration tests still own setup that the manifest cannot
describe cleanly, including fake tool installation, generated-cache side-effect
inspection, command help assertions, and broad diagnostic detail checks.

## Future Inventory

Keep later declarative harness features separate from completed slices. Likely
follow-ups include:

- Cache state assertions that can inspect command-visible generated-cache
  behavior without exposing backend cache internals as language facts.
- New assertion shapes or setup rules that replace repeated bespoke CLI test
  code across at least two command paths.

## Non-Goals

- Do not let case manifests execute arbitrary shell commands.
- Do not encode complete backend cache layout, classfile bytes, or compiler
  internals as stable fixture expectations.
- Do not replace bespoke tests whose setup is unique to one command path.
- Do not change command behavior, diagnostic ids, or JSON schemas while adding
  harness features.

## Acceptance Checks

- The new manifest feature replaces at least two bespoke CLI setup or
  assertion patterns.
- The reference page documents the new manifest field and its boundary.
- Existing `toolchain_cases/` fixtures continue to run without changing their
  meaning.
- Behavior-specific assertions still point to the specification page that owns
  the command or JSON rule under test.

## Update When

- Document a completed manifest extension in
  `../reference/toolchain-test-harness.md` after code and tests support it.
- Keep future declarative harness features on this page until a smaller
  proposal page is useful.
