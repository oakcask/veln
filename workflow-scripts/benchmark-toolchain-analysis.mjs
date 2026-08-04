#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { mkdtempSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const DEFAULT_WORKLOADS = [
  {
    id: "small_schema",
    commandKind: "veln",
    cwd: "examples/specification/run/schema-decode-expression",
    args: ["run", "--json", "main", "main.veln", "wire.veln", "facade.veln"],
  },
  {
    id: "hpack_static",
    commandKind: "veln",
    cwd: "examples/specification/run/hpack-static-index-projection-human",
    args: ["run", "main", "main.veln"],
  },
  {
    id: "http2_core",
    commandKind: "veln",
    cwd: "examples/specification/run/http2-protocol-core-continuation-closed-json",
    args: ["run", "--json", "main", "main.veln"],
  },
  {
    id: "http2_connection",
    commandKind: "veln",
    cwd: "examples/specification/run/http2-connection-application-unsupported-request-json",
    args: ["run", "--json", "main", "main.veln"],
    env: {
      VELN_NET_RUNTIME: "production-loopback",
      VELN_NET_PRODUCTION_READS_HEX:
        "505249202a20485454502f322e300d0a0d0a534d0d0a0d0a000006040000000000000500008000000003010400000001828784",
    },
  },
];

const NOISE_BOUNDARY_RATIO = 0.1;

export function median(values) {
  if (values.length === 0) {
    throw new Error("median requires at least one value");
  }
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) {
    return sorted[middle];
  }
  return (sorted[middle - 1] + sorted[middle]) / 2;
}

export function medianAbsoluteDeviation(values) {
  const center = median(values);
  return median(values.map((value) => Math.abs(value - center)));
}

export function normalizeFunctionalOutput(output, replacements = {}) {
  let normalized = output.replace(/\r\n/g, "\n");
  for (const [from, to] of Object.entries(replacements)) {
    normalized = normalized.split(from).join(to);
  }
  return normalized.trimEnd();
}

export function generateAnnotatedModuleGraph(root, size) {
  if (!Number.isInteger(size) || size < 2) {
    throw new Error("generated workload size must be an integer greater than one");
  }

  const imports = [];
  const calls = [];
  for (let index = 0; index < size; index += 1) {
    const moduleName = `generated_${index}`;
    imports.push(`use ${moduleName}`);
    calls.push(`${moduleName}::value_${index}()`);
    writeFileSync(
      join(root, `${moduleName}.veln`),
      [
        `pub fn value_${index}() -> Int`,
        `\tlet seed: Int = ${index + 1}`,
        "\tseed",
        "end",
        "",
      ].join("\n"),
      "utf8",
    );
  }

  writeFileSync(
    join(root, "main.veln"),
    [
      ...imports,
      "",
      "pub fn main() -> Int",
      `\t${calls.join(" + ")}`,
      "end",
      "",
    ].join("\n"),
    "utf8",
  );

  return {
    size,
    moduleCount: size + 1,
    command: ["check", "--json"],
  };
}

export function workloadCommand(binary, workload) {
  if (workload.commandKind === "shell") {
    return workload.args;
  }
  return [binary, ...workload.args];
}

export function summarizeRuns(runs) {
  const wallTimes = runs.map((run) => run.wall_time_seconds);
  const userCpuTimes = runs.map((run) => run.user_cpu_seconds);
  const wallMedian = median(wallTimes);
  const wallMad = medianAbsoluteDeviation(wallTimes);
  return {
    median_wall_time_seconds: wallMedian,
    median_user_cpu_seconds: median(userCpuTimes),
    median_absolute_deviation_wall_time_seconds: wallMad,
    wall_time_noisy: wallMad > wallMedian * NOISE_BOUNDARY_RATIO,
  };
}

export function compareFunctionalOutputs(left, right) {
  return (
    left.exit_status === right.exit_status &&
    left.normalized_stdout === right.normalized_stdout &&
    left.normalized_stderr === right.normalized_stderr
  );
}

export function functionalSnapshot(run, replacements = {}) {
  return {
    exit_status: run.exit_status,
    normalized_stdout: normalizeFunctionalOutput(run.stdout, replacements),
    normalized_stderr: normalizeFunctionalOutput(run.stderr, replacements),
  };
}

export function thresholdDecisions(result) {
  const workloads = new Map(result.workloads.map((workload) => [workload.id, workload]));
  const decisions = [];

  const ratioDecision = (id, description, numerator, denominator, maxRatio) => {
    if (numerator == null || denominator == null) {
      decisions.push({ id, description, status: "skipped", reason: "required workload is unavailable" });
      return;
    }
    const ratio = numerator / Math.max(denominator, 0.000001);
    decisions.push({
      id,
      description,
      status: ratio <= maxRatio ? "passed" : "failed",
      ratio,
      max_ratio: maxRatio,
    });
  };

  const newMedian = (id, metric) => workloads.get(id)?.new?.summary?.[metric];
  const baselineMedian = (id, metric) => workloads.get(id)?.baseline?.summary?.[metric];
  const functionalFailures = result.workloads.filter((workload) => !workload.functional_outputs_equal);
  const noisyWorkloads = result.workloads.filter(
    (workload) =>
      workload.baseline.summary.wall_time_noisy || workload.new.summary.wall_time_noisy,
  );
  const performanceThresholds = [
    {
      id: "http2_core_improvement",
      description: "HTTP/2 core direct analysis new wall-time median is at most one third of baseline",
      numerator: newMedian("http2_core", "median_wall_time_seconds"),
      denominator: baselineMedian("http2_core", "median_wall_time_seconds"),
      maxRatio: 1 / 3,
    },
    {
      id: "http2_connection_improvement",
      description: "HTTP/2 connection direct analysis new wall-time median is at most one third of baseline",
      numerator: newMedian("http2_connection", "median_wall_time_seconds"),
      denominator: baselineMedian("http2_connection", "median_wall_time_seconds"),
      maxRatio: 1 / 3,
    },
    {
      id: "toolchain_case_overhead",
      description: "toolchain-case median is no more than 1.35 times the direct invocation median",
      numerator: newMedian("http2_core_toolchain_case", "median_wall_time_seconds"),
      denominator: newMedian("http2_core", "median_wall_time_seconds"),
      maxRatio: 1.35,
    },
    {
      id: "generated_first_to_second",
      description: "first generated size to second size user CPU median grows by no more than 2.5 times",
      numerator: newMedian("generated_2", "median_user_cpu_seconds"),
      denominator: newMedian("generated_1", "median_user_cpu_seconds"),
      maxRatio: 2.5,
    },
    {
      id: "generated_second_to_third",
      description: "second generated size to third size user CPU median grows by no more than 2.5 times",
      numerator: newMedian("generated_3", "median_user_cpu_seconds"),
      denominator: newMedian("generated_2", "median_user_cpu_seconds"),
      maxRatio: 2.5,
    },
  ];

  if (functionalFailures.length === 0 && noisyWorkloads.length === 0) {
    for (const threshold of performanceThresholds) {
      ratioDecision(
        threshold.id,
        threshold.description,
        threshold.numerator,
        threshold.denominator,
        threshold.maxRatio,
      );
    }
  } else {
    const reason =
      functionalFailures.length > 0
        ? "functional outputs differ"
        : "wall-time measurements are noisy";
    for (const threshold of performanceThresholds) {
      decisions.push({
        id: threshold.id,
        description: threshold.description,
        status: "skipped",
        reason,
      });
    }
  }

  decisions.push({
    id: "functional_outputs",
    description: "baseline and new exit status and normalized output are equal for every workload",
    status: functionalFailures.length === 0 ? "passed" : "failed",
    failing_workloads: functionalFailures.map((workload) => workload.id),
  });
  decisions.push({
    id: "wall_time_noise",
    description: "wall-time median absolute deviation is within the noise boundary for every workload",
    status: noisyWorkloads.length === 0 ? "passed" : "failed",
    noisy_workloads: noisyWorkloads.map((workload) => workload.id),
  });

  return decisions;
}

export function passesBenchmarkThresholds(thresholds) {
  return thresholds.every((threshold) => threshold.status === "passed");
}

export function stableJson(value) {
  return `${JSON.stringify(sortJson(value), null, 2)}\n`;
}

function sortJson(value) {
  if (Array.isArray(value)) {
    return value.map(sortJson);
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, entry]) => [key, sortJson(entry)]),
    );
  }
  return value;
}

function parseArgs(argv) {
  const [command, baselineBinary, newBinary, ...rest] = argv;
  if (command !== "compare" || !baselineBinary || !newBinary) {
    throw new Error("usage: benchmark-toolchain-analysis compare BASELINE_BINARY NEW_BINARY [--output PATH] [--runs N] [--warmups N] [--sizes A,B,C]");
  }
  const args = {
    command,
    baselineBinary,
    newBinary,
    output: null,
    runs: 5,
    warmups: 1,
    sizes: [32, 64, 128],
    repoRoot: resolve(dirname(fileURLToPath(import.meta.url)), ".."),
  };
  for (let index = 0; index < rest.length; index += 1) {
    const flag = rest[index];
    const value = rest[index + 1];
    if (flag === "--output") {
      args.output = value;
      index += 1;
    } else if (flag === "--runs") {
      args.runs = Number.parseInt(value, 10);
      index += 1;
    } else if (flag === "--warmups") {
      args.warmups = Number.parseInt(value, 10);
      index += 1;
    } else if (flag === "--sizes") {
      args.sizes = value.split(",").map((entry) => Number.parseInt(entry, 10));
      index += 1;
    } else {
      throw new Error(`unknown option: ${flag}`);
    }
  }
  if (
    args.runs < 1 ||
    args.warmups < 0 ||
    args.sizes.length !== 3 ||
    args.sizes.some((size) => !Number.isInteger(size) || size < 2) ||
    !(
      args.sizes[1] === args.sizes[0] * 2 &&
      args.sizes[2] === args.sizes[1] * 2
    )
  ) {
    throw new Error(
      "--runs must be positive, --warmups must not be negative, and --sizes must contain three adjacent doubling integers greater than one",
    );
  }
  return args;
}

function parseShellCommand(command) {
  return ["sh", "-c", command];
}

function runMeasured(command, options) {
  const completed = spawnSync("/usr/bin/time", ["-f", "__veln_time__ %e %U", ...command], {
    cwd: options.cwd,
    env: options.env,
    encoding: "utf8",
    maxBuffer: 10 * 1024 * 1024,
  });
  const stderr = completed.stderr ?? "";
  const timeLine = stderr.split("\n").find((line) => line.startsWith("__veln_time__ "));
  if (!timeLine) {
    throw new Error(`time output was missing for command: ${command.join(" ")}`);
  }
  const [, wall, user] = timeLine.split(/\s+/);
  return {
    exit_status: completed.status ?? 1,
    stdout: completed.stdout ?? "",
    stderr: stderr
      .split("\n")
      .filter((line) => !line.startsWith("__veln_time__ "))
      .join("\n"),
    wall_time_seconds: Number.parseFloat(wall),
    user_cpu_seconds: Number.parseFloat(user),
  };
}

function prepareWorkloads(repoRoot, sizes, generatedRoot) {
  const workloads = DEFAULT_WORKLOADS.map((workload) => ({
    ...workload,
    cwd: resolve(repoRoot, workload.cwd),
  }));
  sizes.forEach((size, index) => {
    const actualCwd = mkdtempSync(join(generatedRoot, `generated-${index + 1}-`));
    const generated = generateAnnotatedModuleGraph(actualCwd, size);
    workloads.push({
      id: `generated_${index + 1}`,
      commandKind: "veln",
      cwd: actualCwd,
      displayCwd: `<temporary>/generated_${index + 1}`,
      args: generated.command,
      generated,
    });
  });
  if (process.env.VELN_TOOLCHAIN_CASE_COMMAND) {
    workloads.push({
      id: "http2_core_toolchain_case",
      commandKind: "shell",
      cwd: repoRoot,
      args: parseShellCommand(process.env.VELN_TOOLCHAIN_CASE_COMMAND),
      displayCommand: process.env.VELN_TOOLCHAIN_CASE_COMMAND,
    });
  }
  return workloads;
}

function measurePair(args, workload) {
  const env = { ...process.env, ...(workload.env ?? {}) };
  const baselineBinary = realpathSync(args.baselineBinary);
  const newBinary = realpathSync(args.newBinary);
  const replacements = {
    [args.repoRoot]: "<repo>",
    [workload.cwd]: "<workload>",
    [baselineBinary]: "<baseline-binary>",
    [newBinary]: "<new-binary>",
  };
  const baselineCommand = workloadCommand(baselineBinary, workload);
  const newCommand = workloadCommand(newBinary, workload);

  for (let index = 0; index < args.warmups; index += 1) {
    if (index % 2 === 0) {
      runMeasured(baselineCommand, { cwd: workload.cwd, env });
      runMeasured(newCommand, { cwd: workload.cwd, env });
    } else {
      runMeasured(newCommand, { cwd: workload.cwd, env });
      runMeasured(baselineCommand, { cwd: workload.cwd, env });
    }
  }

  const baselineRuns = [];
  const newRuns = [];
  for (let index = 0; index < args.runs; index += 1) {
    if (index % 2 === 0) {
      baselineRuns.push(runMeasured(baselineCommand, { cwd: workload.cwd, env }));
      newRuns.push(runMeasured(newCommand, { cwd: workload.cwd, env }));
    } else {
      newRuns.push(runMeasured(newCommand, { cwd: workload.cwd, env }));
      baselineRuns.push(runMeasured(baselineCommand, { cwd: workload.cwd, env }));
    }
  }

  const functionalComparisons = baselineRuns.map((baselineRun, index) =>
    compareFunctionalOutputs(
      functionalSnapshot(baselineRun, replacements),
      functionalSnapshot(newRuns[index], replacements),
    ),
  );

  return {
    id: workload.id,
    cwd: workload.displayCwd ?? (relative(args.repoRoot, workload.cwd) || "."),
    command: {
      baseline: baselineCommand,
      new: newCommand,
      display: workload.displayCommand ?? null,
    },
    generated: workload.generated ?? null,
    baseline: {
      binary: baselineBinary,
      summary: summarizeRuns(baselineRuns),
      runs: baselineRuns.map(({ wall_time_seconds, user_cpu_seconds, exit_status }) => ({
        exit_status,
        user_cpu_seconds,
        wall_time_seconds,
      })),
    },
    new: {
      binary: newBinary,
      summary: summarizeRuns(newRuns),
      runs: newRuns.map(({ wall_time_seconds, user_cpu_seconds, exit_status }) => ({
        exit_status,
        user_cpu_seconds,
        wall_time_seconds,
      })),
    },
    functional_output_comparisons: functionalComparisons,
    functional_outputs_equal: functionalComparisons.every(Boolean),
  };
}

function summarizeForHuman(result) {
  const skipped = result.thresholds.filter((threshold) => threshold.status === "skipped");
  console.log(`toolchain analysis benchmark: ${result.passes_thresholds ? "passed" : "failed"}`);
  for (const workload of result.workloads) {
    const noisy = workload.baseline.summary.wall_time_noisy || workload.new.summary.wall_time_noisy;
    if (workload.functional_outputs_equal && !noisy) {
      console.log(
        `${workload.id}: baseline ${workload.baseline.summary.median_wall_time_seconds.toFixed(3)}s, new ${workload.new.summary.median_wall_time_seconds.toFixed(3)}s, functional equal`,
      );
    } else {
      console.log(
        `${workload.id}: performance suppressed, functional ${workload.functional_outputs_equal ? "equal" : "different"}${noisy ? ", noisy" : ""}`,
      );
    }
  }
  for (const threshold of result.thresholds) {
    console.log(`${threshold.id}: ${threshold.status}`);
  }
  if (skipped.length > 0) {
    console.log("skipped thresholds require VELN_TOOLCHAIN_CASE_COMMAND.");
  }
}

export function main(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const generatedRoot = mkdtempSync(join(tmpdir(), "veln-toolchain-analysis-"));
  try {
    const workloads = prepareWorkloads(args.repoRoot, args.sizes, generatedRoot);
    const result = {
      command: "benchmark-toolchain-analysis compare",
      measured_runs: args.runs,
      warmup_runs: args.warmups,
      workloads: workloads.map((workload) => measurePair(args, workload)),
    };
    result.thresholds = thresholdDecisions(result);
    result.passes_thresholds = passesBenchmarkThresholds(result.thresholds);
    summarizeForHuman(result);
    if (args.output) {
      writeFileSync(args.output, stableJson(result), "utf8");
    }
    if (!result.passes_thresholds) {
      process.exitCode = 1;
    }
    return result;
  } finally {
    rmSync(generatedRoot, { recursive: true, force: true });
  }
}

if (process.argv[1]?.endsWith("benchmark-toolchain-analysis.mjs")) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 2;
  }
}
