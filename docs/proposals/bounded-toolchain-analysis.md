---
role: proposal
review-when: The standard-library analysis growth evidence, benchmark scope, or implementation status changes.
---

# Bounded Toolchain Analysis

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
- Indexed reachable-function, semantic-function, and ADT candidate lookup is
  implemented. Its controlled comparison is recorded in
  [../reviews/toolchain-analysis-reachable-lookups.json](../reviews/toolchain-analysis-reachable-lookups.json).
- Demand-driven embedded standard-library initialization is implemented. Its
  controlled comparison is recorded in
  [../reviews/toolchain-analysis-demand-standard-library.json](../reviews/toolchain-analysis-demand-standard-library.json).
- Separate application and selected standard-library analysis inputs are
  implemented. Their controlled comparison is recorded in
  [../reviews/toolchain-analysis-separated-standard-inputs.json](../reviews/toolchain-analysis-separated-standard-inputs.json).
- Embedded lowered standard-library modules are implemented. Their controlled
  comparison is recorded in
  [../reviews/toolchain-analysis-embedded-lowered-standard.json](../reviews/toolchain-analysis-embedded-lowered-standard.json).
  The structural first-analysis coverage now observes only the target analysis
  thread, so parallel standard-library analysis in unrelated tests cannot
  contaminate the parse/lower counter. Embedded package initialization keeps
  generated lowered bytes borrowed from the toolchain bundle, and structural
  coverage checks that closure-external generated lowered data does not
  increase selected module materialization, selected lowered bytes decoded, or
  selected standard declarations. The recorded comparison uses the `dfdf2eb7`
  pre-slice binary from `main` and the final `77f0a36e` binary. Functional
  output and wall-time noise passed, but both representative HTTP/2 wall-time
  ratios stayed above the one-third threshold.
- Separate application and selected standard-library inputs are preserved
  through reachable-entry lowering. The controlled comparison is recorded in
  [../reviews/toolchain-analysis-separated-reachable-inputs.json](../reviews/toolchain-analysis-separated-reachable-inputs.json).
  Functional output and wall-time noise passed, and HTTP/2
  `reachable_entry_lowering` medians fell for both representative workloads.
  Structural coverage includes codec `with` target resolution and reachable
  body materialization bounds on the separated input path.
  The representative HTTP/2 wall-time ratios still stayed above the one-third
  threshold.
- The existing toolchain suite remains authoritative for command behavior.

The remaining proposal scope starts after those slices. It is limited to
further work that makes the representative HTTP/2 core and connection
workloads meet their one-third wall-time thresholds, plus final benchmark
evidence that shows every proposal acceptance threshold passes.

## Motivation

The HTTP/2 standard library exposes high analysis cost because production
sources contain a large module graph and many functions. Prior completed
slices bounded several repeated semantic-analysis paths and made the benchmark
harness capable of measuring pipeline stages without changing normal CLI
stdout, stderr, JSON, exit status, or generated output.

The first stage-resolved review record selected `reachable_entry_lowering` as
the next implementation slice. Indexed candidate lookup completed that slice:
the controlled comparison reduced HTTP/2 connection median wall time from
6.580130816 seconds to 1.427894989 seconds and reduced its median reachable
lowering time from 5.216808242 seconds to 0.100644811 seconds.

The same comparison did not complete the proposal. HTTP/2 core median
reachable lowering fell from 0.160090167 seconds to 0.036408763 seconds, but
its median wall time only fell from 1.391151211 seconds to 1.190085222 seconds.
That 0.8554679122 wall-time ratio remains above the required one-third ratio.

Demand-driven embedded standard-library initialization reduced HTTP/2 core
median wall time from 1.191886089 seconds to 0.656577607 seconds. The
0.5508727831 wall-time ratio remains above the required one-third ratio. It
also reduced HTTP/2 connection median wall time from 1.448842086 seconds to
0.950928367 seconds. The 0.6563367921 wall-time ratio remains above the
required one-third ratio in that local comparison. The new stage evidence
identifies `surface_parse_lower` and `semantic_environment_check` as the next
HTTP/2 core hot path.

Separating application and selected standard-library analysis inputs reduced
the HTTP/2 core sum of median `surface_parse_lower` and
`semantic_environment_check` time from 0.537219144 seconds to 0.283280795
seconds. It reduced the same HTTP/2 connection stage sum from 0.60004038
seconds to 0.309804194 seconds. Functional output matched for every measured
workload, wall-time noise stayed within the accepted boundary, and both
representative median wall times also fell. The broader one-third HTTP/2
wall-time thresholds still did not pass, so the proposal remains open.

Embedding per-module lowered standard-library data reduced HTTP/2 core median
`surface_parse_lower` time from 0.045545017 seconds to 0.009942617 seconds and
HTTP/2 connection median `surface_parse_lower` time from 0.049651884 seconds
to 0.010768857 seconds. Functional output matched for every measured workload
and wall-time noise stayed within the accepted boundary. The representative
HTTP/2 wall-time ratios were 0.7855141867 for core and 0.8940777698 for
connection, so the broader one-third wall-time thresholds still did not pass
and the proposal remains open.

Preserving separated inputs through reachable-entry lowering reduced HTTP/2
core median `reachable_entry_lowering` time from 0.015221209 seconds to
0.007923789 seconds and HTTP/2 connection median
`reachable_entry_lowering` time from 0.039002408 seconds to 0.030307844
seconds. Functional output matched for every measured workload and wall-time
noise stayed within the accepted boundary. The representative HTTP/2 wall-time
ratios were 0.943778466 for core and 0.9526936514 for connection, so the
broader one-third wall-time thresholds still did not pass and the proposal
remains open.

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
| Project isolation | Analysis reused for one copied project is not reused after source text, manifest data, command inputs, or dependency identity changes | Implemented cache invalidation and concurrent-project unit tests |
| Determinism | Repeated and concurrent analysis returns diagnostics in the same stable order and does not share mutable project state | Implemented repeated and concurrent analyzer tests |
| Bounded reachable lowering | Adding unrelated fully annotated modules does not increase reachable function-target or ADT-constructor candidate scans | Implemented structural lookup counters and generated high-cardinality analysis benchmark |
| Bounded standard initialization | Adding unrelated standard modules outside the selected closure does not increase first-analysis standard parse/lower or semantic prepare work | Implemented closure-driven standard loading and selected standard-environment tests |
| Separate standard and application inputs | Application analysis keeps application declarations separate from selected standard declarations and builds semantics from application facts plus the selected reusable standard environment | Implemented structural input-separation tests and controlled stage comparison |
| Separate reachable lowering inputs | Reachable-entry lowering preserves the application and selected standard-library surface inputs until after reachability traversal, while keeping reachable function sets, diagnostics, checked core, and typed IR equivalent to the former combined-input path | Implemented separated reachability tests and controlled stage comparison |
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

The baseline binary for each optimization slice must represent the
implementation state immediately before that slice. The new binary must
contain that slice. Ratios from separate slices do not accumulate for
acceptance. The completion comparison uses the state immediately before the
final optimization slice as its baseline, and the review record must identify
both binary states.

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

- Reduce the HTTP/2 core and connection direct-analysis wall-time medians to
  at most one third of the medians from the implementation state immediately
  before the final optimization slice, without regressing completed functional
  comparisons.
- Preserve normal command stdout, stderr, JSON, exit status, diagnostics, and
  generated output.
- Preserve existing functional-output comparisons in the controlled benchmark.
- Keep the completed structural coverage that holds reachable function-target
  and ADT-constructor candidate scans constant as unrelated declarations grow.
- Keep the completed demand-driven standard-library initialization boundary
  that parses, lowers, and prepares only the selected standard-module closure
  during first analysis.
- Keep the completed boundary that standard-library and application
  declarations remain separate analysis inputs after source loading.
- Keep the completed boundary that selected embedded standard modules are
  decoded from generated lowered representations instead of being parsed and
  surface-lowered during application analysis.
- Keep the completed boundary that reachable-entry lowering traverses
  application and selected standard-library inputs separately and materializes
  only reachable functions for lowering.
- Replace the current controlled comparison only when every functional
  comparison passes, wall-time noise remains within the accepted boundary,
  and both representative HTTP/2 wall-time thresholds pass.
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
