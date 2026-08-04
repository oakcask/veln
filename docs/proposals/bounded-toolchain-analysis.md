---
review-when: The standard-library analysis growth evidence, benchmark scope, or implementation status changes.
---

# Bounded Toolchain Analysis

Status: proposed

## Summary

Keep standard-library analysis growth bounded as imported module graphs grow.

This proposal changes toolchain implementation and test infrastructure. It
does not change Veln source semantics, diagnostics, command output, or the JVM
runtime contract.

## Current Boundary

Completed slices and their evidence are historical records, not remaining
proposal scope:

- CLI harness source-error artifact isolation, controlled benchmark harness,
  private-signature inference bounds, function/private-handler effect
  inference bounds, shared-analysis determinism, reusable standard-library
  semantic signatures, and stage-resolved benchmark measurement are recorded in
  [../reference/implemented-proposals/bounded-toolchain-analysis-slices.md](../reference/implemented-proposals/bounded-toolchain-analysis-slices.md).
- The controlled stage-timing evidence is recorded in
  [../reviews/toolchain-analysis-stage-benchmark.json](../reviews/toolchain-analysis-stage-benchmark.json).
- The existing toolchain suite remains authoritative for command behavior.

The remaining proposal scope starts after those slices. It is limited to
bounded reachable-entry selection and lowering work for representative HTTP/2
applications, plus the benchmark evidence that shows the proposal's
representative HTTP/2 acceptance thresholds are met.

## Motivation

The HTTP/2 standard library exposes high analysis cost because production
sources contain a large module graph and many functions. Prior completed
slices bounded several repeated semantic-analysis paths and made the benchmark
harness capable of measuring pipeline stages without changing normal CLI
stdout, stderr, JSON, exit status, or generated output.

The stage-resolved review record shows that the remaining dominant measured
stage for both representative HTTP/2 workloads is
`reachable_entry_lowering`. The HTTP/2 core workload recorded a
0.158566097 second median for that stage, ahead of semantic environment
construction and checking at 0.10378384 seconds. The HTTP/2 connection
workload recorded a 5.135779555 second median for that stage, ahead of the
backend and runtime remainder at 0.222157528 seconds. That evidence selects
the next implementation slice. It does not complete the proposal.

## Proposed Outcome

Analysis work must scale with the declarations and dependency relationships
that can affect the result. Adding unrelated, fully annotated modules must not
cause repeated scans of every previously analyzed function during each
inference round or reachable lowering operation.

These are toolchain performance requirements. They do not make compilation
time part of the Veln language semantics.

## Required Behavior

| Requirement | Observable condition | Planned primary evidence |
| --- | --- | --- |
| CLI compatibility | Existing toolchain cases retain their exit status, stdout, stderr, JSON values, diagnostics, and generated files | Existing `veln-cli` toolchain suite |
| Project isolation | Analysis reused for one copied project is not reused after source text, manifest data, command inputs, or dependency identity changes | Cache invalidation and concurrent-project unit tests |
| Determinism | Repeated and concurrent analysis returns diagnostics in the same stable order and does not share mutable project state | Repeated and concurrent analyzer tests |
| Bounded reachable lowering | Adding unrelated fully annotated modules does not produce superlinear reachable-entry selection or lowering work | Generated high-cardinality analysis benchmark and structural lowering counters |
| Representative improvement | HTTP/2 core and connection workloads become materially faster without weakening their assertions | Controlled before-and-after benchmark described below |

The existing toolchain suite remains authoritative for command behavior. New
cache, lowering, and benchmark tests are authoritative only for analysis reuse
and performance properties.

## Analysis Reuse Contract

Reusable analysis data must be immutable or isolated from command-specific
mutation. A reuse key must distinguish all inputs that can change diagnostics
or lowering results, including source text, package identity, manifest data,
toolchain standard-library identity, and analysis mode.

A cache miss must produce the same result as analysis without reuse. A stale
entry must never suppress a diagnostic or retain a diagnostic from another
project.

Concurrent callers may share immutable standard-library data. They must not
share mutable application state, inference progress, diagnostic buffers, or
lowering state.

Application analysis caching remains a non-goal for the reachable-lowering
slice. The slice may reuse immutable standard-library facts that are already
implemented, but it must not introduce a persistent daemon, a global on-disk
cache, or mutable cross-project application analysis sharing.

## Performance Acceptance Model

The implemented benchmark harness provides
`scripts/benchmark-toolchain-analysis`. The script compares two prebuilt
toolchain binaries. Toolchain builds stay outside measured runs. The script
records the exact binary and workload command used for every result. It
measures these tracked workloads:

- a small schema example that does not import HTTP/2;
- the HPACK static codec boundary example;
- the HTTP/2 protocol closed-input example;
- the HTTP/2 connection application example;
- generated unrelated fully annotated module graphs at three adjacent doubling
  sizes.

For each workload, the script performs one warm-up run and five measured runs
by default. It alternates the baseline and new binary during comparison. It
reports wall time, user CPU time, and the median for each metric. It writes
deterministic machine-readable JSON when an output path is supplied.

Before-and-after binaries must run on the same machine with the same build
profile. Runs must alternate between the two binaries to reduce ordering bias.
If the median absolute deviation of wall time exceeds ten percent of the
median, the result is noisy and must be repeated before it is used as
acceptance evidence.

The script compares exit status and normalized functional output for measured
runs before it reports a performance result. The controlled benchmark result
is accepted when all of these comparisons pass:

| Comparison | Required result |
| --- | --- |
| HTTP/2 core direct analysis | New median wall time is at most one third of the baseline median |
| HTTP/2 connection direct analysis | New median wall time is at most one third of the baseline median |
| Toolchain case versus its direct CLI invocation | Toolchain-case median is no more than 1.35 times the direct-invocation median |
| First generated size versus second size | Doubling declarations increases median user CPU time by no more than 2.5 times |
| Second generated size versus third size | Doubling declarations increases median user CPU time by no more than 2.5 times |
| Functional outputs | Before-and-after exit status and normalized output are equal for every tracked workload |

The toolchain-case overhead comparison requires a harness command that cannot
be derived from the two CLI binary paths alone. Set
`VELN_TOOLCHAIN_CASE_COMMAND` to the exact command to measure that case. When
the variable is absent, the script reports that comparison as skipped instead
of silently treating it as passing.

Hosted CI wall time is not a stable acceptance threshold. CI will continue to
report nextest slow cases, but a particular hosted-run duration will not fail
this proposal. Structural regression tests must run in normal CI. The
controlled benchmark remains a review and completion artifact until a stable
dedicated benchmark runner exists.

## Remaining Work

- Add bounded reachable-entry selection and lowering for representative HTTP/2
  applications.
- Preserve normal command stdout, stderr, JSON, exit status, diagnostics, and
  generated output.
- Preserve existing functional-output comparisons in the controlled benchmark.
- Add structural coverage that proves unrelated annotated modules do not cause
  unbounded reachable-lowering work.
- Record new controlled benchmark evidence only when functional comparisons
  pass and wall-time noise remains within the accepted boundary.
- Keep application analysis caching out of scope for this proposal slice.

## Non-Goals

- Do not change Veln syntax, typing rules, effect semantics, or runtime
  behavior.
- Do not remove source-error validation from specification examples.
- Do not raise the nextest slow threshold as the primary fix.
- Do not serialize mutable application analysis across unrelated CLI
  processes.
- Do not require a persistent daemon or a global on-disk cache.
- Do not make hosted-run wall time a language compatibility guarantee.
- Do not replace end-to-end CLI cases with compiler unit tests when process
  behavior is part of the case.

## Verification Commands

Planned verification uses these repository-relative commands:

```sh
bash scripts/agent-test -p veln-cli --test toolchain_harness
bash scripts/agent-test -p veln-analysis
bash scripts/agent-test -p veln-sema
bash scripts/benchmark-toolchain-analysis compare BASELINE_BINARY NEW_BINARY
```

## Completion Boundary

This proposal is complete only when the analyzer optimization work lands, the
functional acceptance cases pass, the structural regression tests run in CI,
and the controlled benchmark meets all comparison thresholds.

After completion, move this document to
`../reference/implemented-proposals/` and remove it from the proposal catalog.
