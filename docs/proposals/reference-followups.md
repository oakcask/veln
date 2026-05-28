# Reference Follow-Ups

Status: proposed

This page collects follow-up work that remains outside current specification
behavior. Proposal text is not current behavior unless `../specification/`
also states it.

## Read First

- Current behavior: [../specification/README.md](../specification/README.md).
- Shared command analysis target:
  [project-analysis-pipeline.md](project-analysis-pipeline.md).
- Runtime-failure doctest target:
  [doctest-runtime-failure-expectations.md](doctest-runtime-failure-expectations.md).
- Declarative CLI harness follow-up:
  [toolchain-test-harness-extensions.md](toolchain-test-harness-extensions.md).
- Path runtime representation follow-up:
  [path-runtime-representation.md](path-runtime-representation.md).
- Implemented formatter follow-up record:
  [formatter-stabilization.md](formatter-stabilization.md).

## Follow-Up Targets

This page is an index, not one implementation target. A listed area should
route to one short proposal page before implementation work starts.

- Shared project analysis across command entry points:
  [project-analysis-pipeline.md](project-analysis-pipeline.md).
- Runtime-failure doctest expectations:
  [doctest-runtime-failure-expectations.md](doctest-runtime-failure-expectations.md).
- Declarative CLI harness features beyond the implemented case manifest:
  [toolchain-test-harness-extensions.md](toolchain-test-harness-extensions.md).
- Runtime `Path` representation beyond the current source-visible assignment
  boundary:
  [path-runtime-representation.md](path-runtime-representation.md).
- Repair application workflows beyond the implemented confirmation, override,
  and post-edit check boundary:
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md).
- Backend replacement work covered by
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md).
- Self-hosting standard library expansion covered by
  [self-hosting-standard-library.md](self-hosting-standard-library.md).

## Update When

- Move a target into `../reference/` only after current code and tests support
  it.
- Remove a target from this page when the matching specification page fully states
  the implemented behavior.
- Keep implemented records only when they route useful history or completion
  evidence without restating current behavior.
- Keep remaining proposed implementation work in this page or the matching
  short proposal page.
