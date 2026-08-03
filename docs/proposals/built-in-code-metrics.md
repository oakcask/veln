---
review-when: The metrics scope, acceptance evidence, enforcement graduation criteria, or implementation status changes.
---

# Built-In Code Metrics

Status: proposed

## Summary

Extend the implemented `veln metrics` command with the remaining evidence
needed for dependency-pressure policy and stable, bounded analysis. Current
behavior is specified in [commands.md](../specification/commands.md),
[commands-full.md](../specification/commands-full.md), and
[metrics-json.md](../specification/metrics-json.md).

The implemented command already reports advisory module dependency metrics,
advisory ABC size metrics, dependency-cycle policy checks with explicit
baselines, experimental exact whole-body similarity with structural bound
evidence, a controlled similarity benchmark command, and bounded human-output
findings through `[tool.metrics] max_findings`.

## Remaining Scope

The remaining proposal work is limited to:

- dependency-pressure policy evidence and any policy surface that follows from
  that evidence;
- stable-ordering evidence beyond the executable cases that currently cover
  the implemented report shape;
- enforcement-graduation evidence for any advisory metric that becomes a
  blocking policy.

Remaining work must not redefine the current dependency graph, ABC, baseline,
similarity, JSON, or human-output truncation contracts without updating the
current specification and executable metrics cases.

## Motivation

AI-generated changes can add locally plausible code while gradually increasing
function size, ownership pressure, dependency cycles, and repeated logic. These
properties cross file boundaries and are easy to miss when a review focuses on
functional output.

The repository already uses `veln-code-metrics` to guide Rust refactoring. That
experience supports dependency cycles as a blocking ownership signal. ABC and
dependency pressure remain review signals until Veln-specific evidence justifies
enforcement.

## Decision

Keep metrics separate from `veln check`.

`veln check` answers whether source satisfies language rules. Code metrics are
maintainability signals whose useful limits vary by project. A metrics policy
must be opted into through `veln metrics --check` and project configuration.

Do not generalize the Rust-only `veln-code-metrics` executable into the public
command. Veln metrics must operate on Veln syntax, Veln module identities, and
project analysis artifacts.

## Remaining Policy Model

Dependency pressure remains advisory until the proposal records all evidence
needed to make it enforceable.

A dependency-pressure policy proposal must state:

- the exact subject population;
- the measured fact and threshold;
- how generated modules, selected paths, external imports, and baseline
  comparison affect evaluation;
- the human diagnostic text and JSON violation shape;
- pass and fail cases for equal, improved, worsened, renamed, generated, and
  selected-path boundaries.

The policy must not block a check until the enforcement-graduation evidence
below is complete.

## Enforcement Graduation

A later slice may make an advisory metric enforceable only after it provides
project evidence for the exact subject population, metric mapping, threshold,
and diagnostic text that will become policy.

The evidence set must satisfy all of these conditions:

1. It analyzes representative hand-maintained Veln projects and separates
   functions, tests, and generated subjects.
2. A maintainer labels every finding when the set contains at most 100
   findings. For a larger set, the maintainer labels a deterministic sample of
   100 findings.
3. Each label is `action-required`, `advisory-useful`, or `not-useful`, with a
   short reason.
4. Fewer than 10 percent of the reviewed findings are `not-useful` before the
   metric can block a check.
5. The proposed blocking condition rejects a known worsened case and passes an
   equal case, an improved case, a subject rename that preserves the intended
   allowance, and a relevant subject-kind boundary.
6. The proposal records how the threshold was derived from project evidence.
   It must not adopt the Rust tool's threshold or a literature value without
   Veln-specific validation.
7. The human diagnostic identifies the measured fact and a concrete review
   action. It must not claim that a metric alone proves a defect.

The evidence belongs in a review record, not in the current behavior
specification. A metric that misses any graduation condition remains advisory.

## Implemented Similarity Bounds

Bounded exact whole-body similarity analysis is no longer remaining proposal
scope. Current JSON behavior, structural bounds, executable evidence, and the
explicit benchmark command are specified in
[metrics-json.md](../specification/metrics-json.md).

## Acceptance Cases

Planned executable cases follow the placement rules in
`../../examples/specification/README.md` and use capability-specific case names.

| Case | Input distinction | Required observation |
| --- | --- | --- |
| Dependency pressure policy | Modules include high fan-in only, high fan-out only, and high pressure cases with a configured policy threshold | Only the configured pressure violation fails `--check`; human and JSON output name pressure, fan-in, fan-out, and the affected module |
| Baseline pressure allowance | A reviewed baseline contains an equal or worse pressure value for the same module identity | The check passes for equal or improved pressure and fails when pressure worsens beyond the baseline allowance |
| Stable ordering expansion | Discovery order and path separator representation vary for graph, ABC, and similarity subjects | JSON findings and human prefix order are identical across equivalent inputs |

CLI parsing, human output, JSON shape, and exit status must have integration
coverage in `veln-cli`. Metric calculation must have table-driven unit coverage
in a reusable metrics library.

## Implementation Guidance

This section is not normative except where the bounded-analysis requirement
constrains the implementation result.

Reuse `veln-project` discovery and the lowered surface AST. Preserve per-source
module identity before merged project analysis discards file grouping.

Use source `UseOrigin` to exclude implicit prelude edges. Compute graph values
from canonical module identities rather than textual aliases. Derive similarity
tokens from the Veln lexer so formatting and comments do not become semantic
input.

An implementation can group complete body fingerprints to keep similarity
analysis bounded. It must verify candidate declarations against their complete
normalized token sequences before reporting equality. A fingerprint collision
must not create a finding.

The existing Rust-only `veln-code-metrics` remains a repository maintenance
tool. Do not make the public command depend on `syn` or accept Rust source.

## Non-Goals

- Do not change Veln syntax, typing, effect, contract, or runtime semantics.
- Do not make metric limits part of `veln check`.
- Do not claim that one threshold is universally healthy for every project.
- Do not describe ABC as a direct complexity or defect measure.
- Do not enforce ABC, fan-in, fan-out, pressure, or similarity without
  enforcement-graduation evidence.
- Do not automatically rewrite functions, split modules, or deduplicate code.
- Do not automatically update a baseline during `--check`.
- Do not measure dependencies inside external packages.
- Do not include generated or doctest-derived declarations in ABC or
  similarity policy subjects.
- Do not add partial-body, approximate, identifier-insensitive, or semantic
  clone detection under this proposal.
- Do not use a single combined maintainability score that hides the underlying
  measurements.

## Planned Verification Commands

Implementation must keep these repository-relative checks available:

```sh
bash scripts/agent-test -p veln-metrics
bash scripts/agent-test -p veln-cli --test toolchain_harness
bash scripts/agent-run cargo run --locked -p veln-cli -- metrics --json path/to/project
bash scripts/agent-run scripts/benchmark-metrics-similarity --runs 3
```

The metrics crate, metrics executable cases, and exact whole-body similarity
executable cases exist. The benchmark command exists as explicit review
evidence.

## Completion Boundary

This proposal is complete only when all remaining acceptance cases pass and
any new policy has repository review evidence.

Completion must add current command behavior to
`../specification/commands-full.md`, route it from
`../specification/commands.md`, specify JSON behavior through
`../specification/json-output.md`, and place named executable cases according
to `../../examples/specification/README.md`.

After completion, move this document to
`../reference/implemented-proposals/` and remove it from the proposal catalog.
