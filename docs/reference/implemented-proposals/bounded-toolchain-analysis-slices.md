---
review-when: The bounded toolchain analysis proposal status, completion evidence, or routed review record changes.
---

# Bounded Toolchain Analysis Slices

Status: implemented

This page records completed slices from the bounded toolchain analysis
proposal. Use
[../../proposals/bounded-toolchain-analysis.md](../../proposals/bounded-toolchain-analysis.md)
for remaining planned work and
[../../reviews/toolchain-analysis-stage-benchmark.json](../../reviews/toolchain-analysis-stage-benchmark.json)
for the checked stage-timing review record.

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

The measured HTTP/2 workloads identified `reachable_entry_lowering` as the
dominant remaining stage. The HTTP/2 core workload recorded a 0.158566097
second median for that stage. The HTTP/2 connection workload recorded a
5.135779555 second median for that stage. The next proposal slice is bounded
reachable-entry selection and lowering for representative HTTP/2
applications.

The controlled benchmark evidence did not complete the full proposal because
the representative HTTP/2 connection improvement threshold remained unmet.

## Read When

- Checking why completed bounded-analysis slices are no longer described as
  proposal work.
- Auditing the evidence behind choosing reachable-entry lowering as the next
  implementation slice.
- Preserving the boundary that application analysis caching remains outside
  the measurement slice.
