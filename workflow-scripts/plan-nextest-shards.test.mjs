import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  planShards,
  readNextestElapsedSeconds,
} from "./plan-nextest-shards.mjs";

test("plans from the combined elapsed time of prior shards", () => {
  const plan = planShards([110.413, 104.123, 62.397, 105.508], {
    targetSeconds: 90,
    fallbackShards: 4,
    maxShards: 16,
  });

  assert.equal(plan.shardCount, 5);
  assert.equal(plan.requiredShards, 5);
  assert.ok(Math.abs(plan.measuredSeconds - 382.441) < 0.001);
});

test("uses the fallback when no timing is available", () => {
  assert.deepEqual(
    planShards([], { targetSeconds: 90, fallbackShards: 4, maxShards: 16 }),
    { shardCount: 4, measuredSeconds: null, requiredShards: null },
  );
});

test("caps plans without hiding the uncapped requirement", () => {
  assert.deepEqual(
    planShards([1_500], { targetSeconds: 90, fallbackShards: 4, maxShards: 16 }),
    { shardCount: 16, measuredSeconds: 1_500, requiredShards: 17 },
  );
});

test("reads only valid top-level nextest elapsed times", (context) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "nextest-plan-"));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.mkdirSync(path.join(root, "nextest-junit-1"));
  fs.mkdirSync(path.join(root, "nextest-junit-2"));
  fs.mkdirSync(path.join(root, "other"));
  fs.writeFileSync(
    path.join(root, "nextest-junit-1", "junit.xml"),
    '<testsuites tests="10" time="12.5"><testsuite time="99"/></testsuites>',
  );
  fs.writeFileSync(
    path.join(root, "nextest-junit-2", "junit.xml"),
    '<testsuites time="7.25" tests="8"></testsuites>',
  );
  fs.writeFileSync(path.join(root, "other", "junit.xml"), '<testsuites time="invalid"/>');

  assert.deepEqual(readNextestElapsedSeconds(root), [12.5, 7.25]);
});
