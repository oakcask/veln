import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const runner = fileURLToPath(new URL("./run-nextest.sh", import.meta.url));
const testArgs = ["--profile", "ci", "--filterset", "not (binary(=toolchain_harness) & test(generated_toolchain_cases::))", "--partition", "count:2/3"];

function runSuite(args, status = 0) {
  const root = mkdtempSync(join(tmpdir(), "veln-nextest-scope-"));
  try {
    mkdirSync(join(root, "scripts"));
    writeFileSync(join(root, "scripts", "ci-run"), `printf '%s\\0' "$@"\nexit ${status}\n`);
    return spawnSync("bash", [runner, ...args], { cwd: root, encoding: "utf8" });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

for (const [suite, scope] of [
  ["workspace", ["--workspace", "--exclude", "veln-repo-*"]],
  ["toolchain", ["-p", "veln-cli", "--test", "toolchain_harness"]],
  ["repository", ["-p", "veln-repo-*"]],
]) {
  test(`${suite} selects Cargo targets before applying nextest filters and partitions`, () => {
    const result = runSuite([suite, ...testArgs]);
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(result.stdout.split("\0").slice(0, -1), [
      "cargo", "llvm-cov", "--no-report", "nextest", "--locked", ...scope, ...testArgs,
    ]);
  });
}

test("rejects missing or unknown suites without falling back to a workspace build", () => {
  for (const args of [[], ["typo", ...testArgs]]) {
    const result = runSuite(args);
    assert.equal(result.status, 64);
    assert.equal(result.stdout, "");
    assert.match(result.stderr, /Select workspace, toolchain, or repository/);
  }
});

test("propagates test failures to the job", () => {
  const result = runSuite(["toolchain", ...testArgs], 42);
  assert.equal(result.status, 42);
});
