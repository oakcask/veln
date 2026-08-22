import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

function parsePositiveInteger(value, name) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 1) {
    throw new Error(`Set ${name} to a positive integer so shard planning has a valid bound.`);
  }
  return parsed;
}

function collectFiles(root) {
  if (!fs.existsSync(root)) {
    return [];
  }

  const files = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectFiles(entryPath));
    } else if (entry.isFile() && entry.name === "junit.xml") {
      files.push(entryPath);
    }
  }
  return files;
}

export function readNextestElapsedSeconds(root) {
  const elapsedSeconds = [];
  for (const report of collectFiles(root)) {
    const xml = fs.readFileSync(report, "utf8");
    const openingTag = xml.match(/<testsuites\b[^>]*>/u)?.[0];
    const value = openingTag?.match(/\btime="([^"]+)"/u)?.[1];
    const elapsed = Number(value);
    if (Number.isFinite(elapsed) && elapsed >= 0) {
      elapsedSeconds.push(elapsed);
    }
  }
  return elapsedSeconds;
}

export function planShards(elapsedSeconds, { targetSeconds, fallbackShards, maxShards }) {
  if (elapsedSeconds.length === 0) {
    const shardCount = Math.min(fallbackShards, maxShards);
    return { shardCount, measuredSeconds: null, requiredShards: null };
  }

  const measuredSeconds = elapsedSeconds.reduce((total, elapsed) => total + elapsed, 0);
  const requiredShards = Math.max(1, Math.ceil(measuredSeconds / targetSeconds));
  return {
    shardCount: Math.min(requiredShards, maxShards),
    measuredSeconds,
    requiredShards,
  };
}

function appendOutput(file, name, value) {
  fs.appendFileSync(file, `${name}=${value}\n`);
}

function runGh(args) {
  return spawnSync("gh", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  });
}

export function downloadPriorNextestReports({ runnerTemp, runCommand = runGh, notice = console.log }) {
  const query = runCommand([
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
  if (query.error || query.status !== 0) {
    notice(
      "::notice::Using fallback Rust test shards because workflow history could not be queried; rerun if shard planning repeatedly cannot access prior reports.",
    );
    return null;
  }

  const runId = query.stdout.trim();
  if (runId === "" || runId === "null") {
    return null;
  }

  const downloadRoot = fs.mkdtempSync(path.join(runnerTemp, "nextest-history-download-"));
  const reportRoot = path.join(runnerTemp, "nextest-history");
  try {
    const download = runCommand([
      "run",
      "download",
      runId,
      "--pattern",
      "nextest-junit-*",
      "--dir",
      downloadRoot,
    ]);
    if (download.error || download.status !== 0) {
      notice(
        "::notice::Using fallback Rust test shards because prior timing reports could not be downloaded; the next successful run will provide fresh reports.",
      );
      return null;
    }

    fs.renameSync(downloadRoot, reportRoot);
    return reportRoot;
  } finally {
    fs.rmSync(downloadRoot, { recursive: true, force: true });
  }
}

function main() {
  const targetSeconds = parsePositiveInteger(
    process.env.NEXTEST_TARGET_SECONDS,
    "NEXTEST_TARGET_SECONDS",
  );
  const fallbackShards = parsePositiveInteger(
    process.env.NEXTEST_FALLBACK_SHARDS,
    "NEXTEST_FALLBACK_SHARDS",
  );
  const maxShards = parsePositiveInteger(process.env.NEXTEST_MAX_SHARDS, "NEXTEST_MAX_SHARDS");
  if (!process.env.RUNNER_TEMP) {
    throw new Error("Run shard planning on a GitHub Actions runner so prior reports have a workspace.");
  }
  if (!process.env.GITHUB_OUTPUT) {
    throw new Error("Run shard planning as a GitHub Actions step so its matrix outputs can be published.");
  }

  const reportRoot =
    downloadPriorNextestReports({ runnerTemp: process.env.RUNNER_TEMP }) ??
    path.join(process.env.RUNNER_TEMP, "nextest-history");
  const elapsedSeconds = readNextestElapsedSeconds(reportRoot);
  const plan = planShards(elapsedSeconds, { targetSeconds, fallbackShards, maxShards });
  const shards = Array.from({ length: plan.shardCount }, (_, index) => index + 1);

  appendOutput(process.env.GITHUB_OUTPUT, "shard_count", plan.shardCount);
  appendOutput(process.env.GITHUB_OUTPUT, "shards", JSON.stringify(shards));

  let summary;
  if (plan.measuredSeconds === null) {
    summary = `No prior nextest timing was available, so this run uses ${plan.shardCount} fallback shards. A successful run will provide timings for the next plan.`;
  } else {
    summary = `Planned ${plan.shardCount} Rust test shards from ${plan.measuredSeconds.toFixed(1)} seconds of prior test time with a ${targetSeconds}-second target.`;
    if (plan.requiredShards > maxShards) {
      summary += ` The plan was capped at ${maxShards}; reduce test runtime or raise NEXTEST_MAX_SHARDS if shards keep exceeding the target.`;
    }
  }

  console.log(summary);
  if (process.env.GITHUB_STEP_SUMMARY) {
    fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${summary}\n`);
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
