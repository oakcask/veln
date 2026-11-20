import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { median, range, stable, validateArtifact } from "./benchmark-lightweight-tasks.mjs";

test("summarizes deterministic benchmark samples", () => {
  assert.equal(median([9, 1, 5, 3, 7]), 5);
  assert.equal(range([9, 1, 5]), 8);
  assert.equal(stable([100, 105, 110]), true);
  assert.equal(stable([100, 105, 130]), false);
  assert.equal(stable([0, 0, 0]), true);
  assert.equal(stable([0, 0, 1]), false);
});

test("validates the checked adoption artifact against its workload source", () => {
  const harness = readFileSync(resolve(
    "tools/veln-repo-lightweight-task-benchmark/LightweightTaskBenchmark.java",
  ));
  const digest = createHash("sha256").update(harness).digest("hex");
  const artifact = JSON.parse(readFileSync(resolve(
    "benchmarks/lightweight-tasks/result.json",
  ), "utf8"));
  assert.doesNotThrow(() => validateArtifact(artifact, digest));
});
