import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  compareFunctionalOutputs,
  generateAnnotatedModuleGraph,
  median,
  medianAbsoluteDeviation,
  normalizeFunctionalOutput,
  stableJson,
  summarizeRuns,
  thresholdDecisions,
} from "./benchmark-toolchain-analysis.mjs";

test("generates adjacent fully annotated module graph workloads", () => {
  const root = mkdtempSync(join(tmpdir(), "veln-benchmark-generation-"));
  const generated = generateAnnotatedModuleGraph(root, 4);

  assert.deepEqual(generated, {
    size: 4,
    moduleCount: 5,
    command: ["check", "--json", "main.veln"],
  });
  assert.match(readFileSync(join(root, "main.veln"), "utf8"), /use generated_3/);
  assert.match(
    readFileSync(join(root, "generated_3.veln"), "utf8"),
    /pub fn value_3\(\) -> Int\n\tlet seed: Int = 4\n\tseed \+ generated_2::value_2\(\)/,
  );
});

test("calculates median and median absolute deviation", () => {
  assert.equal(median([9, 1, 5]), 5);
  assert.equal(median([10, 2, 8, 4]), 6);
  assert.equal(medianAbsoluteDeviation([10, 12, 14, 40, 16]), 2);
});

test("normalizes functional output before comparison", () => {
  const left = {
    exit_status: 1,
    normalized_stdout: normalizeFunctionalOutput("ok\r\n/tmp/run\n", { "/tmp/run": "<workload>" }),
    normalized_stderr: "",
  };
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
});

test("writes deterministic machine-readable JSON", () => {
  assert.equal(
    stableJson({ z: 1, nested: { b: 2, a: 1 }, list: [{ d: 4, c: 3 }] }),
    '{\n  "list": [\n    {\n      "c": 3,\n      "d": 4\n    }\n  ],\n  "nested": {\n    "a": 1,\n    "b": 2\n  },\n  "z": 1\n}\n',
  );
});

function workload(id, baselineWall, newWall, newUser, functionalOutputsEqual) {
  return {
    id,
    baseline: {
      summary: {
        median_wall_time_seconds: baselineWall,
        median_user_cpu_seconds: 1,
      },
    },
    new: {
      summary: {
        median_wall_time_seconds: newWall,
        median_user_cpu_seconds: newUser,
      },
    },
    functional_outputs_equal: functionalOutputsEqual,
  };
}
