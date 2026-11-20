#!/usr/bin/env node
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const REPO_ROOT = resolve(dirname(SCRIPT_PATH), "..");
const HARNESS = join(
  REPO_ROOT,
  "tools/veln-repo-lightweight-task-benchmark/LightweightTaskBenchmark.java",
);
const DEFAULT_OUTPUT = join(
  REPO_ROOT,
  "benchmarks/lightweight-tasks/result.json",
);
const VIRTUAL_WORKER = "return startTaskWorker(future, Thread::startVirtualThread);";
const PLATFORM_WORKER =
  "return startTaskWorker(future, task -> Thread.ofPlatform().daemon(false).start(task));";
export const JVM_ARGS = [
  "-Xms512m",
  "-Xmx512m",
  "-Xss256k",
  "-XX:+UseSerialGC",
  "-Djdk.virtualThreadScheduler.parallelism=2",
  "-Djdk.virtualThreadScheduler.maxPoolSize=2",
];
export const WORKLOAD = {
  parked_populations: [256, 1024, 4096, 16384],
  channel_operations: 16384,
  channel_offered_operations_per_second: 2000,
  channel_capacities: [0, 64],
  select_deliveries: 128,
  select_timeout_millis: 1,
  select_offered_interval_millis: 2,
  loopback_connections: 1024,
  loopback_offered_connections_per_second: 250,
  loopback_payload: "veln-lightweight-task-loopback-payload",
  cpu_saturators: 3,
  cpu_saturation_millis: 200,
  cpu_short_jobs: 128,
  reclamation_batches: 10,
  reclamation_population: 1024,
  per_process_preparation: {
    rendezvous_channel_operations: 2048,
    bounded_channel_operations: 2048,
    loopback_connections: 32,
  },
};
const REQUIRED_SAMPLES = 7;
const TIMEOUT_MILLIS = 180_000;
const PROCESS_TREE_RSS_LIMIT_KB = 4 * 1024 * 1024;
export const FUNCTIONAL_EXPECTATIONS = {
  parked: { completed_populations: [256, 1024, 4096, 16384] },
  rendezvous_channel: { checksum: 134225920 },
  bounded_channel: { checksum: 134225920 },
  select: { checksum: 8128 },
  loopback: { checksum: 563712 },
  cpu_saturation: { checksum: 8128 },
  reclamation: { completed_registrations: 0, retained_owners: 0 },
};
export const GOVERNED_NOISE_METRICS = [
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
];

export function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  if (sorted.length === 0) throw new Error("median requires values");
  return sorted[Math.floor(sorted.length / 2)];
}

export function range(values) {
  return Math.max(...values) - Math.min(...values);
}

export function stable(values) {
  const center = median(values);
  return center === 0 ? range(values) === 0 : range(values) <= center * 0.2;
}

function metricSummary(values) {
  return {
    median: median(values),
    minimum: Math.min(...values),
    maximum: Math.max(...values),
    range: range(values),
    stable: stable(values),
  };
}

function parkedAt(sample, population) {
  return sample.result.workload.parked.find((entry) => entry.population === population);
}

function parkedObservationAt(sample, population) {
  return sample.population_observations.find((entry) => entry.population === population);
}

function values(samples, selector) {
  return samples.map(selector);
}

export function summarizeVariant(samples) {
  if (samples.length !== REQUIRED_SAMPLES || samples.some((sample) => sample.status !== "passed")) {
    return { status: "failed", measured_samples: samples.length };
  }
  const workloadMetrics = {};
  for (const workload of ["rendezvous_channel", "bounded_channel", "loopback"]) {
    workloadMetrics[workload] = {
      throughput_per_second: metricSummary(values(samples,
        (sample) => sample.result.workload[workload].throughput_per_second)),
      latency_p95_nanos: metricSummary(values(samples,
        (sample) => sample.result.workload[workload].latency_p95_nanos)),
    };
  }
  const parkedMetrics = Object.fromEntries(WORKLOAD.parked_populations.flatMap((population) => [
    [`parked_${population}_rss_kb`, metricSummary(values(samples,
      (sample) => parkedObservationAt(sample, population).peak_rss_kb))],
    [`parked_${population}_os_threads`, metricSummary(values(samples,
      (sample) => parkedObservationAt(sample, population).peak_os_threads))],
    [`parked_${population}_create_nanos`, metricSummary(values(samples,
      (sample) => parkedAt(sample, population).create_nanos))],
    [`parked_${population}_complete_nanos`, metricSummary(values(samples,
      (sample) => parkedAt(sample, population).complete_nanos))],
  ]));
  const reclamationMetrics = Object.fromEntries(Array.from(
    { length: WORKLOAD.reclamation_batches },
    (_, index) => [`live_heap_batch_${index + 1}_bytes`, metricSummary(values(samples,
      (sample) => sample.result.workload.reclamation.live_heap_bytes[index]))],
  ));
  return {
    status: "passed",
    duration_seconds: metricSummary(values(samples, (sample) => sample.duration_seconds)),
    peak_rss_kb: metricSummary(values(samples, (sample) => sample.peak_rss_kb)),
    peak_os_threads: metricSummary(values(samples, (sample) => sample.peak_os_threads)),
    ...parkedMetrics,
    parked_16384_completed: samples.every((sample) => parkedAt(sample, 16384)?.completed === true),
    workload: workloadMetrics,
    select: {
      latency_p95_nanos: metricSummary(values(samples,
        (sample) => sample.result.workload.select.latency_p95_nanos)),
      idle_cpu_nanos: metricSummary(values(samples,
        (sample) => sample.result.workload.select.idle_cpu_nanos)),
      wakeups: metricSummary(values(samples, (sample) => sample.result.workload.select.wakeups)),
    },
    cpu_saturation: {
      throughput_per_second: metricSummary(values(samples,
        (sample) => sample.result.workload.cpu_saturation.throughput_per_second)),
      latency_p95_nanos: metricSummary(values(samples,
        (sample) => sample.result.workload.cpu_saturation.latency_p95_nanos)),
      cancellation_latency_nanos: metricSummary(values(samples,
        (sample) => sample.result.workload.cpu_saturation.cancellation_latency_nanos)),
      cancellation_observed: samples.every((sample) =>
        sample.result.workload.cpu_saturation.cancellation_observed === true),
    },
    descriptor_growth: metricSummary(values(samples,
      (sample) => sample.result.workload.loopback.descriptor_growth)),
    selector_operations: metricSummary(values(samples,
      (sample) => sample.result.workload.loopback.selector_operations)),
    reclamation: {
      ...reclamationMetrics,
      completed_registrations: metricSummary(values(samples,
        (sample) => sample.result.workload.reclamation.completed_registrations)),
      retained_owners: metricSummary(values(samples,
        (sample) => sample.result.workload.reclamation.retained_owners)),
      final_five_stable: samples.every((sample) => {
        const heaps = sample.result.workload.reclamation.live_heap_bytes.slice(-5);
        const center = median(heaps);
        return heaps.every((heap) => Math.abs(heap - center) <= center * 0.1);
      }),
      completed_registrations_zero: samples.every((sample) =>
        sample.result.workload.reclamation.completed_registrations === 0),
      retained_owners_zero: samples.every((sample) =>
        sample.result.workload.reclamation.retained_owners === 0),
    },
  };
}

export function summarizeSamples(samples) {
  const byVariant = Object.fromEntries(["platform", "virtual"].map((variant) => [
    variant,
    samples.filter((sample) => sample.variant === variant),
  ]));
  return Object.fromEntries(Object.entries(byVariant).map(([variant, entries]) => [
    variant,
    summarizeVariant(entries),
  ]));
}

export function evaluateGates(samples, summaries) {
  const baseline = summaries.platform;
  const candidate = summaries.virtual;
  const passedSamples = samples.filter((sample) => sample.status === "passed");
  const functionalOutcomesExpected = passedSamples.length === REQUIRED_SAMPLES * 2
    && passedSamples.every((sample) => canonicalJson(sample.result.functional_outcomes)
      === canonicalJson(FUNCTIONAL_EXPECTATIONS));
  const functionalOutcomesEqual = passedSamples.length === REQUIRED_SAMPLES * 2
    && Array.from({ length: REQUIRED_SAMPLES }, (_, index) => index + 1).every((run) => {
      const pair = passedSamples.filter((sample) => sample.run === run);
      return pair.length === 2
        && pair.some((sample) => sample.variant === "platform")
        && pair.some((sample) => sample.variant === "virtual")
        && canonicalJson(pair[0].result.functional_outcomes)
          === canonicalJson(pair[1].result.functional_outcomes);
    });
  const commonPopulation = ["platform", "virtual"].every((variant) =>
    samples.filter((sample) => sample.variant === variant).every((sample) =>
      sample.status === "passed" && parkedAt(sample, 1024)?.completed === true));
  const noiseMetrics = [baseline, candidate].flatMap((variant) =>
    variant.status === "passed"
      ? GOVERNED_NOISE_METRICS.map((path) => path.split(".").reduce(
        (value, component) => value?.[component], variant,
      ))
      : []
  );
  const conclusive = noiseMetrics.length === GOVERNED_NOISE_METRICS.length * 2
    && noiseMetrics.every((metric) => metric?.stable === true);
  const gates = {
    functional_outcomes_match_expectations: functionalOutcomesExpected,
    functional_outcomes_equal_between_variants: functionalOutcomesEqual,
    common_population_completed: commonPopulation,
    candidate_16384_completed: candidate.parked_16384_completed === true,
    candidate_os_thread_growth_at_most_8:
      candidate.status === "passed"
      && candidate.parked_16384_completed
      && candidate.parked_16384_os_threads.median - candidate.parked_256_os_threads.median <= 8,
    candidate_1024_rss_not_above_baseline:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.parked_1024_rss_kb.median <= baseline.parked_1024_rss_kb.median,
    candidate_1024_os_threads_at_most_quarter_baseline:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.parked_1024_os_threads.median <= baseline.parked_1024_os_threads.median / 4,
    channel_throughput_at_least_80_percent: ["rendezvous_channel", "bounded_channel"].every(
      (workload) => candidate.status === "passed" && baseline.status === "passed"
        && candidate.workload[workload].throughput_per_second.median
          >= baseline.workload[workload].throughput_per_second.median * 0.8,
    ),
    loopback_throughput_at_least_80_percent:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.workload.loopback.throughput_per_second.median
        >= baseline.workload.loopback.throughput_per_second.median * 0.8,
    rendezvous_delivery_latency_p95_at_most_125_percent:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.workload.rendezvous_channel.latency_p95_nanos.median
        <= baseline.workload.rendezvous_channel.latency_p95_nanos.median * 1.25,
    bounded_delivery_latency_p95_at_most_125_percent:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.workload.bounded_channel.latency_p95_nanos.median
        <= baseline.workload.bounded_channel.latency_p95_nanos.median * 1.25,
    loopback_delivery_latency_p95_at_most_125_percent:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.workload.loopback.latency_p95_nanos.median
        <= baseline.workload.loopback.latency_p95_nanos.median * 1.25,
    reclamation_final_five_within_10_percent: candidate.reclamation?.final_five_stable === true,
    completed_registrations_absent: candidate.reclamation?.completed_registrations_zero === true,
    released_owners_absent: candidate.reclamation?.retained_owners_zero === true,
    cpu_saturation_cancellation_observed:
      candidate.cpu_saturation?.cancellation_observed === true,
    sample_ranges_conclusive: conclusive,
  };
  return {
    status: Object.values(gates).every(Boolean) ? "passed" : "failed",
    gates,
  };
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function requireEquivalent(actual, expected, message) {
  if (canonicalJson(actual) !== canonicalJson(expected)) throw new Error(message);
}

export function validateArtifact(artifact, expectedIdentity) {
  if (artifact.schema_version !== 3 || artifact.benchmark !== "veln-jvm-lightweight-task-adoption") {
    throw new Error("unexpected lightweight-task benchmark schema");
  }
  if (!Array.isArray(artifact.samples) || artifact.samples.length !== REQUIRED_SAMPLES * 2) {
    throw new Error("artifact must contain fourteen measured samples");
  }
  for (let run = 1; run <= REQUIRED_SAMPLES; run += 1) {
    const expected = run % 2 === 1 ? ["platform", "virtual"] : ["virtual", "platform"];
    const actual = artifact.samples.slice((run - 1) * 2, run * 2).map((sample) => sample.variant);
    if (actual.join(",") !== expected.join(",")) {
      throw new Error(`sample pair ${run} is not in alternating order`);
    }
  }
  if (artifact.samples.some((sample) => sample.status !== "passed")) {
    throw new Error("artifact contains a failed measured sample");
  }
  for (const sample of artifact.samples) {
    const populations = sample.population_observations?.map((entry) => entry.population);
    if (canonicalJson(populations) !== canonicalJson(WORKLOAD.parked_populations)
        || sample.population_observations.some((entry) =>
          !Number.isFinite(entry.peak_rss_kb) || entry.peak_rss_kb <= 0
          || !Number.isInteger(entry.peak_os_threads) || entry.peak_os_threads <= 0)) {
      throw new Error("artifact is missing external parked-population observations");
    }
  }
  if (artifact.identity?.harness_sha256 !== expectedIdentity.harness_sha256) {
    throw new Error("artifact harness digest does not match the benchmark source");
  }
  if (artifact.identity?.virtual_runtime_source_sha256
      !== expectedIdentity.virtual_runtime_source_sha256
      || artifact.identity?.platform_runtime_source_sha256
      !== expectedIdentity.platform_runtime_source_sha256) {
    throw new Error("artifact runtime digest does not match the current generated runtime");
  }
  requireEquivalent(artifact.identity.jvm_args, JVM_ARGS,
    "artifact JVM arguments do not match the benchmark protocol");
  requireEquivalent(artifact.identity.workload, WORKLOAD,
    "artifact workload identity does not match the benchmark protocol");
  if (artifact.identity.baseline_difference !== PLATFORM_WORKER
      || artifact.identity.candidate_difference !== VIRTUAL_WORKER) {
    throw new Error("artifact worker difference does not match the benchmark protocol");
  }
  const summaries = summarizeSamples(artifact.samples);
  requireEquivalent(artifact.summaries, summaries,
    "artifact summaries do not match the raw samples");
  const adoption = evaluateGates(artifact.samples, summaries);
  requireEquivalent(artifact.adoption, adoption,
    "artifact adoption verdict does not match the raw samples");
  if (adoption.status !== "passed"
      || Object.values(adoption.gates).some((gate) => gate !== true)) {
    const failed = Object.entries(adoption.gates)
      .filter(([, passed]) => passed !== true)
      .map(([gate]) => gate);
    throw new Error(
      `artifact fails adoption gates: ${failed.join(", ")}; inspect the raw samples and rerun the controlled collection after resolving the failed evidence`,
    );
  }
  if (artifact.diagnostics?.jfr_virtual_thread_pinning?.status !== "passed") {
    throw new Error("artifact is missing the JFR pinning diagnostic");
  }
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function buildRuntimeSource() {
  const build = spawnSync("cargo", [
    "build",
    "--locked",
    "-p",
    "veln-backend-jvm",
    "--message-format=json-render-diagnostics",
  ], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });
  if (build.status !== 0) throw new Error(`backend prebuild failed: ${build.stderr}`);
  const candidates = build.stdout.split("\n").flatMap((line) => {
    if (!line.startsWith("{")) return [];
    try {
      const message = JSON.parse(line);
      if (message.reason !== "build-script-executed"
          || !message.package_id?.includes("veln-backend-jvm")) return [];
      const path = join(message.out_dir, "runtime/VelnRuntime.java");
      return existsSync(path) ? [path] : [];
    } catch {
      return [];
    }
  });
  if (candidates.length !== 1) {
    throw new Error(`backend build identified ${candidates.length} generated runtime sources`);
  }
  return candidates[0];
}

export function runtimeSourceVariants(original) {
  const occurrences = original.split(VIRTUAL_WORKER).length - 1;
  if (occurrences !== 1) {
    throw new Error(`runtime contains ${occurrences} virtual worker constructors; expected one`);
  }
  return {
    platform: original.replace(VIRTUAL_WORKER, PLATFORM_WORKER),
    virtual: original,
  };
}

export function benchmarkIdentityForRuntime(original, harnessSha256) {
  const sources = runtimeSourceVariants(original);
  return {
    harness_sha256: harnessSha256,
    platform_runtime_source_sha256: createHash("sha256").update(sources.platform).digest("hex"),
    virtual_runtime_source_sha256: createHash("sha256").update(sources.virtual).digest("hex"),
  };
}

function prepareVariants(root, runtimeSource) {
  const sourceVariants = runtimeSourceVariants(readFileSync(runtimeSource, "utf8"));
  const variants = {};
  for (const variant of ["platform", "virtual"]) {
    const directory = join(root, variant);
    mkdirSync(directory, { recursive: true });
    const source = sourceVariants[variant];
    writeFileSync(join(directory, "VelnRuntime.java"), source);
    cpSync(HARNESS, join(directory, basename(HARNESS)));
    const compiled = spawnSync("javac", ["--release", "25", "VelnRuntime.java", basename(HARNESS)], {
      cwd: directory,
      encoding: "utf8",
    });
    if (compiled.status !== 0) {
      throw new Error(`javac failed for ${variant}: ${compiled.stderr}`);
    }
    variants[variant] = {
      directory,
      runtime_source_sha256: createHash("sha256").update(source).digest("hex"),
    };
  }
  return variants;
}

function procStatus(pid) {
  try {
    const status = readFileSync(`/proc/${pid}/status`, "utf8");
    const rss = Number(/^VmRSS:\s+(\d+) kB$/m.exec(status)?.[1] ?? 0);
    const threads = Number(/^Threads:\s+(\d+)$/m.exec(status)?.[1] ?? 0);
    return { rss, threads };
  } catch {
    return { rss: 0, threads: 0 };
  }
}

async function runSample(variant, run, directory, extraJvmArgs = []) {
  const started = process.hrtime.bigint();
  const child = spawn("java", [...JVM_ARGS, ...extraJvmArgs, "-cp", directory,
    "LightweightTaskBenchmark"], {
    cwd: directory,
    detached: true,
    stdio: ["pipe", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  let stderrPending = "";
  let peakRss = 0;
  let peakThreads = 0;
  let currentParkedObservation = null;
  const parkedObservations = new Map();
  const observationErrors = [];
  const updateParkedObservation = () => {
    if (currentParkedObservation === null) return;
    const current = procStatus(child.pid);
    currentParkedObservation.peak_rss_kb = Math.max(
      currentParkedObservation.peak_rss_kb,
      current.rss,
    );
    currentParkedObservation.peak_os_threads = Math.max(
      currentParkedObservation.peak_os_threads,
      current.threads,
    );
  };
  const handleObservationLine = (line) => {
    const fields = line.split("\t");
    if (fields[0] !== "VELN_BENCHMARK_OBSERVE") return;
    const population = Number(fields[2]);
    if (!WORKLOAD.parked_populations.includes(population)) {
      observationErrors.push(`unexpected parked population ${fields[2]}`);
      return;
    }
    if (fields[1] === "start") {
      if (currentParkedObservation !== null || parkedObservations.has(population)) {
        observationErrors.push(`duplicate parked observation start ${population}`);
        return;
      }
      currentParkedObservation = {
        population,
        peak_rss_kb: 0,
        peak_os_threads: 0,
      };
      updateParkedObservation();
      return;
    }
    if (fields[1] === "ready") {
      if (currentParkedObservation?.population !== population) {
        observationErrors.push(`parked observation ready without start ${population}`);
      } else {
        updateParkedObservation();
        parkedObservations.set(population, currentParkedObservation);
        currentParkedObservation = null;
      }
      child.stdin.write("continue\n");
      return;
    }
    observationErrors.push(`unexpected parked observation event ${fields[1]}`);
  };
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => { stdout += chunk; });
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
    stderrPending += chunk;
    while (stderrPending.includes("\n")) {
      const newline = stderrPending.indexOf("\n");
      handleObservationLine(stderrPending.slice(0, newline).replace(/\r$/, ""));
      stderrPending = stderrPending.slice(newline + 1);
    }
    if (process.env.VELN_BENCHMARK_DEBUG === "1") process.stderr.write(chunk);
  });
  const sampler = setInterval(() => {
    const current = procStatus(child.pid);
    peakRss = Math.max(peakRss, current.rss);
    peakThreads = Math.max(peakThreads, current.threads);
    updateParkedObservation();
    if (current.rss > PROCESS_TREE_RSS_LIMIT_KB) {
      try { process.kill(-child.pid, "SIGKILL"); } catch { /* exited */ }
    }
  }, 5);
  const timer = setTimeout(() => {
    try { process.kill(-child.pid, "SIGKILL"); } catch { /* exited */ }
  }, TIMEOUT_MILLIS);
  const result = await new Promise((resolveResult) => {
    child.on("error", (error) => resolveResult({ code: null, signal: null, error }));
    child.on("exit", (code, signal) => resolveResult({ code, signal, error: null }));
  });
  clearInterval(sampler);
  clearTimeout(timer);
  const duration = Number(process.hrtime.bigint() - started) / 1_000_000_000;
  const sample = {
    variant,
    run,
    status: result.code === 0 ? "passed" : result.signal === "SIGKILL" ? "timeout" : "failed",
    exit_code: result.code,
    signal: result.signal,
    duration_seconds: duration,
    peak_rss_kb: peakRss,
    peak_os_threads: peakThreads,
    population_observations: [...parkedObservations.values()]
      .sort((left, right) => left.population - right.population),
    stderr: stderr.trim(),
  };
  if (result.error) sample.spawn_error = result.error.message;
  if (sample.status === "passed") {
    try {
      const jsonLine = stdout.trim().split("\n").findLast((line) => line.startsWith("{"));
      sample.result = JSON.parse(jsonLine ?? "");
    } catch (error) {
      sample.status = "failed";
      sample.parse_error = error.message;
      sample.stdout = stdout.trim();
    }
  } else {
    sample.stdout = stdout.trim();
  }
  if (sample.status === "passed"
      && (observationErrors.length > 0
        || sample.population_observations.length !== WORKLOAD.parked_populations.length)) {
    sample.status = "failed";
    sample.observation_error = observationErrors.join("; ")
      || `observed ${sample.population_observations.length} parked populations`;
  }
  return sample;
}

function javaIdentity() {
  const output = spawnSync("java", ["-version"], { encoding: "utf8" });
  return `${output.stdout}${output.stderr}`.trim().split("\n").map((line, index) =>
    index === 0 ? line.replace(/\s+\d{4}-\d{2}-\d{2}$/, "") : line
  ).join("\n");
}

async function countJfrPinningEvents(directory, root) {
  const recording = join(root, "candidate-pinning.jfr");
  const sample = await runSample("virtual", "jfr-diagnostic", directory, [
    `-XX:StartFlightRecording=filename=${recording},settings=profile,dumponexit=true`,
  ]);
  if (sample.status !== "passed" || !existsSync(recording)) {
    return { status: "failed", sample };
  }
  const printed = spawnSync("jfr", ["print", "--json", "--events", "jdk.VirtualThreadPinned", recording], {
    encoding: "utf8",
  });
  if (printed.status !== 0) return { status: "failed", stderr: printed.stderr.trim() };
  const parsed = JSON.parse(printed.stdout);
  return {
    status: "passed",
    events: parsed.recording?.events?.length ?? 0,
    diagnostic_run_duration_seconds: sample.duration_seconds,
  };
}

async function main() {
  const checkIndex = process.argv.indexOf("--check");
  if (checkIndex >= 0) {
    const artifactPath = resolve(process.argv[checkIndex + 1] ?? DEFAULT_OUTPUT);
    const runtimeSource = buildRuntimeSource();
    const expectedIdentity = benchmarkIdentityForRuntime(
      readFileSync(runtimeSource, "utf8"),
      sha256(HARNESS),
    );
    validateArtifact(JSON.parse(readFileSync(artifactPath, "utf8")), expectedIdentity);
    console.log(`validated ${artifactPath}`);
    return;
  }
  const outputIndex = process.argv.indexOf("--output");
  const output = outputIndex >= 0 ? resolve(process.argv[outputIndex + 1]) : DEFAULT_OUTPUT;
  const runtimeSource = buildRuntimeSource();
  const temporary = mkdtempSync(join(tmpdir(), "veln-lightweight-task-benchmark-"));
  try {
    const variants = prepareVariants(temporary, runtimeSource);
    for (const variant of ["platform", "virtual"]) {
      const warmup = await runSample(variant, "warmup", variants[variant].directory);
      if (warmup.status !== "passed") {
        throw new Error(`${variant} warm-up failed: ${warmup.stderr || warmup.stdout}`);
      }
    }
    if (process.argv.includes("--warmup-only")) {
      console.log("warm-ups passed");
      return;
    }
    const samples = [];
    for (let run = 1; run <= REQUIRED_SAMPLES; run += 1) {
      const order = run % 2 === 1 ? ["platform", "virtual"] : ["virtual", "platform"];
      for (const variant of order) {
        samples.push(await runSample(variant, run, variants[variant].directory));
      }
    }
    const summaries = summarizeSamples(samples);
    const artifact = {
      schema_version: 3,
      benchmark: "veln-jvm-lightweight-task-adoption",
      constraints: {
        process_tree_memory_mib: 4096,
        process_tree_memory_enforcement: "runner samples the Java process RSS every 5 milliseconds and terminates its process group above the ceiling",
        java_heap_mib: 512,
        per_jvm_timeout_seconds: 180,
        discarded_warmups_per_variant: 1,
        measured_runs_per_variant: REQUIRED_SAMPLES,
        alternating_order: true,
      },
      environment: {
        java: javaIdentity(),
        operating_system: `${process.platform} ${process.arch}`,
        available_parallelism: Number(spawnSync("nproc", [], { encoding: "utf8" }).stdout.trim()),
      },
      identity: {
        harness_sha256: sha256(HARNESS),
        platform_runtime_source_sha256: variants.platform.runtime_source_sha256,
        virtual_runtime_source_sha256: variants.virtual.runtime_source_sha256,
        baseline_difference: PLATFORM_WORKER,
        candidate_difference: VIRTUAL_WORKER,
        jvm_args: JVM_ARGS,
        workload: WORKLOAD,
      },
      samples,
      summaries,
      diagnostics: {
        jfr_virtual_thread_pinning: await countJfrPinningEvents(variants.virtual.directory, temporary),
      },
    };
    artifact.adoption = evaluateGates(samples, summaries);
    if (artifact.adoption.status !== "passed") {
      console.error("controlled collection did not pass; inspect these summaries before rerunning:");
      console.error(JSON.stringify({ summaries, adoption: artifact.adoption }, null, 2));
    }
    mkdirSync(dirname(output), { recursive: true });
    writeFileSync(output, `${JSON.stringify(artifact, null, 2)}\n`);
    validateArtifact(artifact, benchmarkIdentityForRuntime(
      readFileSync(runtimeSource, "utf8"),
      sha256(HARNESS),
    ));
    console.log(`wrote ${output}`);
    console.log(`adoption: ${artifact.adoption.status}`);
    if (artifact.adoption.status !== "passed") process.exitCode = 1;
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  main().catch((error) => {
    console.error(error.stack ?? error.message);
    process.exitCode = 1;
  });
}
