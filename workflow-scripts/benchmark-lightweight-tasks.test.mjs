import assert from "node:assert/strict";
import test from "node:test";

import {
  JVM_ARGS,
  WORKLOAD,
  benchmarkIdentityForRuntime,
  evaluateGates,
  median,
  range,
  runtimeSourceVariants,
  stable,
  summarizeSamples,
  validateArtifact,
} from "./benchmark-lightweight-tasks.mjs";

test("summarizes deterministic benchmark samples", () => {
  assert.equal(median([9, 1, 5, 3, 7]), 5);
  assert.equal(range([9, 1, 5]), 8);
  assert.equal(stable([100, 105, 110]), true);
  assert.equal(stable([100, 105, 130]), false);
  assert.equal(stable([0, 0, 0]), true);
  assert.equal(stable([0, 0, 1]), false);
});

function timed(variant) {
  return {
    throughput_per_second: variant === "platform" ? 1000 : 900,
    latency_p95_nanos: variant === "platform" ? 100 : 110,
  };
}

function measuredSample(variant, run) {
  const platform = variant === "platform";
  const rss = platform ? 200_000 : 100_000;
  const threads = platform ? 1_100 : 12;
  return {
    variant,
    run,
    status: "passed",
    exit_code: 0,
    signal: null,
    duration_seconds: 1,
    peak_rss_kb: rss,
    peak_os_threads: threads,
    population_observations: WORKLOAD.parked_populations.map((population) => ({
      population,
      peak_rss_kb: population === 1024 ? rss : rss - 1_000,
      peak_os_threads: platform ? threads : 10 + (population === 16384 ? 2 : 0),
    })),
    stderr: "",
    result: {
      workload: {
        parked: WORKLOAD.parked_populations.map((population) => ({
          population,
          completed: true,
          create_nanos: 1,
          complete_nanos: 1,
          failure: null,
        })),
        rendezvous_channel: timed(variant),
        bounded_channel: timed(variant),
        loopback: {
          ...timed(variant),
          descriptor_growth: 0,
          selector_operations: 4,
        },
        select: {
          latency_p95_nanos: platform ? 100 : 110,
          idle_cpu_nanos: 10,
          wakeups: 10,
        },
        cpu_saturation: {
          ...timed(variant),
          cancellation_latency_nanos: 200,
          cancellation_observed: true,
        },
        reclamation: {
          live_heap_bytes: Array(10).fill(1000),
          completed_registrations: 0,
          retained_owners: 0,
        },
      },
      functional_result: 42,
    },
  };
}

function passingArtifact() {
  const runtime = "before return startTaskWorker(future, Thread::startVirtualThread); after";
  const expectedIdentity = benchmarkIdentityForRuntime(runtime, "harness-digest");
  const samples = [];
  for (let run = 1; run <= 7; run += 1) {
    const order = run % 2 === 1 ? ["platform", "virtual"] : ["virtual", "platform"];
    for (const variant of order) samples.push(measuredSample(variant, run));
  }
  const summaries = summarizeSamples(samples);
  return {
    expectedIdentity,
    artifact: {
      schema_version: 2,
      benchmark: "veln-jvm-lightweight-task-adoption",
      identity: {
        ...expectedIdentity,
        baseline_difference:
          "return startTaskWorker(future, task -> Thread.ofPlatform().daemon(false).start(task));",
        candidate_difference: "return startTaskWorker(future, Thread::startVirtualThread);",
        jvm_args: JVM_ARGS,
        workload: WORKLOAD,
      },
      samples,
      summaries,
      diagnostics: {
        jfr_virtual_thread_pinning: { status: "passed", events: 0 },
      },
      adoption: evaluateGates(samples, summaries),
    },
  };
}

test("validates recomputed samples and current runtime identities", () => {
  const { artifact, expectedIdentity } = passingArtifact();
  assert.doesNotThrow(() => validateArtifact(artifact, expectedIdentity));

  const staleRuntime = structuredClone(artifact);
  staleRuntime.identity.virtual_runtime_source_sha256 = "stale";
  assert.throws(
    () => validateArtifact(staleRuntime, expectedIdentity),
    /runtime digest does not match/,
  );

  const staleSummary = structuredClone(artifact);
  staleSummary.samples[0].result.workload.loopback.throughput_per_second = 1;
  assert.throws(
    () => validateArtifact(staleSummary, expectedIdentity),
    /summaries do not match the raw samples/,
  );

  const staleAdoption = structuredClone(artifact);
  staleAdoption.adoption.gates.functional_results_equal = false;
  assert.throws(
    () => validateArtifact(staleAdoption, expectedIdentity),
    /adoption verdict does not match the raw samples/,
  );
});

test("derives a one-constructor platform baseline", () => {
  const runtime = "before return startTaskWorker(future, Thread::startVirtualThread); after";
  const variants = runtimeSourceVariants(runtime);
  assert.equal(variants.virtual, runtime);
  assert.equal(
    variants.platform,
    "before return startTaskWorker(future, task -> Thread.ofPlatform().daemon(false).start(task)); after",
  );
  assert.throws(
    () => runtimeSourceVariants(`${runtime} ${runtime}`),
    /expected one/,
  );
});
