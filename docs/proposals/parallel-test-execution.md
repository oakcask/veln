# Parallel Test Execution

Status: proposed

`veln test` exposes bounded job control and schedules runnable cases through
the ordered bounded executor. This proposal remains active only for the
remaining completion evidence and cleanup gates that are not yet promoted to
implemented proposal history.

## Goals

- Demonstrate a wall-clock improvement on a representative multi-case suite
  without making timing thresholds part of correctness tests.

## Non-Goals

- Parallel project discovery, parsing, or semantic analysis.
- Running statements within one test case in parallel.
- Adding fail-fast behavior, retries, sharding across machines, or persistent
  worker processes.
- Guaranteeing a particular processor percentage or speedup on every host.
- Serializing access to external resources chosen by test code. Tests that
  mutate shared files, ports, services, or process-wide state must provide
  their own isolation or run with one job.

## Command Interface

The implemented command accepts a job limit before its existing targets:

```text
veln test [--json] [-j <JOBS> | --jobs <JOBS>] [target ...]
```

`JOBS` is a positive decimal integer.

- When `--jobs` is omitted, the runner uses the process's available parallelism.
  If that value cannot be determined, it falls back to one job.
- `--jobs 1` uses the same serial scheduling order as the current command and
  is the compatibility route for suites with shared external state.
- The active worker count is the smaller of `JOBS` and the number of runnable
  cases. Zero runnable cases starts no workers.
- Zero, a missing value, a non-integer value, repeated job flags, and values
  that cannot be represented by the runner are command-line errors reported
  before project discovery.
- `-j` and `--jobs` have identical behavior. A job flag is an option, not a
  test target, including when it follows another target before `--`.

This proposal does not add an environment-variable override. One explicit
command-line control avoids precedence rules and keeps invocations reproducible.

## Observable Semantics

### Selection and Static Gate

Discovery, explicit target expansion, dependency-aware selection, doctest
extraction, and semantic analysis complete before scheduling cases. Their
behavior remains as specified by
[Commands](../specification/commands-full.md#veln-test---json-target) and
[Test JSON](../specification/test-json.md).

If static diagnostics contain an error, every discovered case remains
`blocked` with reason `static_gate`; the runner does not create a worker pool or
start Java. A discovery error likewise produces the existing suite result
without scheduling cases.

### Scheduling

The implemented runner gives each runnable case a stable ordinal from the
existing discovered case order. A bounded scheduler claims ordinals and
executes no more than the active job limit concurrently. Every claimed job owns
its complete case pipeline:

1. lower the entry reachable from that test;
2. generate its JVM program;
3. prepare or reuse its JVM class-cache entry;
4. run its Java process and capture its traces and output; and
5. apply runtime and expected-output results to that case.

A failed case does not cancel queued or running cases. This preserves the
current run-all-cases behavior. The scheduler waits for every claimed case and
joins every worker before constructing the report.

### Deterministic Reporting

Workers return completed case records tagged with their stable ordinals. The
coordinator stores each record in its original slot and constructs
`TestReport` only after all workers finish.

Human status lines, failure details, diagnostics, captured events, JSON
`cases`, summary counts, top-level status, and process exit status are derived
from that ordered report. Completion order must not change output bytes. Output
from a case is never streamed directly while workers are active.

Selection notes and suite diagnostics remain ahead of case output in human
mode. Per-case stdio event sequence numbers remain local to their case; this
proposal does not introduce a suite-wide event sequence.

### Failure Boundaries

Existing case classifications remain unchanged:

- an unavailable Java runner is a case `error` with reason `runner_error`;
- runtime, contract, result, runtime-expectation, and expected-output failures
  retain their current case records;
- lowering diagnostics block only the affected case after the suite-wide
  static gate has passed; and
- ordinary test failures do not prevent other cases from running.

The executor must not silently lose a case if a worker cannot return normally.
An orchestration failure is surfaced through the existing command-error path
after all other workers are joined. The implementation must test this path
without deliberately panicking a production worker.

## Isolation and Shared State

Each case continues to run in a separate Java process. Its build directory and
stdio, contract-error, and result-error trace files must be unique for the full
case lifetime. Cleanup happens after that case's traces have been read and must
not remove another case's files.

The JVM class cache is intentionally shared. Before parallel execution becomes
the default, cache preparation and publication must be verified for simultaneous
cache hits, different cache keys, and competing publication of the same key.
The winning entry must be completely validated before reuse, and a losing
worker must not delete or partially observe a valid entry.

Test code still shares its working directory and any external resources it
names. Parallel execution therefore formalizes that separate cases must not
depend on execution order. The serial `--jobs 1` escape hatch is required
before the default changes.

## Implementation Shape

The implementation keeps scheduling separate from Veln-specific case
execution:

- a bounded executor accepts ordered jobs and returns ordered results;
- the case executor owns lowering through result comparison for one case;
- worker count resolution is a small, directly tested command-layer function;
  and
- report construction and rendering remain single-threaded.

Use standard-library concurrency unless implementation work demonstrates a
missing capability. Do not add a third-party runtime only to manage this
bounded pool.

The shared analysis representation and lowering path must be audited for safe
concurrent read access. If they cannot be shared safely, the implementation
must first make immutable analysis inputs safely shareable or move the smallest
unavoidable unsafe phase outside the workers. It must not duplicate whole-project
analysis once per case, because that would trade elapsed time for unbounded
memory and repeated work.

Current implementation note: `ProjectAnalysis` is not `Sync`, so per-case
lowering preparation is guarded while JVM execution and capture run outside
that guard. Future work may make immutable analysis inputs shareable to overlap
more of the per-case pipeline.

## Verification

### Remaining Filesystem and Cache Evidence

- Run simultaneous cases and prove their trace paths and cleanup are disjoint.
- Exercise concurrent JVM cache hits, different-key publication, and same-key
  publication without incomplete reuse or deletion of a valid winner.
- Confirm that captured stdout, stderr, contract traces, and result traces stay
  attached to the producing case under overlap.

### Remaining End-to-End and Performance Evidence

- Cover mixed pass, failure, blocked-lowering, doctest expectation, and runner
  error results in one ordered report where practical.
- Measure a representative CPU-bound multi-case suite in serial and automatic
  modes before and after implementation. Record case count, resolved job count,
  elapsed time, peak memory, and time spent in analysis, per-case lowering, JVM
  generation and cache preparation, and Java execution when those stages can
  be separated.
- Repeat the measurement with adjacent smaller and larger case counts so the
  evidence distinguishes bounded parallel scaling from a one-off cache or
  startup effect. Include both cold-cache and warm-cache observations when
  cache preparation materially changes the result.
- Completion requires observed overlap and lower elapsed time on a host that
  exposes more than one processor; correctness tests must not fail on a noisy
  timing ratio. Keep the representative measurement command with the completion
  evidence when a stable CI threshold is not available.
- Check that peak memory remains bounded by the configured jobs rather than the
  total number of cases.

## Documentation and Completion

Implementation history may move this proposal out of the active catalog after
the remaining mixed-result, isolation, cache-race, and performance evidence is
recorded.
