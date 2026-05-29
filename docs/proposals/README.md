# Proposals

This directory keeps proposal routes, proposal candidates, implemented proposal
records, and historical evidence that still matters for follow-up work.
Proposal text is not current language behavior unless `../specification/` also
states it.

## Read First

- Need implementation or promotion mechanics for proposal work:
  [implementation-route.md](implementation-route.md).
- Shared command analysis for `check`, `run`, `test`, and `repair`:
  [project-analysis-pipeline.md](project-analysis-pipeline.md).
- Runtime-failure doctest route:
  [doctest-runtime-failure-expectations.md](doctest-runtime-failure-expectations.md).
- Status labels: [../document-status.md](../document-status.md).

## Proposal Flow

1. Start with the proposal page that matches the task. All proposal pages are
   available work routes unless their own status says they are implemented,
   closed, superseded, or rejected.
2. Compare the proposal with `../specification/` before changing code. Stop
   when the specification already states the behavior.
3. Use [implementation-route.md](implementation-route.md) for implementation,
   promotion, and cleanup checks.
4. Use the categorized route list below when the task names a proposal area.

## Choose A Route

- Shared command analysis and command parity:
  [project-analysis-pipeline.md](project-analysis-pipeline.md).
- Doctest runtime expectations:
  [doctest-runtime-failure-expectations.md](doctest-runtime-failure-expectations.md).
- Test harness work:
  [toolchain-test-harness-extensions.md](toolchain-test-harness-extensions.md).
- Runtime, backend, and path representation work:
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md) and
  [path-runtime-representation.md](path-runtime-representation.md).
- Repair workflow and design-wall follow-ups:
  [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md) and
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md).
- Library and formatter follow-ups:
  [self-hosting-standard-library.md](self-hosting-standard-library.md) and
  [formatter-stabilization.md](formatter-stabilization.md).
- Mixed follow-up inventory:
  [reference-followups.md](reference-followups.md).

## Read When

- Checking whether a proposal page still describes absent behavior or must stay
  as history, inventory, or a candidate pool.
- Checking completed prelude helper migrations before choosing more helper
  migration work.
- Checking historical gaps that have been revalidated as proposal work.
- Checking residual scope that has been split into a short proposal route.
- Checking completion evidence before changing target status.

## Update When

- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A candidate's target class changes.
- New proposal work is added, split, superseded, or removed.
- Historical evidence discovered elsewhere belongs in the matching proposal or
  reference page.

## Skip Unless Needed

- Use `../specification/` when you need current implemented behavior.
- Do not open `*-full.md` proposal records until a short proposal page names
  the section needed for the task.
- Do not read implemented proposal records before the matching specification
  page unless you are checking history, evidence, or cleanup.
