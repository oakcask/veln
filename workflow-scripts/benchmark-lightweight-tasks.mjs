#!/usr/bin/env node
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
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
const JVM_ARGS = [
  "-Xms512m",
  "-Xmx512m",
  "-Xss256k",
  "-XX:+UseSerialGC",
  "-Djdk.virtualThreadScheduler.parallelism=2",
  "-Djdk.virtualThreadScheduler.maxPoolSize=2",
];
const WORKLOAD = {
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
  cpu_saturators: 2,
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
  return {
    status: "passed",
    peak_rss_kb: metricSummary(values(samples, (sample) => sample.peak_rss_kb)),
    peak_os_threads: metricSummary(values(samples, (sample) => sample.peak_os_threads)),
    parked_256_threads: metricSummary(values(samples,
      (sample) => parkedAt(sample, 256).platform_threads)),
    parked_1024_threads: metricSummary(values(samples,
      (sample) => parkedAt(sample, 1024).platform_threads)),
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
      latency_p95_nanos: metricSummary(values(samples,
        (sample) => sample.result.workload.cpu_saturation.latency_p95_nanos)),
    },
    descriptor_growth: metricSummary(values(samples,
      (sample) => sample.result.workload.loopback.descriptor_growth)),
    selector_operations: metricSummary(values(samples,
      (sample) => sample.result.workload.loopback.selector_operations)),
    reclamation: {
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

export function evaluateGates(samples, summaries) {
  const baseline = summaries.platform;
  const candidate = summaries.virtual;
  const passedSamples = samples.filter((sample) => sample.status === "passed");
  const functional = passedSamples.length === REQUIRED_SAMPLES * 2
    && new Set(passedSamples.map((sample) => sample.result.functional_result)).size === 1;
  const commonPopulation = ["platform", "virtual"].every((variant) =>
    samples.filter((sample) => sample.variant === variant).every((sample) =>
      sample.status === "passed" && parkedAt(sample, 1024)?.completed === true));
  const noiseMetrics = [];
  for (const variant of [baseline, candidate]) {
    if (variant.status !== "passed") continue;
    noiseMetrics.push(variant.peak_rss_kb);
    for (const workload of Object.values(variant.workload)) {
      noiseMetrics.push(workload.throughput_per_second);
    }
    noiseMetrics.push(variant.select.latency_p95_nanos);
  }
  const conclusive = noiseMetrics.length === 10 && noiseMetrics.every((metric) => metric.stable);
  const gates = {
    functional_results_equal: functional,
    common_population_completed: commonPopulation,
    candidate_16384_completed: candidate.parked_16384_completed === true,
    candidate_platform_thread_growth_at_most_8:
      candidate.status === "passed"
      && candidate.parked_16384_completed
      && candidate.parked_16384_threads?.median - candidate.parked_256_threads.median <= 8,
    candidate_peak_rss_not_above_baseline:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.peak_rss_kb.median <= baseline.peak_rss_kb.median,
    candidate_worker_threads_at_most_quarter_baseline:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.parked_1024_threads.median <= baseline.parked_1024_threads.median / 4,
    channel_throughput_at_least_80_percent: ["rendezvous_channel", "bounded_channel"].every(
      (workload) => candidate.status === "passed" && baseline.status === "passed"
        && candidate.workload[workload].throughput_per_second.median
          >= baseline.workload[workload].throughput_per_second.median * 0.8,
    ),
    loopback_throughput_at_least_80_percent:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.workload.loopback.throughput_per_second.median
        >= baseline.workload.loopback.throughput_per_second.median * 0.8,
    delivery_latency_p95_at_most_125_percent:
      candidate.status === "passed" && baseline.status === "passed"
      && candidate.select.latency_p95_nanos.median <= baseline.select.latency_p95_nanos.median * 1.25,
    reclamation_final_five_within_10_percent: candidate.reclamation?.final_five_stable === true,
    completed_registrations_absent: candidate.reclamation?.completed_registrations_zero === true,
    released_owners_absent: candidate.reclamation?.retained_owners_zero === true,
    sample_ranges_conclusive: conclusive,
  };
  return {
    status: Object.values(gates).every(Boolean) ? "passed" : "failed",
    gates,
  };
}

export function validateArtifact(artifact, harnessSha256) {
  if (artifact.schema_version !== 1 || artifact.benchmark !== "veln-jvm-lightweight-task-adoption") {
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
  if (artifact.identity?.harness_sha256 !== harnessSha256) {
    throw new Error("artifact harness digest does not match the benchmark source");
  }
  if (artifact.adoption?.status !== "passed"
      || Object.values(artifact.adoption.gates ?? {}).some((gate) => gate !== true)) {
    throw new Error("artifact does not pass every adoption gate");
  }
  if (artifact.diagnostics?.jfr_virtual_thread_pinning?.status !== "passed") {
    throw new Error("artifact is missing the JFR pinning diagnostic");
  }
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function newestRuntimeSource() {
  const buildRoot = join(REPO_ROOT, "target/debug/build");
  const candidates = readdirSync(buildRoot)
    .filter((entry) => entry.startsWith("veln-backend-jvm-"))
    .map((entry) => join(buildRoot, entry, "out/runtime/VelnRuntime.java"))
    .filter(existsSync)
    .sort((left, right) => statSync(right).mtimeMs - statSync(left).mtimeMs);
  if (candidates.length === 0) throw new Error("prebuilt JVM runtime source was not found");
  return candidates[0];
}

function prepareVariants(root, runtimeSource) {
  const original = readFileSync(runtimeSource, "utf8");
  const virtualWorker = "return Thread.startVirtualThread(task);";
  if (!original.includes(virtualWorker)) {
    throw new Error("runtime does not contain the expected virtual worker constructor");
  }
  const variants = {};
  for (const variant of ["platform", "virtual"]) {
    const directory = join(root, variant);
    mkdirSync(directory, { recursive: true });
    const source = variant === "platform"
      ? original.replace(virtualWorker, "return Thread.ofPlatform().daemon(false).start(task);")
      : original;
    if (variant === "platform" && source.includes(virtualWorker)) {
      throw new Error("platform baseline did not replace exactly one worker constructor");
    }
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
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  let peakRss = 0;
  let peakThreads = 0;
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => { stdout += chunk; });
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
    if (process.env.VELN_BENCHMARK_DEBUG === "1") process.stderr.write(chunk);
  });
  const sampler = setInterval(() => {
    const current = procStatus(child.pid);
    peakRss = Math.max(peakRss, current.rss);
    peakThreads = Math.max(peakThreads, current.threads);
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
    validateArtifact(JSON.parse(readFileSync(artifactPath, "utf8")), sha256(HARNESS));
    console.log(`validated ${artifactPath}`);
    return;
  }
  const outputIndex = process.argv.indexOf("--output");
  const output = outputIndex >= 0 ? resolve(process.argv[outputIndex + 1]) : DEFAULT_OUTPUT;
  const build = spawnSync("cargo", ["build", "--locked", "-p", "veln-backend-jvm"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });
  if (build.status !== 0) throw new Error(`backend prebuild failed: ${build.stderr}`);
  const temporary = mkdtempSync(join(tmpdir(), "veln-lightweight-task-benchmark-"));
  try {
    const runtimeSource = newestRuntimeSource();
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
    const byVariant = Object.fromEntries(["platform", "virtual"].map((variant) => [
      variant,
      samples.filter((sample) => sample.variant === variant),
    ]));
    const summaries = Object.fromEntries(Object.entries(byVariant).map(([variant, entries]) => [
      variant,
      summarizeVariant(entries),
    ]));
    if (summaries.virtual.status === "passed") {
      summaries.virtual.parked_16384_threads = metricSummary(values(byVariant.virtual,
        (sample) => parkedAt(sample, 16384).platform_threads));
    }
    const artifact = {
      schema_version: 1,
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
        baseline_difference: "Thread.ofPlatform().daemon(false).start(task)",
        candidate_difference: "Thread.startVirtualThread(task)",
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
    validateArtifact(artifact, sha256(HARNESS));
    mkdirSync(dirname(output), { recursive: true });
    writeFileSync(output, `${JSON.stringify(artifact, null, 2)}\n`);
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
