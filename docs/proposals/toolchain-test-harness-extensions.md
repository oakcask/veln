# Toolchain Test Harness Extensions

Status: partially implemented

This page routes declarative harness follow-ups left outside the completed
structured CLI integration test harness target. Implemented manifest fields
live in the reference harness documentation.

## Read First

- Implemented harness organization:
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
- Current command and JSON behavior:
  [../specification/commands.md](../specification/commands.md) and
  [../specification/json-output.md](../specification/json-output.md).
- Current JVM execution and cache behavior:
  [../specification/execution.md](../specification/execution.md).
- Use this page only for manifest capabilities that the reference page does
  not list as implemented.

## Completed Slices

The implemented case manifest covers command invocation, fixture setup,
stream and JSON assertions, diagnostic selectors, file assertions, host and
platform gates, command help assertions, and JVM cache assertions and
mutations. Keep field-level details in the
[reference harness manifest section](../reference/toolchain-test-harness.md#manifest-fields).

Do not restate implemented field contracts here. This page keeps only
incomplete or unsplit harness-extension work.

## Current Target

No smaller target is selected on this page. The command help assertion slice is
complete; use [target-selection.md](target-selection.md) for the current
proposal-level target status.

## Open Follow-Ups

Candidate follow-ups include broad diagnostic detail checks and setup rules
that replace repeated bespoke CLI test code across at least two command paths.
Move any large follow-up to its own proposal page before implementation.

## Non-Goals

- Do not let case manifests execute arbitrary shell commands.
- Do not encode complete backend cache layout, classfile bytes, or compiler
  internals as stable fixture expectations.
- Do not replace bespoke tests whose setup is unique to one command path.
- Do not change command behavior, diagnostic ids, or JSON schemas while adding
  harness features.

## Acceptance Checks

- New manifest features replace at least two bespoke CLI setup or assertion
  patterns.
- The reference page documents each new manifest field and its boundary.
- Existing `toolchain_cases/` fixtures continue to run without changing their
  meaning.
- Behavior-specific assertions still point to the specification page that owns
  the command or JSON rule under test.
- Harness assertions stay out of `../specification/` unless they describe
  implemented language or command behavior rather than harness behavior.

## Update When

- Document a completed manifest extension in the reference page after code and
  tests support it.
- Keep future declarative harness features on this page until a smaller
  proposal page is useful.
