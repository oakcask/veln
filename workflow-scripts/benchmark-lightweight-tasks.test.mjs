import assert from "node:assert/strict";
import test from "node:test";

import {
  FUNCTIONAL_EXPECTATIONS,
  GOVERNED_NOISE_METRICS,
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
      functional_outcomes: structuredClone(FUNCTIONAL_EXPECTATIONS),
    },
  };
}

function passingArtifact() {
  const runtime = "before return startTaskWorker(future, task -> Thread.ofPlatform().daemon(false).start(task)); after";
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
      schema_version: 3,
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
  staleAdoption.adoption.gates.functional_outcomes_match_expectations = false;
  assert.throws(
    () => validateArtifact(staleAdoption, expectedIdentity),
    /adoption verdict does not match the raw samples/,
  );
});

test("validator rejects a self-consistent failed adoption artifact", () => {
  const { artifact, expectedIdentity } = passingArtifact();
  for (const sample of artifact.samples) {
    if (sample.variant === "virtual") {
      sample.result.workload.rendezvous_channel.latency_p95_nanos = 200;
    }
  }
  artifact.summaries = summarizeSamples(artifact.samples);
  artifact.adoption = evaluateGates(artifact.samples, artifact.summaries);
  assert.equal(artifact.adoption.status, "failed");
  assert.throws(
    () => validateArtifact(artifact, expectedIdentity),
    /rendezvous_delivery_latency_p95_at_most_125_percent/,
  );
});

function setPath(value, path, replacement) {
  const components = path.split(".");
  const key = components.pop();
  const parent = components.reduce((entry, component) => entry[component], value);
  parent[key] = replacement;
}

test("applies the offered-load latency budget independently", () => {
  const cases = [
    [
      "rendezvous_delivery_latency_p95_at_most_125_percent",
      "workload.rendezvous_channel.latency_p95_nanos.median",
    ],
    [
      "bounded_delivery_latency_p95_at_most_125_percent",
      "workload.bounded_channel.latency_p95_nanos.median",
    ],
    [
      "loopback_delivery_latency_p95_at_most_125_percent",
      "workload.loopback.latency_p95_nanos.median",
    ],
  ];
  for (const [gate, path] of cases) {
    const { artifact } = passingArtifact();
    setPath(artifact.summaries.virtual, path, 126);
    const verdict = evaluateGates(artifact.samples, artifact.summaries);
    assert.equal(verdict.status, "failed", gate);
    assert.equal(verdict.gates[gate], false, gate);
    for (const [otherGate] of cases) {
      assert.equal(verdict.gates[otherGate], otherGate === gate ? false : true, otherGate);
    }
  }
});

test("enumerates every governed metric in the complete noise gate", () => {
  assert.deepEqual(GOVERNED_NOISE_METRICS, [
    "duration_seconds",
    "peak_rss_kb",
    "peak_os_threads",
    "parked_256_rss_kb",
    "parked_256_os_threads",
    "parked_256_create_nanos",
    "parked_256_complete_nanos",
    "parked_1024_rss_kb",
    "parked_1024_os_threads",
    "parked_1024_create_nanos",
    "parked_1024_complete_nanos",
    "parked_4096_rss_kb",
    "parked_4096_os_threads",
    "parked_4096_create_nanos",
    "parked_4096_complete_nanos",
    "parked_16384_rss_kb",
    "parked_16384_os_threads",
    "parked_16384_create_nanos",
    "parked_16384_complete_nanos",
    "workload.rendezvous_channel.throughput_per_second",
    "workload.rendezvous_channel.latency_p95_nanos",
    "workload.bounded_channel.throughput_per_second",
    "workload.bounded_channel.latency_p95_nanos",
    "workload.loopback.throughput_per_second",
    "workload.loopback.latency_p95_nanos",
    "select.latency_p95_nanos",
    "select.idle_cpu_nanos",
    "select.wakeups",
    "cpu_saturation.throughput_per_second",
    "cpu_saturation.latency_p95_nanos",
    "cpu_saturation.cancellation_latency_nanos",
    "descriptor_growth",
    "selector_operations",
    "reclamation.live_heap_batch_1_bytes",
    "reclamation.live_heap_batch_2_bytes",
    "reclamation.live_heap_batch_3_bytes",
    "reclamation.live_heap_batch_4_bytes",
    "reclamation.live_heap_batch_5_bytes",
    "reclamation.live_heap_batch_6_bytes",
    "reclamation.live_heap_batch_7_bytes",
    "reclamation.live_heap_batch_8_bytes",
    "reclamation.live_heap_batch_9_bytes",
    "reclamation.live_heap_batch_10_bytes",
    "reclamation.completed_registrations",
    "reclamation.retained_owners",
  ]);
  const summarizedMetrics = [];
  const visitSummaries = (value, path = "") => {
    if (value && typeof value === "object" && !Array.isArray(value)) {
      if (["median", "minimum", "maximum", "range", "stable"].every((key) => key in value)) {
        summarizedMetrics.push(path);
        return;
      }
      for (const [key, child] of Object.entries(value)) {
        visitSummaries(child, path ? `${path}.${key}` : key);
      }
    }
  };
  visitSummaries(passingArtifact().artifact.summaries.platform);
  assert.deepEqual(summarizedMetrics.sort(), [...GOVERNED_NOISE_METRICS].sort());
  for (const variant of ["platform", "virtual"]) {
    for (const path of GOVERNED_NOISE_METRICS) {
      const { artifact } = passingArtifact();
      const metric = path.split(".").reduce(
        (value, component) => value[component], artifact.summaries[variant],
      );
      metric.stable = false;
      const verdict = evaluateGates(artifact.samples, artifact.summaries);
      assert.equal(verdict.status, "failed", `${variant}.${path}`);
      assert.equal(verdict.gates.sample_ranges_conclusive, false, `${variant}.${path}`);
    }
  }
});

test("rejects named functional outcomes whose lossy aggregate could match", () => {
  const exchanged = passingArtifact().artifact;
  for (const sample of exchanged.samples) {
    const outcomes = sample.result.functional_outcomes;
    [outcomes.select.checksum, outcomes.loopback.checksum] =
      [outcomes.loopback.checksum, outcomes.select.checksum];
  }
  let verdict = evaluateGates(exchanged.samples, summarizeSamples(exchanged.samples));
  assert.equal(verdict.status, "failed");
  assert.equal(verdict.gates.functional_outcomes_match_expectations, false);
  assert.equal(verdict.gates.functional_outcomes_equal_between_variants, true);

  const changed = passingArtifact().artifact;
  for (const sample of changed.samples) {
    sample.result.functional_outcomes.rendezvous_channel.checksum ^= 1;
    sample.result.functional_outcomes.bounded_channel.checksum ^= 1;
  }
  verdict = evaluateGates(changed.samples, summarizeSamples(changed.samples));
  assert.equal(verdict.status, "failed");
  assert.equal(verdict.gates.functional_outcomes_match_expectations, false);
  assert.equal(verdict.gates.functional_outcomes_equal_between_variants, true);
});

test("compares corresponding platform and virtual functional outcomes", () => {
  const { artifact } = passingArtifact();
  const candidate = artifact.samples.find((sample) =>
    sample.variant === "virtual" && sample.run === 4
  );
  candidate.result.functional_outcomes.loopback.checksum += 1;
  const verdict = evaluateGates(artifact.samples, summarizeSamples(artifact.samples));
  assert.equal(verdict.status, "failed");
  assert.equal(verdict.gates.functional_outcomes_match_expectations, false);
  assert.equal(verdict.gates.functional_outcomes_equal_between_variants, false);
});

test("derives a one-constructor virtual candidate", () => {
  const runtime = "before return startTaskWorker(future, task -> Thread.ofPlatform().daemon(false).start(task)); after";
  const variants = runtimeSourceVariants(runtime);
  assert.equal(variants.platform, runtime);
  assert.equal(
    variants.virtual,
    "before return startTaskWorker(future, Thread::startVirtualThread); after",
  );
  assert.throws(
    () => runtimeSourceVariants(`${runtime} ${runtime}`),
    /expected one/,
  );
});
