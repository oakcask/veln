# Proposals

This directory keeps planned work and candidate gates. Completed proposal
records move to `../reference/implemented-proposals/` after their observable
behavior is documented under `../specification/` or checked examples. Proposal
text is not current language behavior unless `../specification/` also states
it.

## Start Here

- No concrete target named, or checking whether a target exists:
  [target-selection.md](target-selection.md).
- Concrete proposal page already named: read that page first, then compare it
  with `../specification/` before changing code.
- Implementation, promotion, or cleanup mechanics after a target is chosen:
  [implementation-route.md](implementation-route.md).
- Status labels and placement rules: [../document-status.md](../document-status.md).

## Stop Rule

- Stop when the matching specification page already states the behavior.
- Do not begin implementation from this index or from
  [reference-followups.md](reference-followups.md) alone.
- Treat this index, target selection, and implementation routing as separate
  steps: this page chooses an area, [target-selection.md](target-selection.md)
  chooses or rejects a concrete target, and
  [implementation-route.md](implementation-route.md) applies only after that
  target exists.
- Keep candidate-gate wording in [target-selection.md](target-selection.md);
  this page only routes to proposal areas.
- Read [implementation-route.md](implementation-route.md) only after one short
  proposal page owns the target.

## Choose A Route

- Tests, doctests, command analysis, and harness work:
  [toolchain-test-harness-extensions.md](toolchain-test-harness-extensions.md),
  [doctest-runtime-failure-expectations.md](doctest-runtime-failure-expectations.md).
- Runtime, backend, and path representation work:
  [path-runtime-representation.md](path-runtime-representation.md).
- Repair workflow and design-wall follow-ups:
  [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md) and
  [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md).
- Library and formatter follow-ups:
  [self-hosting-standard-library.md](self-hosting-standard-library.md).
- Implemented proposal history:
  [../reference/implemented-proposals/README.md](../reference/implemented-proposals/README.md).
- Mixed follow-up inventory:
  [reference-followups.md](reference-followups.md).

## Read When

- Choosing a proposal area after current behavior does not answer the task.
- Checking whether a proposal page still describes absent behavior.
- Checking residual scope before changing target status.

## Update When

- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
- A candidate gate changes, moves to its own page, completes, or is rejected.
- New proposal work is added, split, superseded, or removed.
- Historical evidence discovered elsewhere belongs in the matching reference
  page unless it describes absent behavior.

## Skip Unless Needed

- Use `../specification/` when you need current implemented behavior.
- Do not read implemented proposal records before the matching specification
  page and `../reference/implemented-proposals/` route.
