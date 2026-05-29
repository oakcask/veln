# Toolchain Test Harness Extensions

Status: partially implemented

This page records the declarative harness follow-up left outside the completed
structured CLI integration test harness target. It is proposal work, not
current test-harness behavior. Implemented manifest fields live in the
reference harness documentation.

## Read First

- Implemented harness organization:
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
- Current command and JSON behavior:
  [../specification/commands.md](../specification/commands.md) and
  [../specification/json-output.md](../specification/json-output.md).
- Current JVM execution and cache behavior:
  [../specification/execution.md](../specification/execution.md).
- Use this page only for manifest capabilities that the reference page does
  not yet list as implemented.

## Already Implemented

The implemented case manifest already covers command invocation, fixture
setup, repeated invocations inside one isolated project, environment and tool
setup, exit status, stream checks, JSON assertions, diagnostic selectors, file
content assertions, host requirements, and platform skips. Keep field-level
details in
[../reference/toolchain-test-harness.md#manifest-fields](../reference/toolchain-test-harness.md#manifest-fields).

The fake external tool setup slice is implemented and documented in the
reference page. Do not restate that field contract here.

## Current Target

Add declarative cache state assertions for command-visible generated JVM class
cache behavior. This target should let `case.toml` fixtures cover behavior that
bespoke Rust CLI tests currently inspect by hand:

- Cache reuse across repeated `run` or `test` invocations in one isolated
  project.
- New cache entries after source changes.
- Replacement of invalid or incomplete cache entries before execution.

The assertion shape may inspect stable, command-visible cache state only. It
must not expose classfile bytes, complete cache layout, backend cache keys, or
other compiler internals as fixture expectations.

## Later Inventory

Keep later declarative harness features separate from this cache slice. Likely
follow-ups include command help assertions, broad diagnostic detail checks, and
new setup rules that replace repeated bespoke CLI test code across at least two
command paths.

## Non-Goals

- Do not let case manifests execute arbitrary shell commands.
- Do not encode complete backend cache layout, classfile bytes, or compiler
  internals as stable fixture expectations.
- Do not replace bespoke tests whose setup is unique to one command path.
- Do not change command behavior, diagnostic ids, or JSON schemas while adding
  harness features.

## Acceptance Checks

- The new manifest feature replaces at least two bespoke CLI cache setup or
  assertion patterns.
- The reference page documents the new manifest field and its boundary.
- Existing `toolchain_cases/` fixtures continue to run without changing their
  meaning.
- Behavior-specific assertions still point to the specification page that owns
  the command or JSON rule under test.
- Cache assertions stay out of `../specification/` unless they describe
  implemented language or command behavior rather than harness behavior.

## Update When

- Document a completed manifest extension in
  `../reference/toolchain-test-harness.md` after code and tests support it.
- Keep future declarative harness features on this page until a smaller
  proposal page is useful.
