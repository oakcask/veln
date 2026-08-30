import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  benchmarkCommand,
  compareFunctionalOutputs,
  DEFAULT_WORKLOADS,
  dominantMeasuredStage,
  functionalSnapshot,
  generateAnnotatedModuleGraph,
  median,
  medianAbsoluteDeviation,
  passesBenchmarkThresholds,
  parseUserCpuSeconds,
  parseTimingRecords,
  stableJson,
  validateBenchmarkResult,
  validateMeasuredRuns,
  validateStageSummaryFitsRuns,
  summarizeStageRecords,
  summarizeRuns,
  thresholdDecisions,
} from "./benchmark-toolchain-analysis.mjs";

test("uses the complete tracked schema decode command", () => {
  assert.deepEqual(
    DEFAULT_WORKLOADS.find((workload) => workload.id === "small_schema").args,
    ["run", "--json", "main", "main.veln", "wire.veln", "facade.veln"],
  );
});

test("records replay command labels separately from executable paths", () => {
  assert.deepEqual(
    benchmarkCommand({
      baselineBinary: "target/debug/veln-before-stage-timing",
      newBinary: "target/debug/veln",
      baselineLabel: "<baseline-debug-veln>",
      newLabel: "target/debug/veln",
      baselineIdentity: "baseline build",
      newIdentity: "new build",
      buildProfile: "debug",
      runs: 5,
      warmups: 1,
      sizes: [32, 64, 128],
      output: "docs/reviews/toolchain-analysis-stage-benchmark.json",
    }),
    [
      "benchmark-toolchain-analysis",
      "compare",
      "target/debug/veln-before-stage-timing",
      "target/debug/veln",
      "--build-profile",
      "debug",
      "--runs",
      "5",
      "--warmups",
      "1",
      "--sizes",
      "32,64,128",
      "--baseline-label",
      "<baseline-debug-veln>",
      "--baseline-identity",
      "baseline build",
      "--new-identity",
      "new build",
      "--output",
      "docs/reviews/toolchain-analysis-stage-benchmark.json",
    ],
  );
});

test("generates adjacent fully annotated module graph workloads", () => {
  const root = mkdtempSync(join(tmpdir(), "veln-benchmark-generation-"));
  const generated = generateAnnotatedModuleGraph(root, 4);

  assert.deepEqual(generated, {
    size: 4,
    moduleCount: 5,
    command: ["check", "--json"],
  });
  assert.match(readFileSync(join(root, "main.veln"), "utf8"), /use generated_3/);
  assert.match(
    readFileSync(join(root, "main.veln"), "utf8"),
    /generated_0::value_0\(\) \+ generated_1::value_1\(\) \+ generated_2::value_2\(\) \+ generated_3::value_3\(\)/,
  );
  assert.match(
    readFileSync(join(root, "generated_3.veln"), "utf8"),
    /pub fn value_3\(\) -> Int\n\tlet seed: Int = 4\n\tseed/,
  );
  assert.doesNotMatch(readFileSync(join(root, "generated_3.veln"), "utf8"), /use generated_2/);
});

test("calculates median and median absolute deviation", () => {
  assert.equal(median([9, 1, 5]), 5);
  assert.equal(median([10, 2, 8, 4]), 6);
  assert.equal(medianAbsoluteDeviation([10, 12, 14, 40, 16]), 2);
});

test("normalizes functional output before comparison", () => {
  const left = functionalSnapshot(
    {
      exit_status: 1,
      stdout: "ok\r\n/tmp/run\n",
      stderr: "",
    },
    { "/tmp/run": "<workload>" },
  );
  const right = {
    exit_status: 1,
    normalized_stdout: "ok\n<workload>",
    normalized_stderr: "",
  };

  assert.equal(compareFunctionalOutputs(left, right), true);
});

test("marks noisy wall-time summaries", () => {
  const summary = summarizeRuns([
    { wall_time_seconds: 1, user_cpu_seconds: 2 },
    { wall_time_seconds: 1, user_cpu_seconds: 2.1 },
    { wall_time_seconds: 1.5, user_cpu_seconds: 2.2 },
    { wall_time_seconds: 1.8, user_cpu_seconds: 2.3 },
    { wall_time_seconds: 3, user_cpu_seconds: 2.4 },
  ]);

  assert.equal(summary.median_wall_time_seconds, 1.5);
  assert.equal(summary.median_absolute_deviation_wall_time_seconds, 0.5);
  assert.equal(summary.wall_time_noisy, true);
});

test("rejects invalid measured run durations", () => {
  assert.throws(
    () => validateMeasuredRuns([{ wall_time_seconds: -1, user_cpu_seconds: 0.1 }]),
    /invalid wall_time_seconds/,
  );
  assert.throws(
    () => validateMeasuredRuns([{ wall_time_seconds: 1, user_cpu_seconds: Number.NaN }]),
    /invalid user_cpu_seconds/,
  );
});

test("evaluates threshold decisions including skipped toolchain-case comparison", () => {
  const result = {
    workloads: [
      workload("http2_core", 9, 3, 1, true),
      workload("http2_connection", 12, 4, 1, true),
      workload("generated_1", 1, 1, 1, true),
      workload("generated_2", 1, 1, 2.5, true),
      workload("generated_3", 1, 1, 6.25, true),
    ],
  };

  const decisions = thresholdDecisions(result);

  assert.equal(decisions.find((decision) => decision.id === "http2_core_improvement").status, "passed");
  assert.equal(decisions.find((decision) => decision.id === "toolchain_case_overhead").status, "skipped");
  assert.equal(decisions.find((decision) => decision.id === "generated_second_to_third").status, "passed");
  assert.equal(decisions.find((decision) => decision.id === "functional_outputs").status, "passed");
  assert.equal(decisions.find((decision) => decision.id === "wall_time_noise").status, "passed");
  assert.equal(passesBenchmarkThresholds(decisions), false);
});

test("requires every acceptance threshold to pass", () => {
  assert.equal(
    passesBenchmarkThresholds([
      { id: "one", status: "passed" },
      { id: "two", status: "skipped" },
    ]),
    false,
  );
  assert.equal(
    passesBenchmarkThresholds([
      { id: "one", status: "passed" },
      { id: "two", status: "failed" },
    ]),
    false,
  );
  assert.equal(passesBenchmarkThresholds([{ id: "one", status: "passed" }]), true);
});

test("fails acceptance and suppresses performance thresholds for noisy runs", () => {
  const result = {
    workloads: thresholdWorkloads({ coreOptions: { baselineNoisy: true } }),
  };

  const decisions = thresholdDecisions(result);

  assert.equal(decisions.find((decision) => decision.id === "http2_core_improvement").status, "skipped");
  assert.equal(
    decisions.find((decision) => decision.id === "http2_core_improvement").reason,
    "wall-time measurements are noisy",
  );
  assert.equal(decisions.find((decision) => decision.id === "wall_time_noise").status, "failed");
  assert.deepEqual(decisions.find((decision) => decision.id === "wall_time_noise").noisy_workloads, [
    "http2_core",
  ]);
  assert.equal(passesBenchmarkThresholds(decisions), false);
});

test("suppresses performance thresholds when functional outputs differ", () => {
  const result = {
    workloads: thresholdWorkloads({ connectionOutputsEqual: false }),
  };

  const decisions = thresholdDecisions(result);

  assert.equal(decisions.find((decision) => decision.id === "http2_connection_improvement").status, "skipped");
  assert.equal(
    decisions.find((decision) => decision.id === "http2_connection_improvement").reason,
    "functional outputs differ",
  );
  assert.equal(decisions.find((decision) => decision.id === "functional_outputs").status, "failed");
  assert.deepEqual(decisions.find((decision) => decision.id === "functional_outputs").failing_workloads, [
    "http2_connection",
  ]);
  assert.equal(passesBenchmarkThresholds(decisions), false);
});

function thresholdWorkloads({ coreOptions = {}, connectionOutputsEqual = true } = {}) {
  return [
    workload("http2_core", 9, 3, 1, true, coreOptions),
    workload("http2_connection", 12, 4, 1, connectionOutputsEqual),
    workload("generated_1", 1, 1, 1, true),
    workload("generated_2", 1, 1, 2.5, true),
    workload("generated_3", 1, 1, 6.25, true),
    workload("http2_core_toolchain_case", 1, 1.3, 1, true),
  ];
}

test("writes deterministic machine-readable JSON", () => {
  assert.equal(
    stableJson({ z: 1, nested: { b: 2, a: 1 }, list: [{ d: 4, c: 3 }] }),
    '{\n  "list": [\n    {\n      "c": 3,\n      "d": 4\n    }\n  ],\n  "nested": {\n    "a": 1,\n    "b": 2\n  },\n  "z": 1\n}\n',
  );
});

test("parses user CPU time independently from wall clock text", () => {
  assert.equal(parseUserCpuSeconds("__veln_time__ -3.87 0.95\n", ["veln", "run"]), 0.95);
  assert.throws(
    () => parseUserCpuSeconds("__veln_time__ 1.25 -0.01\n", ["veln", "run"]),
    /invalid user CPU seconds/,
  );
  assert.throws(
    () => parseUserCpuSeconds("runtime stderr\n", ["veln", "run"]),
    /time output was missing/,
  );
});

test("parses and validates stage timing records", () => {
  const records = parseTimingRecords(
    [
      '{"workload":"http2_core","run":"new-1","stage":"source_loading","boundary":"source_loading","duration_seconds":0.25}',
      '{"workload":"http2_core","run":"new-1","stage":"semantic_environment_check","boundary":"semantic_environment_check","duration_seconds":1.5}',
      "",
    ].join("\n"),
  );

  assert.deepEqual(records, [
    {
      workload: "http2_core",
      run: "new-1",
      stage: "source_loading",
      boundary: "source_loading",
      duration_seconds: 0.25,
    },
    {
      workload: "http2_core",
      run: "new-1",
      stage: "semantic_environment_check",
      boundary: "semantic_environment_check",
      duration_seconds: 1.5,
    },
  ]);
  assert.throws(
    () =>
      parseTimingRecords(
        [
          '{"workload":"http2_core","run":"new-1","stage":"source_loading","boundary":"source_loading","duration_seconds":0.25}',
          '{"workload":"http2_core","run":"new-1","stage":"source_loading","boundary":"source_loading","duration_seconds":0.3}',
        ].join("\n"),
      ),
    /duplicate timing record/,
  );
  assert.throws(
    () =>
      parseTimingRecords(
        '{"workload":"http2_core","run":"new-1","stage":"source_loading","boundary":"source_loading","duration_seconds":-1}\n',
      ),
    /invalid duration/,
  );
  assert.throws(
    () =>
      parseTimingRecords(
        '{"workload":"http2_core","run":"new-1","stage":"source_loading","duration_seconds":1}\n',
      ),
    /must identify workload, run, stage, and boundary/,
  );
  assert.throws(
    () =>
      parseTimingRecords(
        '{"workload":"http2_core","run":"new-1","stage":"source_loading","boundary":"parse","duration_seconds":1}\n',
      ),
    /measured pipeline boundary/,
  );
  assert.throws(
    () =>
      parseTimingRecords(
        '{"workload":"http2_core","run":"new-1","stage":"made_up_stage","boundary":"made_up_stage","duration_seconds":1}\n',
      ),
    /unknown measured pipeline stage/,
  );
  assert.throws(
    () =>
      parseTimingRecords(
        '{"workload":"http2_connection","run":"new-1","stage":"source_loading","boundary":"source_loading","duration_seconds":1}\n',
        { workload: "http2_core" },
      ),
    /unexpected workload/,
  );
});

test("aggregates stage medians and selects the dominant measured stage", () => {
  const summary = summarizeStageRecords(
    [
      timing("http2_connection", "new-1", "source_loading", 0.1),
      timing("http2_connection", "new-1", "surface_parse_lower", 0.2),
      timing("http2_connection", "new-1", "semantic_environment_check", 4),
      timing("http2_connection", "new-1", "reachable_entry_lowering", 0.4),
      timing("http2_connection", "new-1", "backend_runtime_remainder", 0.5),
      timing("http2_connection", "new-2", "source_loading", 0.3),
      timing("http2_connection", "new-2", "surface_parse_lower", 0.6),
      timing("http2_connection", "new-2", "semantic_environment_check", 2),
      timing("http2_connection", "new-2", "reachable_entry_lowering", 0.6),
      timing("http2_connection", "new-2", "backend_runtime_remainder", 0.7),
    ],
    ["new-1", "new-2"],
  );

  assert.deepEqual(summary.stage_medians_seconds, {
    backend_runtime_remainder: 0.6,
    reachable_entry_lowering: 0.5,
    semantic_environment_check: 3,
    surface_parse_lower: 0.4,
    source_loading: 0.2,
  });
  assert.equal(summary.dominant_stage, "semantic_environment_check");
  assert.equal(dominantMeasuredStage({ z_stage: 1, a_stage: 1 }), "a_stage");
});

test("keeps baseline stage timing unavailable and rejects partial instrumented runs", () => {
  assert.deepEqual(summarizeStageRecords([], ["baseline-1"]), { status: "unavailable" });
  assert.throws(
    () => summarizeStageRecords([], ["new-1"], { instrumentationRequired: true }),
    /missing timing records/,
  );
  assert.throws(
    () => summarizeStageRecords([timing("http2_core", "new-1", "source_loading", 0.1)], ["new-1", "new-2"]),
    /missing timing records/,
  );
  assert.throws(
    () => summarizeStageRecords([timing("http2_core", "new-1", "source_loading", 0.1)], ["new-1"]),
    /missing timing stage/,
  );
});

test("rejects stage totals that exceed the measured wall time", () => {
  const stageTiming = summarizeStageRecords(
    [
      timing("http2_connection", "new-1", "source_loading", 0.1),
      timing("http2_connection", "new-1", "surface_parse_lower", 0.2),
      timing("http2_connection", "new-1", "semantic_environment_check", 0.3),
      timing("http2_connection", "new-1", "reachable_entry_lowering", 4),
      timing("http2_connection", "new-1", "backend_runtime_remainder", 0.4),
    ],
    ["new-1"],
  );

  assert.throws(
    () => validateStageSummaryFitsRuns(stageTiming, [{ wall_time_seconds: 3.9, user_cpu_seconds: 5 }], "new"),
    /exceeds measured wall time/,
  );
});

test("validates checked benchmark result structure", () => {
  const stageTiming = summarizeStageRecords(
    [
      timing("http2_core", "new-1", "source_loading", 0.1),
      timing("http2_core", "new-1", "surface_parse_lower", 0.2),
      timing("http2_core", "new-1", "semantic_environment_check", 0.3),
      timing("http2_core", "new-1", "reachable_entry_lowering", 0.4),
      timing("http2_core", "new-1", "backend_runtime_remainder", 0.5),
    ],
    ["new-1"],
  );

  assert.doesNotThrow(() =>
    validateBenchmarkResult({
      workloads: [
        {
          baseline: {
            runs: [{ wall_time_seconds: 2, user_cpu_seconds: 1 }],
            stage_timing: { status: "unavailable" },
          },
          new: {
            runs: [{ wall_time_seconds: 1.5, user_cpu_seconds: 1 }],
            stage_timing: stageTiming,
          },
        },
      ],
    }),
  );
});

test("checked stage benchmark record includes replay metadata", () => {
  const record = JSON.parse(readFileSync("docs/reviews/toolchain-analysis-stage-benchmark.json", "utf8"));

  assert.deepEqual(record.command.slice(0, 4), [
    "benchmark-toolchain-analysis",
    "compare",
    "target/debug/veln-before-stage-timing",
    "target/debug/veln",
  ]);
  assert.ok(record.command.includes("--baseline-label"));
  assert.ok(record.command.includes("<baseline-debug-veln>"));

  const connection = record.workloads.find((workload) => workload.id === "http2_connection");
  assert.deepEqual(
    connection.env,
    DEFAULT_WORKLOADS.find((workload) => workload.id === "http2_connection").env,
  );
});

function workload(id, baselineWall, newWall, newUser, functionalOutputsEqual, options = {}) {
  return {
    id,
    baseline: {
      summary: {
        median_wall_time_seconds: baselineWall,
        median_user_cpu_seconds: 1,
        wall_time_noisy: options.baselineNoisy ?? false,
      },
    },
    new: {
      summary: {
        median_wall_time_seconds: newWall,
        median_user_cpu_seconds: newUser,
        wall_time_noisy: options.newNoisy ?? false,
      },
    },
    functional_outputs_equal: functionalOutputsEqual,
  };
}

function timing(workloadId, run, stage, durationSeconds) {
  return {
    workload: workloadId,
    run,
    stage,
    boundary: stage,
    duration_seconds: durationSeconds,
  };
}
