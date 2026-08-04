---
review-when: The bounded toolchain analysis proposal status, completion evidence, or routed review record changes.
---

# Bounded Toolchain Analysis Slices

Status: implemented

This page records completed slices from the bounded toolchain analysis
proposal. Use
[../../proposals/bounded-toolchain-analysis.md](../../proposals/bounded-toolchain-analysis.md)
for remaining planned work,
[../../reviews/toolchain-analysis-stage-benchmark.json](../../reviews/toolchain-analysis-stage-benchmark.json)
for the stage-timing review record, and
[../../reviews/toolchain-analysis-reachable-lookups.json](../../reviews/toolchain-analysis-reachable-lookups.json)
for the reachable-lookup comparison.

## Completed Scope

The completed slices keep Veln syntax, type rules, diagnostics, command
output, JSON output, generated files, and runtime behavior unchanged. They
only narrow analyzer work or add opt-in measurement.

Completed implementation scope:

- The CLI integration harness no longer performs an independent source-error
  guard analysis for normal `check`, `run`, and `test` cases. The implemented
  policy is specified in
  [../toolchain-test-harness.md](../toolchain-test-harness.md).
- Private return, call-site, and prelude-callback inference skip repeated body
  scans when no omitted private slot can still affect the result.
- Private-reference indexing selects only functions that own eligible private
  slots or reference them in ways that can contribute constraints.
- Function-body effect inference and private-handler retained-effect inference
  share one dependency graph over functions and private handlers.
- Shared project analysis has repeated and concurrent diagnostic-order
  regression coverage across distinct projects with overlapping paths and
  declaration names.
- Application project analysis reuses immutable standard-library semantic
  signatures prepared from the embedded standard-library bundle.
- Cached standard-library facts are cloned only when combining selected
  prepared facts with freshly constructed application facts.
- `veln run` has an opt-in timing path that records source loading, surface
  parsing/lowering, semantic environment construction and checking,
  reachable-entry lowering, and backend/runtime remainder stages.
- `scripts/benchmark-toolchain-analysis` aggregates timing records, reports
  unavailable baseline instrumentation, preserves functional-output
  comparison, validates timing records, and writes deterministic JSON.
- Reachable-function selection indexes functions by name and qualified name,
  and indexes callable targets by name, qualified name, and function shape.
- Semantic function and ADT resolution indexes candidates by function name,
  ADT type name, and variant name while preserving declaration order.

## Evidence

Structural tests cover private-reference candidate filtering,
private-reference indexing, call-site contributor discovery, repeated
inference body traversal, private handler retained effects, stable effect
ordering, unrelated fully annotated module growth, standard-environment
selection, fallback when prepared standard facts are not current, and repeated
and concurrent application analysis.

The controlled stage-timing benchmark used prebuilt debug binaries, one
warm-up run, and five measured runs. The baseline binary had no stage
instrumentation, so baseline stage data is recorded as unavailable while
wall-time and functional comparisons remain active.

The reachable-lookup comparison used prebuilt debug binaries, one warm-up,
and five measured runs. Functional output matched for every workload and no
wall-time result exceeded the accepted noise boundary. Structural tests also
hold function-target and ADT-constructor candidate scans constant when 128
unrelated annotated declarations are added.

For the HTTP/2 connection workload, median `reachable_entry_lowering` time
fell from 5.216808242 seconds to 0.100644811 seconds. Median wall time fell
from 6.580130816 seconds to 1.427894989 seconds, a ratio of 0.2170010033 that
passes the one-third threshold. The two generated-growth ratios were
1.0113636364 and 1.0224719101, and toolchain-case overhead was 1.0585586593;
all passed their thresholds.

The HTTP/2 core workload's median `reachable_entry_lowering` time fell from
0.160090167 seconds to 0.036408763 seconds, but median wall time only fell
from 1.391151211 seconds to 1.190085222 seconds. Its 0.8554679122 ratio does
not pass the one-third wall-time threshold, so the proposal remains open for
that representative workload.

## Read When

- Checking why completed bounded-analysis slices are no longer described as
  proposal work.
- Auditing the evidence behind choosing reachable-entry lowering as the next
  implementation slice.
- Reviewing why reachable candidate indexing is implemented while the HTTP/2
  core acceptance threshold remains proposal work.
- Preserving the boundary that application analysis caching remains outside
  the measurement slice.
