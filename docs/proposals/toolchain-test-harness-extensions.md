# Toolchain Test Harness Extensions

Status: proposed

This page records the declarative harness follow-up left outside the completed
structured CLI integration test harness target. It is proposal work, not
current test-harness behavior.

## Read First

- Implemented harness organization:
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
- Implemented manifest completion notes are summarized below.
- Current command and JSON behavior:
  [../specification/commands.md](../specification/commands.md) and
  [../specification/json-output.md](../specification/json-output.md).

## Current Boundary

The manifest covers reusable command invocation, exit status, stream
fragments, semantic JSON assertions, diagnostic selectors, JDK requirements,
platform skips, fixture copying, fixed environment variables, and repeated
invocations inside one isolated project.

Case discovery walks `tests/toolchain_cases/` for `case.toml` files and runs
each case in a temporary project. Fixture copying treats the case directory as
the project tree and excludes only `case.toml`. JSON output is parsed and
checked semantically, so cases assert stable paths and diagnostic fields
instead of relying on full JSON string equality.

Existing declarative cases cover check, run, and test behavior including valid
JSON output, human diagnostics, type diagnostics, nested discovery, ignored
build output, manifest module drift, entry resolution, entry argument errors,
JSON contract failure, no discovered tests, source-to-test convention
selection, static gate blocking, doctest expected output, and runtime stdio
capture.

Bespoke Rust integration tests still own setup that the manifest cannot
describe cleanly, including fake tool installation, generated-cache side-effect
inspection, command help assertions, formatter mutation checks, and broad
diagnostic detail checks.

## Target

Add one narrow declarative capability at a time when multiple bespoke CLI
integration tests need the same setup or assertion pattern.

Preferred first targets are:

- Fake external tool setup for commands that need controlled JVM tool
  availability.
- Cache state assertions that can inspect command-visible generated-cache
  behavior without exposing backend cache internals as language facts.
- File mutation assertions for commands such as formatter or repair checks
  that intentionally write source files.

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

- Move implemented harness organization into
  `../reference/toolchain-test-harness.md` after code and tests support the new
  manifest feature.
- Keep future declarative harness features on this page until a smaller
  proposal page is useful.
