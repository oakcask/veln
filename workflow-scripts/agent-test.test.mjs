import assert from "node:assert/strict";
import { copyFileSync, mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

function runAgentTest(args, extraEnvironment = {}) {
  const root = mkdtempSync(join(tmpdir(), "veln-agent-test-"));
  const scripts = join(root, "scripts");
  const trace = join(root, "trace");
  mkdirSync(scripts);
  copyFileSync(join(repositoryRoot, "scripts", "agent-test"), join(scripts, "agent-test"));
  writeFileSync(
    join(scripts, "agent-run"),
    `set -eu
printf '<%s>' "$@" >> "${"${TRACE_FILE:?}"}"
printf '\n' >> "${"${TRACE_FILE:?}"}"
for argument in "$@"; do
  if [ -n "${"${FAIL_ARGUMENT:-}"}" ] && [ "$argument" = "${"${FAIL_ARGUMENT}"}" ]; then
    exit 17
  fi
done
`,
  );

  const result = spawnSync("bash", ["scripts/agent-test", ...args], {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, TRACE_FILE: trace, ...extraEnvironment },
  });
  const calls = readFileSync(trace, "utf8").trim().split("\n");
  return { ...result, calls };
}

test("passes a single Cargo test filter through unchanged", () => {
  const result = runAgentTest(["-p", "veln-cli", "only_this_test", "--locked"]);

  assert.equal(result.status, 0);
  assert.deepEqual(result.calls, ["<cargo><test><-p><veln-cli><only_this_test><--locked>"]);
});

test("runs each Cargo test filter with the same target selection", () => {
  const result = runAgentTest([
    "-p",
    "veln-cli",
    "--test",
    "toolchain_harness",
    "first_test",
    "second_test",
  ]);

  assert.equal(result.status, 0);
  assert.deepEqual(result.calls, [
    "<cargo><test><-p><veln-cli><--test><toolchain_harness><first_test>",
    "<cargo><test><-p><veln-cli><--test><toolchain_harness><second_test>",
  ]);
});

test("forwards test-binary arguments to every selected filter", () => {
  const result = runAgentTest(["first_test", "second_test", "--", "--exact", "--nocapture"]);

  assert.equal(result.status, 0);
  assert.deepEqual(result.calls, [
    "<cargo><test><first_test><--><--exact><--nocapture>",
    "<cargo><test><second_test><--><--exact><--nocapture>",
  ]);
});

test("stops after the first failing selected filter", () => {
  const result = runAgentTest(["first_test", "second_test", "third_test"], {
    FAIL_ARGUMENT: "second_test",
  });

  assert.equal(result.status, 17);
  assert.deepEqual(result.calls, [
    "<cargo><test><first_test>",
    "<cargo><test><second_test>",
  ]);
});

test("keeps the guarded workspace default when no arguments are given", () => {
  const result = runAgentTest([]);

  assert.equal(result.status, 0);
  assert.deepEqual(result.calls, ["<cargo><test><--locked><--workspace>"]);
});
