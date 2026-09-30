import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { readNextestElapsedSeconds, planShards } from "./plan-nextest-shards.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const rustBinary = process.env.WORKFLOW_SCRIPTS_RUST_BIN
  ?? path.join(repoRoot, ".cache", "veln-repo-workflow-scripts", "veln-repo-workflow-scripts");

if (process.argv[2] === "--js-plan") {
  const elapsedSeconds = readNextestElapsedSeconds(process.argv[3]);
  const plan = planShards(elapsedSeconds, {
    targetSeconds: 90,
    fallbackShards: 4,
    maxShards: 16,
  });
  process.stdout.write(JSON.stringify(plan));
} else {
  main();
}

function main() {
  if (!fs.existsSync(rustBinary)) {
    throw new Error(
      `Build the cached Rust workflow-script binary before benchmarking: cargo build --locked --release -p veln-repo-workflow-scripts`,
    );
  }

  const fixtureRoot = fs.mkdtempSync(path.join(os.tmpdir(), "veln-workflow-script-benchmark-"));
  try {
    const repetitions = positiveInteger(process.env.WORKFLOW_SCRIPT_BENCH_REPETITIONS ?? "5");
    const sizes = (process.env.WORKFLOW_SCRIPT_BENCH_SIZES ?? "1,16,64,256,1024")
      .split(",")
      .map(positiveInteger);
    const benchmarks = [
      benchmarkMemoryGuards({ fixtureRoot, repetitions, sizes }),
      benchmarkShardPlanning({ fixtureRoot, repetitions, sizes }),
      benchmarkDuplicationSummary({ fixtureRoot, repetitions, sizes }),
    ];
    const summary = renderSummary({ benchmarks, repetitions });
    if (process.env.GITHUB_STEP_SUMMARY) {
      fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary);
    }
    process.stdout.write(summary);
  } finally {
    fs.rmSync(fixtureRoot, { recursive: true, force: true });
  }
}

function benchmarkMemoryGuards({ fixtureRoot, repetitions, sizes }) {
  const root = path.join(fixtureRoot, "memory-guards");
  fs.mkdirSync(root);
  return benchmarkSizes({
    name: "workflow YAML scan",
    unit: "files",
    sizes,
    repetitions,
    prepare(size) {
      growFiles(root, size, (index) => `steps:\n  - name: command ${index}\n    run: cargo check --workspace\n`);
    },
    javascript: () => command("node", [
      "workflow-scripts/check-rust-test-memory-guards.mjs",
      root,
    ]),
    rust: () => command(rustBinary, ["check-memory-guards", root]),
    rustSerial: () => command(
      rustBinary,
      ["check-memory-guards", root],
      { VELN_WORKFLOW_SCRIPT_THREADS: "1" },
    ),
    compare(left, right) {
      assert.equal(right.stdout, left.stdout);
      assert.equal(right.stderr, left.stderr);
    },
  });
}

function benchmarkShardPlanning({ fixtureRoot, repetitions, sizes }) {
  const root = path.join(fixtureRoot, "nextest-reports");
  fs.mkdirSync(root);
  return benchmarkSizes({
    name: "nextest report scan",
    unit: "reports",
    sizes,
    repetitions,
    prepare(size) {
      growFiles(root, size, (index) => `<testsuites name="suite-${index}" time="1.25"></testsuites>\n`, "junit.xml");
    },
    javascript: () => command("node", [
      "workflow-scripts/benchmark-rust-workflow-scripts.mjs",
      "--js-plan",
      root,
    ]),
    rust: () => command(rustBinary, ["plan-shards", root, "90", "4", "16"]),
    compare(left, right) {
      assert.deepEqual(JSON.parse(right.stdout), JSON.parse(left.stdout));
    },
  });
}

function benchmarkDuplicationSummary({ fixtureRoot, repetitions, sizes }) {
  const reportPath = path.join(fixtureRoot, "jscpd-report.json");
  return benchmarkSizes({
    name: "duplication JSON summary",
    unit: "clone pairs",
    sizes,
    repetitions,
    prepare(size) {
      const duplicates = Array.from({ length: size }, (_, index) => ({
        lines: 5 + index % 40,
        tokens: 50 + index % 200,
        firstFile: { name: `crates/sample/src/first-${index}.rs`, start: index + 1 },
        secondFile: { name: `crates/sample/src/second-${index}.rs`, start: index + 2 },
      }));
      fs.writeFileSync(reportPath, JSON.stringify({
        statistics: { total: {
          sources: size * 2,
          lines: size * 20,
          clones: size,
          duplicatedLines: size * 5,
          percentage: 25,
        } },
        duplicates,
      }));
    },
    javascript: () => command("node", ["workflow-scripts/summarize-code-duplication.mjs", reportPath]),
    rust: () => command(rustBinary, ["summarize-duplication", reportPath]),
    compare(left, right) {
      assert.equal(right.stdout, left.stdout);
    },
  });
}

function benchmarkSizes({ name, unit, sizes, repetitions, prepare, javascript, rust, rustSerial, compare }) {
  const rows = [];
  for (const size of sizes) {
    prepare(size);
    compare(javascript(), rust());
    javascript();
    rust();
    const javascriptSamples = [];
    const rustSamples = [];
    const rustSerialSamples = [];
    for (let index = 0; index < repetitions; index += 1) {
      const runs = [[javascript, javascriptSamples], [rust, rustSamples]];
      if (rustSerial) {
        runs.push([rustSerial, rustSerialSamples]);
      }
      const order = index % 2 === 0 ? runs : runs.toReversed();
      for (const [run, samples] of order) {
        const start = process.hrtime.bigint();
        run();
        samples.push(Number(process.hrtime.bigint() - start) / 1_000_000);
      }
    }
    rows.push({
      size,
      javascriptMs: median(javascriptSamples),
      rustMs: median(rustSamples),
      rustSerialMs: rustSerial ? median(rustSerialSamples) : undefined,
    });
  }
  return { name, unit, rows };
}

function command(executable, args, extraEnv = {}) {
  const result = spawnSync(executable, args, {
    cwd: repoRoot,
    encoding: "utf8",
    env: { ...process.env, GITHUB_STEP_SUMMARY: "", ...extraEnv },
  });
  if (result.error || result.status !== 0) {
    throw new Error(
      `${executable} ${args.join(" ")} failed: ${result.error?.message ?? result.stderr}`,
    );
  }
  return result;
}

function growFiles(root, size, contents, leafName) {
  const current = leafName === undefined
    ? fs.readdirSync(root).length
    : fs.readdirSync(root).length;
  for (let index = current; index < size; index += 1) {
    if (leafName === undefined) {
      fs.writeFileSync(path.join(root, `workflow-${index}.yaml`), contents(index));
    } else {
      const directory = path.join(root, `report-${index}`);
      fs.mkdirSync(directory);
      fs.writeFileSync(path.join(directory, leafName), contents(index));
    }
  }
}

function renderSummary({ benchmarks, repetitions }) {
  const lines = [
    "## Cached Rust workflow-script benchmark",
    "",
    `Median of ${repetitions} process executions after one warm-up; Rust compilation is excluded to model an exact build-cache hit. Lower is better.`,
    "",
    `Runner concurrency: ${os.availableParallelism()}; Node: ${process.version}`,
    "",
  ];
  for (const benchmark of benchmarks) {
    const crossover = benchmark.rows.find((row) => row.rustMs < row.javascriptMs);
    const parallelCrossover = benchmark.rows.find(
      (row) => row.rustSerialMs !== undefined && row.rustMs < row.rustSerialMs * 0.95,
    );
    const hasSerial = benchmark.rows.some((row) => row.rustSerialMs !== undefined);
    lines.push(
      `### ${benchmark.name}`,
      "",
      `First measured Rust win: ${crossover ? `${crossover.size} ${benchmark.unit}` : "not observed"}.`,
      ...(hasSerial ? [
        `First measured auto-parallel win of at least 5% over one Rust worker: ${parallelCrossover ? `${parallelCrossover.size} ${benchmark.unit}` : "not observed"}.`,
      ] : []),
      "",
      hasSerial
        ? `| ${benchmark.unit} | JavaScript (ms) | Rust, 1 worker (ms) | Rust, auto (ms) | Rust auto / JS |`
        : `| ${benchmark.unit} | JavaScript (ms) | Rust (ms) | Rust / JS |`,
      hasSerial ? "| ---: | ---: | ---: | ---: | ---: |" : "| ---: | ---: | ---: | ---: |",
    );
    for (const row of benchmark.rows) {
      lines.push(hasSerial
        ? `| ${row.size} | ${row.javascriptMs.toFixed(2)} | ${row.rustSerialMs.toFixed(2)} | ${row.rustMs.toFixed(2)} | ${(row.rustMs / row.javascriptMs).toFixed(2)} |`
        : `| ${row.size} | ${row.javascriptMs.toFixed(2)} | ${row.rustMs.toFixed(2)} | ${(row.rustMs / row.javascriptMs).toFixed(2)} |`);
    }
    lines.push("");
  }
  lines.push(
    "Interpretation: the YAML scan can use multiple workers after discovery; the other two cases isolate native startup and parsing without parallel work. Reconsider a Rust port when its cached binary is unavailable or the script depends primarily on GitHub/Node APIs rather than local computation.",
    "",
  );
  return `${lines.join("\n")}\n`;
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

function positiveInteger(value) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 1) {
    throw new Error(`Expected a positive integer benchmark setting, received ${JSON.stringify(value)}.`);
  }
  return parsed;
}
