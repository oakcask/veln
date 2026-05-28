# Implementation Reviews

This directory keeps implementation review findings, verification evidence, and
gap lists that guide follow-up work. Use review files as evidence, not as the
source of current language behavior.

## Read First

- Current behavior: [../specification/README.md](../specification/README.md).
- Current proposal target route:
  [../proposals/target-selection.md](../proposals/target-selection.md).
- Open a review only when checking evidence for a completion claim, gap, or
  target-status change.

## Evidence Routes

- No active proposal target:
  [no-proposal-target-completion.md](no-proposal-target-completion.md).
- [jvm-bytecode-backend-completion.md](jvm-bytecode-backend-completion.md)
  records the JVM bytecode backend completion review and source-backend cleanup
  result.
- [formatter-stabilization-completion.md](formatter-stabilization-completion.md)
  records the completion review for the formatter stabilization target.
- [repair-command-completion.md](repair-command-completion.md) records the
  completion review for the repair command proposal promotion.
- [agent-language-spec-wall-completion.md](agent-language-spec-wall-completion.md)
  records the earlier advisory repair candidate boundary review.
- [expected-error-doctest-completion.md](expected-error-doctest-completion.md)
  records the completion review for expected-error doctest examples.
- [toolchain-test-harness-completion.md](toolchain-test-harness-completion.md)
  records the completion review for the structured CLI integration test
  harness target.
- [opaque-path-boundary-review.md](opaque-path-boundary-review.md)
  records the completion review for the self-hosting standard library `Path`
  boundary target.
- [first-slice-gap-review.md](first-slice-gap-review.md)
  routes historical review evidence. It points to the current specification,
  follow-up targets, and the full gap review.

## Read When

- Use this directory before relying on completion claims or gap closure.
- Use `../proposals/` for proposal targets that still need implementation.
- Use `../reference/source-decisions/` and
  `../proposals/agent-language-spec-wall/` for original decision rationale.

## Skip Unless Needed

- Do not read full review evidence before the short review route identifies a
  gap or verification note relevant to the task.
- Do not use review notes as current behavior when `../specification/`
  states a newer implemented rule.
