---
role: implementation-record
authority: supporting
update-when: The implemented metrics report, its executable evidence, or the decision to keep enforcement out of scope changes.
---

# Built-In Code Metrics

## Completion Summary

The `veln metrics` command measures and reports maintainability signals for
project-owned Veln source. Current behavior is specified in
[commands.md](../../specification/commands.md) and
[metrics-json.md](../../specification/metrics-json.md).

The implemented command reports:

- module fan-in, fan-out, and dependency pressure;
- internal dependency edges and cycles;
- external dependency counts;
- advisory ABC size;
- experimental exact whole-body similarity;
- stable human and JSON output with bounded detailed findings;
- an opt-in dependency-cycle policy with explicit baseline comparison.

The measurement and reporting surfaces complete this proposal. Making fan-in,
fan-out, dependency pressure, ABC size, or similarity enforceable is not part
of this completed scope.

## Decision Boundary

`veln metrics` remains separate from `veln check`. Metrics are
maintainability evidence whose useful interpretation varies by project.
Measurement does not claim that a reported value proves a defect.

The existing dependency-cycle check remains the only blocking metrics policy.
The other metrics remain advisory. A future change that makes another metric
blocking requires a new proposal with its own subject population, threshold,
baseline behavior, diagnostic contract, and project evidence.

The public Veln metrics command operates on Veln syntax, Veln module
identities, and project analysis artifacts. It does not generalize the
Rust-only `veln-repo-metrics` repository-maintenance tool.

## Implemented Evidence

Executable cases under `../../../examples/specification/metrics/` cover
dependency graph reporting, path selection, stable ordering, ABC size,
dependency-cycle checks and baselines, exact whole-body similarity, JSON
output, invalid configuration, and human-output truncation.

The reusable metrics library has table-driven graph, policy, baseline, ABC,
similarity, ordering, and structural-bound tests. The controlled similarity
benchmark is recorded in
[../../reviews/metrics-similarity-benchmark.json](../../reviews/metrics-similarity-benchmark.json).

The repository keeps these verification routes:

```sh
bash scripts/agent-test -p veln-metrics
bash scripts/agent-test -p veln-cli --test toolchain_harness
bash scripts/agent-run cargo run --locked -p veln-cli -- metrics --json path/to/project
bash scripts/agent-run scripts/benchmark-metrics-similarity --runs 3
```

## Non-Goals

- Do not make metric limits part of `veln check`.
- Do not claim that one threshold is universally healthy for every project.
- Do not describe ABC as a direct complexity or defect measure.
- Do not enforce fan-in, fan-out, dependency pressure, ABC size, or similarity
  under this completed proposal.
- Do not automatically rewrite functions, split modules, deduplicate code, or
  update a baseline.
- Do not measure dependencies inside external packages.
- Do not use a combined maintainability score that hides the reported facts.
