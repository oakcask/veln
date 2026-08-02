# Bounded Toolchain Analysis

Status: proposed

## Summary

Reduce repeated whole-project analysis in CLI integration cases and keep
standard-library analysis growth bounded as imported module graphs grow.

This proposal changes toolchain implementation and test infrastructure. It
does not change Veln source semantics, diagnostics, command output, or the JVM
runtime contract.

## Motivation

The [observed GitHub Actions run](https://github.com/oakcask/veln/actions/runs/30751102937)
reported 199 slow tests. Their median duration was 24.62 seconds. Of those
tests, 185 exercised the HTTP/2 core, protocol, connection, or service
surfaces.

Controlled local measurements isolated the cost to project analysis:

| Workload | `veln check` wall time | Full operation wall time |
| --- | ---: | ---: |
| Schema decode example | 0.36 seconds | `veln run`: 0.43 seconds |
| HPACK example | 1.01 seconds | toolchain case: 2.87 seconds |
| HTTP/2 core example | 6.58 seconds | toolchain case: 13.56 seconds |
| HTTP/2 connection example | 7.77 seconds | CI case: up to 55.22 seconds |

These values are diagnostic evidence, not portable performance guarantees.
The measurements used a prebuilt debug toolchain and one workload at a time.

Each specification case currently performs an independent source-error guard
before it invokes the CLI. Both operations analyze the copied project. A large
standard-library import therefore pays the project-analysis cost at least
twice. Parallel nextest workers amplify the elapsed duration when several
analysis-heavy cases compete for the same processors.

The analysis pipeline also constructs a type environment for the merged
application and standard-library surface. Type and effect inference performs
repeated whole-module scans until results stabilize. The HTTP/2 standard
library makes this behavior visible because its production sources contain a
large module graph and many functions.

## Proposed Outcome

The toolchain must preserve all command-visible results while avoiding an
independent second whole-project analysis for a single toolchain case.

Analysis work must scale with the declarations and dependency relationships
that can affect the result. Adding unrelated, fully annotated modules must not
cause repeated scans of every previously analyzed function during each
inference round.

These are toolchain performance requirements. They do not make compilation
time part of the Veln language semantics.

## Required Behavior

| Requirement | Observable condition | Planned primary evidence |
| --- | --- | --- |
| CLI compatibility | Existing toolchain cases retain their exit status, stdout, stderr, JSON values, diagnostics, and generated files | Existing `veln-cli` toolchain suite |
| Source-error protection | A specification example with an unexpected source error still fails before a runtime result can be accepted as valid evidence | Harness regression case with an injected source error |
| One project-analysis payment | A normal `check`, `run`, or `test` toolchain case does not perform a separate whole-project analysis solely for the source-error guard | Harness unit test with an analysis invocation counter |
| Project isolation | Analysis reused for one copied project is not reused after source text, manifest data, command inputs, or dependency identity changes | Cache invalidation and concurrent-project unit tests |
| Determinism | Repeated and concurrent analysis returns diagnostics in the same stable order and does not share mutable project state | Repeated and concurrent analyzer tests |
| Bounded growth | Doubling an unrelated fully annotated module set does not produce superlinear repeated whole-module inference work | Generated high-cardinality analysis benchmark |
| Representative improvement | HTTP/2 core and connection workloads become materially faster without weakening their assertions | Controlled before-and-after benchmark described below |

The existing toolchain suite remains authoritative for command behavior. The
new counter tests and benchmark are authoritative only for analysis reuse and
performance properties.

## Source-Error Guard Contract

The harness must continue to reject unexpected source diagnostics in
specification examples. The harness may obtain that evidence from the command
analysis result, a shared analysis result, or another equivalent checked
artifact.

The harness must not launch an independent whole-project analysis when the
same case invocation already produces authoritative source diagnostics. This
internal constraint is required because the duplicated analysis is a measured
source of the regression.

If a command intentionally expects source errors, its existing manifest
expectations remain authoritative. If a command expects a runtime failure, a
source error must not satisfy that runtime expectation.

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

## Performance Acceptance Model

Implementation must add `scripts/benchmark-toolchain-analysis`. The script
will build or select one debug toolchain binary before measurement. It will
measure these tracked workloads:

- a small schema example that does not import HTTP/2;
- the HPACK static codec boundary example;
- the HTTP/2 protocol closed-input example;
- the HTTP/2 connection application example;
- generated fully annotated module graphs at three adjacent sizes.

For each workload, the script will perform one warm-up run and five measured
runs. It will report wall time, user CPU time, and the median for each metric.
It will write machine-readable JSON when an output path is supplied.

Before-and-after binaries must run on the same machine with the same build
profile. Runs must alternate between the two binaries to reduce ordering bias.
If the median absolute deviation of wall time exceeds ten percent of the
median, the result is noisy and must be repeated before it is used as
acceptance evidence.

The implementation is accepted when all of these comparisons pass:

| Comparison | Required result |
| --- | --- |
| HTTP/2 core direct analysis | New median wall time is at most one third of the baseline median |
| HTTP/2 connection direct analysis | New median wall time is at most one third of the baseline median |
| Toolchain case versus its direct CLI invocation | Toolchain-case median is no more than 1.35 times the direct-invocation median |
| First generated size versus second size | Doubling declarations increases median user CPU time by no more than 2.5 times |
| Second generated size versus third size | Doubling declarations increases median user CPU time by no more than 2.5 times |
| Functional outputs | Before-and-after exit status and normalized output are equal for every tracked workload |

Hosted CI wall time is not a stable acceptance threshold. CI will continue to
report nextest slow cases, but a particular hosted-run duration will not fail
this proposal. Structural regression tests must run in normal CI. The
controlled benchmark remains a review and completion artifact until a stable
dedicated benchmark runner exists.

## Implementation Guidance

This section is not normative except where the contracts above require
isolation or prohibit duplicate whole-project analysis.

The implementation should first measure parsing, type-environment
construction, semantic checks, reachable lowering, JVM generation, and
process execution separately. It should optimize the measured dominant stage.

Likely implementation options include:

- preserve source diagnostics from the real command analysis for harness
  validation;
- precompute immutable standard-library declarations and signatures;
- reuse standard-library analysis within a process;
- infer only declarations with omitted information;
- replace repeated global inference scans with dependency-directed work;
- rebuild only analysis entries affected by changed input identity.

The implementation may combine these options. It must not weaken diagnostic
coverage to meet the performance target.

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

The benchmark command does not exist until this proposal is implemented.

## Completion Boundary

This proposal is complete only when the functional acceptance cases pass, the
structural regression tests run in CI, and the controlled benchmark meets all
comparison thresholds. Completion must also update
`../reference/toolchain-test-harness.md` with the implemented source-error
guard and analysis-reuse policy.

After completion, move this document to
`../reference/implemented-proposals/` and remove it from the proposal catalog.
