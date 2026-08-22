import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  downloadPriorNextestReports,
  planShards,
  readNextestElapsedSeconds,
} from "./plan-nextest-shards.mjs";

function temporaryDirectory(context) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "nextest-plan-"));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return root;
}

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
  const root = temporaryDirectory(context);
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

test("downloads reports from the latest successful Rust test run", (context) => {
  const runnerTemp = temporaryDirectory(context);
  const calls = [];
  const reportRoot = downloadPriorNextestReports({
    runnerTemp,
    runCommand(args) {
      calls.push(args);
      if (args[1] === "list") {
        return { status: 0, stdout: "12345\n" };
      }

      const downloadRoot = args.at(-1);
      fs.mkdirSync(path.join(downloadRoot, "nextest-junit-1"));
      fs.writeFileSync(path.join(downloadRoot, "nextest-junit-1", "junit.xml"), "<testsuites/>");
      return { status: 0, stdout: "" };
    },
  });

  assert.deepEqual(calls[0], [
    "run",
    "list",
    "--workflow",
    "test--rust.yaml",
    "--status",
    "success",
    "--limit",
    "1",
    "--json",
    "databaseId",
    "--jq",
    ".[0].databaseId",
  ]);
  assert.deepEqual(calls[1].slice(0, -1), [
    "run",
    "download",
    "12345",
    "--pattern",
    "nextest-junit-*",
    "--dir",
  ]);
  assert.equal(reportRoot, path.join(runnerTemp, "nextest-history"));
  assert.equal(fs.existsSync(path.join(reportRoot, "nextest-junit-1", "junit.xml")), true);
});

test("uses the fallback when workflow history cannot be queried", (context) => {
  const runnerTemp = temporaryDirectory(context);
  const notices = [];
  const reportRoot = downloadPriorNextestReports({
    runnerTemp,
    runCommand: () => ({ status: 1, stdout: "" }),
    notice: (message) => notices.push(message),
  });

  assert.equal(reportRoot, null);
  assert.deepEqual(notices, [
    "::notice::Using fallback Rust test shards because workflow history could not be queried; rerun if shard planning repeatedly cannot access prior reports.",
  ]);
});

test("uses the fallback when no successful Rust test run exists", (context) => {
  const runnerTemp = temporaryDirectory(context);
  let callCount = 0;
  const reportRoot = downloadPriorNextestReports({
    runnerTemp,
    runCommand: () => {
      callCount += 1;
      return { status: 0, stdout: "\n" };
    },
  });

  assert.equal(reportRoot, null);
  assert.equal(callCount, 1);
});

test("uses the fallback and removes partial reports when download fails", (context) => {
  const runnerTemp = temporaryDirectory(context);
  const notices = [];
  let downloadRoot;
  const reportRoot = downloadPriorNextestReports({
    runnerTemp,
    runCommand(args) {
      if (args[1] === "list") {
        return { status: 0, stdout: "12345\n" };
      }
      downloadRoot = args.at(-1);
      fs.writeFileSync(path.join(downloadRoot, "partial-report"), "incomplete");
      return { status: 1, stdout: "" };
    },
    notice: (message) => notices.push(message),
  });

  assert.equal(reportRoot, null);
  assert.equal(fs.existsSync(downloadRoot), false);
  assert.deepEqual(notices, [
    "::notice::Using fallback Rust test shards because prior timing reports could not be downloaded; the next successful run will provide fresh reports.",
  ]);
});
