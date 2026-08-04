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

The CLI integration harness no longer performs an independent source-error
guard analysis for normal `check`, `run`, and `test` cases. The implemented
policy is specified in
[../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
The completed harness slice includes boundary tests for copied-project and
repeated-invocation artifact isolation. The remaining proposal scope starts at
analysis-pipeline growth. It does not include more source-error guard payment
changes for those normal harness cases.

The analysis pipeline also constructs a type environment for the merged
application and standard-library surface. Type and effect inference performs
repeated whole-module scans until results stabilize. The HTTP/2 standard
library makes this behavior visible because its production sources contain a
large module graph and many functions.

The private-signature inference slice is implemented for private return,
call-site, and prelude-callback fixed-point passes. The private return and
call-site passes first identify omitted private parameter and return slots
that can still affect the pass. The prelude-callback pass first identifies
omitted private returns that still contain unknown type information or whose
tail expression can still use a prelude callback expected type. If no such
slot exists, these passes do not traverse function bodies. If such slots
exist, a private-reference index first excludes functions whose body cannot
mention any eligible private slot name in the same module. The index then
selects functions that own eligible private slots or reference them in ways
that can contribute constraints before the repeated body traversals begin.
Call-site and prelude-callback contributor discovery is not rebuilt inside
each stabilization round.
Structural `veln-sema` tests use test-only work counters to assert that
unrelated fully annotated module sets do not pay private-inference body scans,
and that an omitted private helper chain still reaches the same inferred
parameter and return types while skipping unrelated annotated modules and
unrelated annotated functions in the same module.
The same counter coverage records deterministic body-return,
private-reference candidate-filter, private-reference index, contributor
discovery, call-site, and prelude-callback scan counts. One case combines
nested and scalar callback helpers while preserving the inferred callback
parameter and return types and proving that already fixed prelude-callback
returns do not enter prelude body traversal. Another case verifies that a
helper whose return is already fixed before the prelude-callback pass performs
no prelude-callback discovery or body scan.

## Proposed Outcome

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
| Project isolation | Analysis reused for one copied project is not reused after source text, manifest data, command inputs, or dependency identity changes | Cache invalidation and concurrent-project unit tests |
| Determinism | Repeated and concurrent analysis returns diagnostics in the same stable order and does not share mutable project state | Repeated and concurrent analyzer tests |
| Bounded growth | Doubling an unrelated fully annotated module set does not produce superlinear repeated whole-module inference work | Generated high-cardinality analysis benchmark |
| Representative improvement | HTTP/2 core and connection workloads become materially faster without weakening their assertions | Controlled before-and-after benchmark described below |

The existing toolchain suite remains authoritative for command behavior. The
new cache and benchmark tests are authoritative only for analysis reuse and
performance properties.

The private-signature inference structural tests are the primary evidence for
the implemented bounded-growth slice that avoids private-inference scans for
fully annotated unrelated modules. The generated high-cardinality benchmark
and representative HTTP/2 comparisons remain required before this proposal is
complete. The `veln-analysis` shared-analysis determinism test is the primary
evidence for the implemented repeated and concurrent diagnostic-order slice.

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

The implemented benchmark slice provides
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

## Implementation Guidance

This section is not normative except where the contracts above require
analysis-state isolation.

The implementation should first measure parsing, type-environment
construction, semantic checks, reachable lowering, JVM generation, and
process execution separately. It should optimize the measured dominant stage.

Likely implementation options include:

- precompute immutable standard-library declarations and signatures;
- reuse standard-library analysis within a process;
- infer only declarations with omitted information; the private-signature
  inference slice implements this for private return, call-site, and
  prelude-callback type inference;
- replace repeated global inference scans with dependency-directed work;
- rebuild only analysis entries affected by changed input identity.

The implementation may combine these options. It must not weaken diagnostic
coverage to meet the performance target.

## Implemented Slice

The private-signature inference slice is implemented. It keeps Veln syntax,
type rules, diagnostics, command output, JSON output, and runtime behavior
unchanged. It narrows only analyzer work:

- private return inference skips the fixed-point pass when no omitted private
  return slot still contains unknown type information;
- private call-site signature inference skips the fixed-point pass when no
  omitted private parameter or return slot can still change;
- private prelude-callback return inference skips contributor discovery and
  body traversal when no omitted private return still contains unknown type
  information or has a tail expression that can use a prelude callback
  expected type;
- when eligible private slots remain, call-site and prelude-callback inference
  use a private-reference index to select only functions that own those slots
  or reference them in ways that can contribute constraints, and their
  contributor sets are not rebuilt in every stabilization round;
- private-reference index construction is limited to modules that contain
  eligible omitted private slots or eligible omitted private returns, and it
  skips functions that cannot mention the eligible private slot names in their
  module;
- private-reference candidate filtering, private-reference indexing, and
  call-site contributor discovery each have deterministic structural counter
  coverage, so the tests distinguish candidate-filter traversal from repeated
  inference body traversal;
- structural tests record private-reference index scan counts separately from
  repeated inference body traversals and cover unrelated annotated functions in
  the same module;
- function-body effect inference and private-handler retained-effect inference
  now share one dependency graph over functions and private handlers. Initial
  dependency discovery, function body collection, private-handler provider
  evaluation, and dependency-triggered reevaluation have separate test-only
  structural counters;
- effect inference reevaluates only declarations whose function or private
  handler dependencies changed. Structural `veln-sema` tests cover private
  helper chains, private handler retained effects flowing through lexical
  handlers, cyclic dependencies with stable effect ordering, and unrelated
  fully annotated module growth at `W(0)`, `W(N)`, and `W(2N)`;
- shared project analysis has `veln-analysis` regression coverage for repeated
  and concurrent checked diagnostic JSON generation across two distinct
  projects that use overlapping source paths and declaration names. The test
  compares each result with that project's isolated ordered JSON sequence and
  checks that project-specific module and type facts do not appear in the
  other project's result;
- reusable standard-library semantic signatures are implemented for
  application project analysis. `veln-analysis` prepares one immutable
  standard environment from the embedded standard-library bundle and passes it
  to `veln-sema` for project checks and reachable-entry lowering. The reusable
  environment is keyed by the embedded standard-library bundle and semantic
  model. Preparing reusable facts from a module set that does not match the
  embedded bundle produces a non-current identity, so application analysis
  falls back to the uncached path instead of trusting unrelated facts.
  `veln-sema` keeps the uncached path available and compares cached and
  uncached diagnostic JSON, lowered core output, and lowered IR output for a
  successful project, a project with type/effect diagnostics, applications
  with overlapping source paths and declaration names, a local `std::`-prefixed
  application module that is outside the reusable standard bundle, and a local
  application source whose module identity collides with an embedded standard
  module name. The same `veln-sema` coverage also checks a standard module
  import used by a standard public alias. The cached path removes only the
  prepared standard declarations from the fresh application environment, so
  same-path and same-module application declarations still construct
  project-local facts. Test-only work counters prove repeated and concurrent
  cached analyses prepare the reusable standard signatures once while each
  non-empty application analysis constructs fresh semantic facts. `veln-analysis`
  regression tests exercise the embedded-standard `OnceLock` path with
  repeated and concurrent distinct projects, local application sources whose
  module name begins with `std::`, and a local `std::prelude` collision;
- reusable standard-library semantic signatures are restricted to the embedded
  standard modules that the merged application project actually loaded. The
  cached path keeps the reusable environment immutable and combines each
  application environment with only that project's standard dependency closure.
  `veln-sema` coverage prepares a reusable standard environment with an unused
  standard module and checks that an application that loaded only
  `std::prelude` receives only `std::prelude` function facts while still
  matching uncached diagnostics and lowering;
- reusable standard-library environments are prepared eagerly and immutably
  from the embedded standard module set. Preparation constructs the standard
  semantic signature environment once, records the embedded bundle identity and
  declaration fingerprints, and stores the result behind shared ownership
  without a mutable cross-project environment map. Each cached application
  analysis derives its selected standard base from the prepared signatures
  instead of rebuilding standard signatures during application analysis.
  `veln-sema` coverage checks applications that load only `std::prelude` and
  applications that load `std::prelude` plus another standard module while the
  prepared standard-environment work counter remains at one across repeated
  and concurrent analyses;
- cached standard-library facts are cloned only when combining selected
  prepared facts with freshly constructed application facts. Cache hits do not
  share mutable application state, diagnostics, lowering state, reachability
  state, or IR between projects.

The controlled benchmark run for the immutable standard-environment
preparation slice kept functional outputs equal and wall-time noise within the
accepted boundary. Median wall time changed from 0.86 to 1.09 seconds for the
small schema workload, from 1.44 to 1.26 seconds for HPACK static, from 1.51
to 1.33 seconds for HTTP/2 core, from 6.44 to 6.38 seconds for HTTP/2
connection, and from 0.77/0.78/0.86 to 0.99/1.00/1.09 seconds for generated
32/64/128 module workloads. The generated-size CPU growth checks passed. The
representative HTTP/2 improvement thresholds did not pass.

The controlled
[stage-timing benchmark](../reviews/toolchain-analysis-stage-benchmark.json)
for the measurement slice used prebuilt debug binaries, one warm-up run, and
five measured runs. The baseline binary had no stage instrumentation, so its
stage data is recorded as unavailable while its wall-time and functional
comparisons remain active. The new binary kept functional outputs equal for
all tracked workloads and kept wall-time noise within the accepted boundary.
For the measured HTTP/2 workloads, `reachable_entry_lowering` was the dominant
stage. The HTTP/2 core workload recorded a 0.158566097 second median for that
stage, ahead of semantic environment construction and checking at 0.10378384
seconds. The HTTP/2 connection workload recorded a 5.135779555 second median
for that stage, ahead of the backend and runtime remainder at 0.222157528
seconds. The evidence-driven next implementation slice is therefore bounded
reachable-entry selection and lowering work for representative HTTP/2
applications. It must preserve the existing functional-output comparisons and
must not add application-analysis caching as part of this measurement slice.

The remaining proposal work is explicit:

- application analysis caching remains out of scope and incomplete;
- representative HTTP/2 connection improvement evidence remains incomplete;
- bounded reachable-entry selection and lowering for representative HTTP/2
  applications remains incomplete.

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
and the controlled benchmark meets all comparison thresholds. The benchmark
harness and private-signature inference slices are implemented, but they do
not complete the analyzer optimization scope.

After completion, move this document to
`../reference/implemented-proposals/` and remove it from the proposal catalog.
