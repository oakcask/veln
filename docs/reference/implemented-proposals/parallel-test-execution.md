---
role: implementation-record
authority: supporting
update-when: Parallel test command scheduling, jobs flags, output ordering, or test execution examples change.
---

# Parallel Test Execution

This page records completion of bounded parallel `veln test` execution. Use
the specification pages and executable examples for current command behavior.

## Read First

- Current command behavior:
  [../../specification/commands.md](../../specification/commands.md), then
  [../../specification/command-test.md](../../specification/command-test.md)
  when exact command rules matter.
- Test JSON behavior:
  [../../specification/test-json.md](../../specification/test-json.md).
- Primary observable examples:
  `../../../examples/specification/test/parallel-jobs-one-json/`,
  `../../../examples/specification/test/parallel-jobs-two-json/`, and
  `../../../examples/specification/test/parallel-jobs-auto-json/`.

## Outcome

`veln test` accepts `-j <JOBS>` and `--jobs <JOBS>` before `--`, including
after an earlier target. The option selects the maximum number of runnable test
cases that may execute concurrently. Omitting it uses available processor
parallelism with a fallback of one, `--jobs 1` is the serial compatibility
route, and the active worker count is clamped to the number of runnable cases.

Runnable cases use the bounded ordered executor. The coordinator builds and
renders the report after all workers finish, so human and JSON case records,
diagnostics, captured events, summary counts, failures, and exit status remain
in discovered-case order. Captured stdout and stderr stay attached to the case
that produced them instead of streaming from workers.

Static diagnostics block execution before worker creation. A failed or errored
case does not cancel remaining cases, and orchestration errors wait for worker
joining before the command reports the failure.

## Completion Evidence

- Parser and help coverage checks both flag spellings, JSON mode, multiple
  targets, option placement after a target, the `--` target boundary, and
  rejected zero, missing, malformed, repeated, mixed-spelling repeated, and
  overflowing values.
- Worker-count unit coverage checks explicit, automatic, fallback, clamped, and
  zero-case outcomes with injected processor availability.
- Ordered executor coverage uses barriers and atomics for selected-bound
  overlap, serial behavior, ordered completion, run-all-cases behavior, and
  joining after an orchestration error.
- Production-path orchestration coverage injects fake preparation and
  execution closures to observe the selected bound, serial route, static-gate
  no-work path, ordered records, and complete joining after failure.
- Production JVM-path coverage runs `prepare_test_case_job`,
  `execute_test_case_job`, and `execute_test_program` with real analyzed
  projects. It forces overlapping runnable cases at the JVM execution boundary,
  checks per-case build directory and trace-file isolation, verifies captured
  stdout and stderr ownership, and combines pass, result failure, per-case
  lowering blockage, doctest, and runner-error outcomes in discovered-case
  order.
- JVM class-cache race coverage checks concurrent warm same-key hits, cold
  different-key publication, and cold same-key publication.
- Executable CLI examples under `../../../examples/specification/test/` check
  ordered JSON records and captured output for `--jobs 1`, `--jobs 2`, and
  automatic job modes.

## Representative Measurement

Representative timing remains procedural evidence rather than a correctness
assertion. On a host reporting 20 available processors, a generated 12-case
suite using tail-recursive countdown work resolved automatic mode to 12 jobs.
The measured commands were `veln test --json --jobs 1` and `veln test --json`
under `/usr/bin/time -v`.

Serial `--jobs 1` measured 1.25s cold and 1.50s warm, with maximum resident set
size around 53 MiB. Automatic mode measured 0.40s cold and 0.44s warm, also
around 53 MiB maximum resident set size. Adjacent automatic suites measured
0.38s cold and 0.41s warm for 10 cases, and 0.42s cold and 0.44s warm for 14
cases. The similar cold and warm results showed JVM cache preparation was not
material for this suite. Pipeline-stage observation was limited to end-to-end
command timing; focused production-path tests separately observed overlap and
bounded worker counts without relying on timing thresholds.

## Read When

- Checking why parallel test execution is no longer listed as active proposal
  work.
- Auditing the evidence that made bounded parallel execution the default.
- Preserving the historical non-goals: no parallel discovery, parsing,
  semantic analysis, statements within one case, fail-fast, retries, sharding,
  persistent workers, environment-variable override, or third-party async
  runtime.
