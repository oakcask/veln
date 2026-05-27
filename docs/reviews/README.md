# Implementation Reviews

This directory keeps implementation review findings, verification evidence, and
gap lists that should guide follow-up work.

## Read First

- [agent-language-spec-wall-completion.md](agent-language-spec-wall-completion.md)
  records the completion review for the current design-wall target route.
- [no-proposal-target-completion.md](no-proposal-target-completion.md)
  records the completion review for the current no-target prompt state.
- [expected-error-doctest-completion.md](expected-error-doctest-completion.md)
  records the completion review for expected-error doctest examples.
- [toolchain-test-harness-completion.md](toolchain-test-harness-completion.md)
  records the completion review for the structured CLI integration test
  harness target.
- [opaque-path-boundary-review.md](opaque-path-boundary-review.md)
  records the completion review for the self-hosting standard library `Path`
  boundary target.
- [first-slice-gap-review.md](first-slice-gap-review.md)
  routes the first-slice review evidence. It points to the current reference,
  follow-up targets, and the full historical gap review.

## Read When

- Use this directory before claiming an implementation phase is complete.
- Use `../phases/` for the implementation plan and intended ordering.
- Use `../proposals/` for accepted targets that still need implementation.
- Use `../reference/source-decisions/` and
  `../proposals/agent-language-spec-wall/` for original decision rationale.

## Skip Unless Needed

- Do not read full review evidence before the short review route identifies a
  gap or verification note relevant to the task.
- Do not use review notes as current behavior when `../reference/language/`
  states a newer implemented rule.
