---
role: implementation-record
authority: supporting
review-when: The bounded toolchain analysis proposal status, completion evidence, or routed review record changes.
---

# Bounded Toolchain Analysis Slices

This page records completed slices from the bounded toolchain analysis
proposal. Use
[../../proposals/bounded-toolchain-analysis.md](../../proposals/bounded-toolchain-analysis.md)
for remaining planned work,
[../../reviews/toolchain-analysis-stage-benchmark.json](../../reviews/toolchain-analysis-stage-benchmark.json)
for the stage-timing review record, and
[../../reviews/toolchain-analysis-reachable-lookups.json](../../reviews/toolchain-analysis-reachable-lookups.json)
for the reachable-lookup comparison. Use
[../../reviews/toolchain-analysis-demand-standard-library.json](../../reviews/toolchain-analysis-demand-standard-library.json)
for the demand-driven standard-library initialization comparison. Use
[../../reviews/toolchain-analysis-separated-standard-inputs.json](../../reviews/toolchain-analysis-separated-standard-inputs.json)
for the separate application and selected standard-library analysis input
comparison. Use
[../../reviews/toolchain-analysis-embedded-lowered-standard.json](../../reviews/toolchain-analysis-embedded-lowered-standard.json)
for the embedded lowered standard-library module comparison. Use
[../../reviews/toolchain-analysis-separated-reachable-inputs.json](../../reviews/toolchain-analysis-separated-reachable-inputs.json)
for the separated reachable-entry lowering input comparison. Use
[../../reviews/toolchain-analysis-backend-runtime-substages.json](../../reviews/toolchain-analysis-backend-runtime-substages.json)
for the backend/runtime substage timing comparison.

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
- Embedded standard-library source loading parses and lowers only the standard
  module closure reached from the application project and that closure's
  standard imports.
- Reusable standard-library semantic facts are cached by selected standard
  module set. A cache miss prepares immutable standard facts for that set, and
  each application analysis still constructs separate application inference,
  diagnostic, and lowering state.
- Application analysis keeps application declarations and the exact selected
  embedded standard-library closure as distinct inputs after source loading.
  The semantic environment is built from application declarations plus cached
  immutable standard facts keyed by the selected closure. Reachable-entry
  lowering reuses the selected standard surface input so standard bodies remain
  available without making the application module own standard declarations.
- The standard-library build validates generated per-module lowered
  representations against standard-library sources and embeds those
  representations in the toolchain. Application and dependency sources still
  use the runtime parser and surface lowerer, while selected embedded standard
  modules are decoded from the generated lowered representation during
  closure-driven standard input preparation.
- Embedded standard-library package initialization keeps generated lowered
  bytes borrowed from the toolchain bundle. The first application analysis
  decodes only the selected standard-module closure, so increasing generated
  lowered data for an unrelated standard module does not add materialized
  module count, materialized lowered byte count, or semantic declarations.
- Reachable-entry lowering keeps selected standard-library and application
  surface inputs separate until after reachability traversal. The traversal
  indexes and resolves targets across both inputs without cloning them into
  one owned module first. The lowering module materializes only reachable
  application and standard-library functions plus the declarations needed by
  lowering.
- `veln run` records four opt-in backend/runtime substages after
  reachable-entry lowering: JVM classfile generation, JVM class cache
  preparation, Java subprocess execution, and result processing plus cleanup.
  The benchmark harness requires all four substages from the new binary for
  every measured `veln run` workload, while still accepting baseline binaries
  that only report the former `backend_runtime_remainder` timing.

## Evidence

Structural tests cover private-reference candidate filtering,
private-reference indexing, call-site contributor discovery, repeated
inference body traversal, private handler retained effects, stable effect
ordering, unrelated fully annotated module growth, initial standard-package
parse/lower and semantic-prepare work for the selected closure,
selected-lowered byte materialization for the selected closure,
standard-environment selection, fallback when prepared standard facts are not
current, and repeated and concurrent application analysis.
The first-analysis embedded standard-library parse/lower counter is scoped to
the target analysis thread, which keeps the structural assertion stable under
parallel test execution while unrelated standard-library analyses run.
Additional reachable-entry coverage compares separated inputs with the former
combined-input path for reachable function sets, diagnostics, checked core,
and typed IR. It covers standard-library calls, public function aliases, effect
handler traversal, project-local reachability cache isolation, and unchanged
materialized reachable body counts when unrelated fully annotated functions
are added. Separated-input structural coverage also verifies codec `with`
target traversal and confirms that unreachable annotated functions do not add
materialized function bodies before lowering.

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

The demand-driven standard-library initialization comparison used prebuilt
debug binaries, one warm-up run, and three measured runs. A five-run attempt
with the toolchain-case command exceeded the guarded local runner limit, so
the toolchain-case overhead threshold is recorded as skipped in that review
record. Functional output matched for every workload and no wall-time result
exceeded the accepted noise boundary.

For the HTTP/2 core workload, median wall time fell from 1.191886089 seconds
to 0.656577607 seconds. Its 0.5508727831 ratio does not pass the one-third
threshold. The new dominant measured stages were `surface_parse_lower` at
0.275918346 seconds and `semantic_environment_check` at 0.264203038 seconds.
For the HTTP/2 connection workload, median wall time fell from 1.448842086
seconds to 0.950928367 seconds. Its 0.6563367921 ratio does not pass the
one-third threshold. The generated-growth ratios were 1.5 and 2.0, so both
remained within the accepted growth threshold.

The separate standard-input comparison used prebuilt debug binaries, one
warm-up run, and five measured runs. Functional output matched for every
workload and wall-time noise remained within the accepted boundary. The
proposal-level one-third HTTP/2 wall-time thresholds still did not pass, so
the benchmark command exited with the expected failing status for the broader
proposal threshold.

For the HTTP/2 core workload, the sum of median `surface_parse_lower` and
`semantic_environment_check` time fell from 0.537219144 seconds to
0.283280795 seconds, a 47.2690431523 percent reduction. Median wall time fell
from 0.651842645 seconds to 0.643178737 seconds. For the HTTP/2 connection
workload, the same stage sum fell from 0.60004038 seconds to 0.309804194
seconds, a 48.3694424032 percent reduction. Median wall time fell from
0.95136904 seconds to 0.929512232 seconds.

The embedded lowered standard-library module comparison used the `dfdf2eb7`
pre-slice release binary from `main` and the final `77f0a36e` release binary,
one warm-up run, and five measured runs. Functional output matched for every
workload and wall-time noise remained within the accepted boundary. The
proposal-level one-third HTTP/2 wall-time thresholds still did not pass, and
the toolchain-case command was unavailable, so the benchmark command exited
with the expected failing status for the broader proposal threshold.

For the HTTP/2 core workload, median `surface_parse_lower` time fell from
0.045545017 seconds to 0.009942617 seconds. Median wall time fell from
0.16136436 seconds to 0.126753994 seconds, a ratio of 0.7855141867. The
median stage timings for the new binary were 0.000023702 seconds for
`source_loading`, 0.009942617 seconds for `surface_parse_lower`,
0.004846215 seconds for `semantic_environment_check`, 0.014803071 seconds for
`reachable_entry_lowering`, and 0.039851081 seconds for
`backend_runtime_remainder`.

For the HTTP/2 connection workload, median `surface_parse_lower` time fell
from 0.049651884 seconds to 0.010768857 seconds. Median wall time fell from
0.243578491 seconds to 0.217778114 seconds, a ratio of 0.8940777698. The
median stage timings for the new binary were 0.000024829 seconds for
`source_loading`, 0.010768857 seconds for `surface_parse_lower`,
0.005226969 seconds for `semantic_environment_check`, 0.04620323 seconds for
`reachable_entry_lowering`, and 0.080485894 seconds for
`backend_runtime_remainder`.

The separated reachable-entry lowering input comparison used the `07416af1`
pre-slice release binary from `main` and the current working-tree release
binary, one warm-up run, and five measured runs. Functional output matched for
every workload and wall-time noise remained within the accepted boundary. The
proposal-level one-third HTTP/2 wall-time thresholds still did not pass, and
the toolchain-case command was unavailable, so the benchmark command exited
with the expected failing status for the broader proposal threshold.

For the HTTP/2 core workload, median `reachable_entry_lowering` time fell from
0.015221209 seconds to 0.007923789 seconds. Median wall time fell from
0.129203376 seconds to 0.121939364 seconds, a ratio of 0.943778466. The
median stage timings for the new binary were 0.000025512 seconds for
`source_loading`, 0.010498896 seconds for `surface_parse_lower`,
0.005258769 seconds for `semantic_environment_check`, 0.007923789 seconds for
`reachable_entry_lowering`, and 0.041528865 seconds for
`backend_runtime_remainder`.

For the HTTP/2 connection workload, median `reachable_entry_lowering` time
fell from 0.039002408 seconds to 0.030307844 seconds. Median wall time fell
from 0.20997279 seconds to 0.200039744 seconds, a ratio of 0.9526936514. The
median stage timings for the new binary were 0.00002436 seconds for
`source_loading`, 0.011313915 seconds for `surface_parse_lower`,
0.005651846 seconds for `semantic_environment_check`, 0.030307844 seconds for
`reachable_entry_lowering`, and 0.078817419 seconds for
`backend_runtime_remainder`.

The backend/runtime substage comparison used the `5a34cab0` pre-slice release
binary from `main` and the current working-tree release binary, one warm-up
run, and five measured runs. Functional output matched for every workload and
wall-time noise remained within the accepted boundary. The proposal-level
one-third HTTP/2 wall-time thresholds still did not pass, and the
toolchain-case command was unavailable, so the benchmark command exited with
the expected failing status for the broader proposal threshold.

For the HTTP/2 core workload, median wall time changed from 0.121832579
seconds to 0.122511869 seconds, a ratio of 1.0055756022. The median backend
substage timings for the new binary were 0.000596564 seconds for
`backend_classfile_generation`, 0.002282136 seconds for
`backend_class_cache_prepare`, 0.039026058 seconds for
`backend_java_subprocess`, and 0.000172353 seconds for
`backend_result_cleanup`. `backend_java_subprocess` was the dominant substage
and accounted for 31.8549201139 percent of new median wall time.

For the HTTP/2 connection workload, median wall time changed from 0.200418137
seconds to 0.200011991 seconds, a ratio of 0.9979735068. The median backend
substage timings for the new binary were 0.013816598 seconds for
`backend_classfile_generation`, 0.008882214 seconds for
`backend_class_cache_prepare`, 0.060410798 seconds for
`backend_java_subprocess`, and 0.000132104 seconds for
`backend_result_cleanup`. `backend_java_subprocess` was the dominant substage
and accounted for 30.2035881439 percent of new median wall time.

## Read When

- Checking why completed bounded-analysis slices are no longer described as
  proposal work.
- Auditing the evidence behind choosing reachable-entry lowering as the next
  implementation slice.
- Reviewing why reachable candidate indexing and demand-driven
  standard-library initialization are implemented while the HTTP/2 wall-time
  acceptance thresholds remain proposal work.
- Reviewing why separated application and selected standard-library analysis
  inputs are implemented while the HTTP/2 wall-time acceptance thresholds
  remain proposal work.
- Reviewing why embedded lowered standard-library modules are implemented
  while the HTTP/2 wall-time acceptance thresholds remain proposal work.
- Reviewing why separated reachable-entry lowering inputs are implemented
  while the HTTP/2 wall-time acceptance thresholds remain proposal work.
- Reviewing why backend/runtime substage timing selects Java subprocess
  execution as the next optimization boundary for both representative HTTP/2
  workloads.
- Preserving the boundary that application analysis caching remains outside
  the measurement slice.
