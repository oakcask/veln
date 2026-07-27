# Parallel Test Execution

Status: proposed

`veln test` currently analyzes the selected project once, then lowers, builds,
and runs each discovered test case in a serial loop. Independent cases cannot
use more than one processor through the test runner, so suites with many
runtime tests spend unnecessary wall-clock time waiting for per-case lowering,
JVM generation, cache preparation, and Java execution.

This proposal makes case execution parallel by default while preserving the
existing discovery, static-gate, result, and output contracts.

## Goals

- Keep at most a bounded number of test cases in flight.
- Use the processors available to the process by default.
- Let callers request an exact concurrency limit, including serial execution.
- Preserve stable case order in human and JSON output regardless of completion
  order.
- Preserve the suite-wide static gate: no case starts when project diagnostics
  contain an error.
- Keep each case's generated files, runtime traces, stdout, and stderr isolated.
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

The command accepts a job limit before its existing targets:

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

Each runnable case receives a stable ordinal from the existing discovered case
order. A bounded scheduler claims ordinals and executes no more than the active
job limit concurrently. Every claimed job owns its complete case pipeline:

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

The implementation should keep scheduling separate from Veln-specific case
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

## Verification

### Command and Scheduler Tests

- Parse long and short job flags with JSON mode and multiple targets.
- Reject zero, missing, malformed, repeated, and overflowing values.
- Verify automatic job resolution, its one-job fallback, and clamping to the
  runnable case count through injected availability rather than host assumptions.
- Use a controllable fake case executor with synchronization barriers and
  atomics to prove that more than one job can overlap, the configured bound is
  never exceeded, and `--jobs 1` never overlaps jobs.
- Complete fake jobs in reverse and mixed orders and assert byte-identical
  ordered human and JSON reports.
- Verify that a static gate and an empty runnable set start no workers.
- Verify that failures do not cancel remaining cases and that workers are
  joined before an orchestration error is returned.

### Filesystem and Cache Tests

- Run simultaneous cases and prove their trace paths and cleanup are disjoint.
- Exercise concurrent JVM cache hits, different-key publication, and same-key
  publication without incomplete reuse or deletion of a valid winner.
- Confirm that captured stdout, stderr, contract traces, and result traces stay
  attached to the producing case under overlap.

### End-to-End and Performance Evidence

- Add an executable CLI case containing multiple independent tests and compare
  `--jobs 1`, `--jobs 2`, and automatic mode for identical ordered reports and
  exit status.
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

Implementation is complete only when:

1. the command help and short command specification document `--jobs` and the
   automatic default;
2. the full command and test JSON specifications describe deterministic
   parallel execution and the serial compatibility route;
3. executable specification coverage demonstrates the observable command
   behavior;
4. the concurrency, isolation, cache-race, ordering, and performance gates
   above pass; and
5. this proposal is removed from the active catalog and retained as an
   implemented proposal record.
