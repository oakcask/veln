# Proposals

This directory catalogs planned or accepted work that is not fully documented
as current behavior under `../specification/`. Proposal text is not current
language behavior unless the matching specification page also states it.

Use this page as a catalog only. Pick the proposal that matches the task, then
compare it with `../specification/` before changing behavior.

## Catalog

- [toolchain-test-harness-extensions.md](toolchain-test-harness-extensions.md):
  declarative test harness and command analysis follow-ups.
- [doctest-runtime-failure-expectations.md](doctest-runtime-failure-expectations.md):
  future runtime-failure doctest expectation kinds beyond the implemented
  `runtime=contract`, `runtime=ensure`, and `runtime=result` routes.
- [path-runtime-representation.md](path-runtime-representation.md):
  runtime `Path` representation work.
- [adt-generalization-route.md](adt-generalization-route.md):
  staged route from descriptor-backed `Option` and `Result` to ADTs, `List`,
  immutable collection helpers, and trampoline execution.
- [immutable-collection-trampoline.md](immutable-collection-trampoline.md):
  internal trampoline follow-up for source-authored immutable collection
  helpers after the ADT and `List` route.
- [agent-repair-loop-followups.md](agent-repair-loop-followups.md):
  remaining repair-loop axes for verification orchestration, candidate
  evidence, edit granularity, and application authority.
- [agent-module-package-docs.md](agent-module-package-docs.md):
  package metadata, generated documentation, and export-model follow-ups.
- [agent-language-surface-expansion.md](agent-language-surface-expansion.md):
  future language surface features outside the implemented subset.
- [reference-followups.md](reference-followups.md):
  broad follow-up inventory that should be split into narrower proposal pages
  before implementation.
- [toolchain-dependency-graph-signal.md](toolchain-dependency-graph-signal.md):
  crate dependency graph metrics as refactor signals, including CI annotation
  and summary output.

## Update When

- New proposal work is added, split, superseded, completed, or removed.
- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
