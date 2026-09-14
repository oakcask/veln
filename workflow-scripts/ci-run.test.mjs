import assert from "node:assert/strict";
import { copyFileSync, mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

function runCiCommand(args, extraEnvironment = {}) {
  const root = mkdtempSync(join(tmpdir(), "veln-ci-run-"));
  const scripts = join(root, "scripts");
  mkdirSync(scripts);
  copyFileSync(join(repositoryRoot, "scripts", "ci-run"), join(scripts, "ci-run"));
  writeFileSync(
    join(scripts, "agent-run"),
    `set -eu
printf '%s\n' "${"${CARGO_BUILD_JOBS-unset}"}"
printf '%s\n' "${"${CARGO_INCREMENTAL-unset}"}"
printf '%s\n' "${"${VELN_AGENT_TIMEOUT:?}"}"
printf '%s\n' "${"${VELN_AGENT_MEMORY_MB:?}"}"
printf '%s\n' "${"${VELN_AGENT_JAVA_OPTIONS:?}"}"
printf '<%s>' "$@"
printf '\n'
`,
  );

  return spawnSync("bash", ["scripts/ci-run", ...args], {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, ...extraEnvironment },
  });
}

test("preserves Cargo settings while fixing CI resource limits", () => {
  const result = runCiCommand(["cargo", "test", "--workspace"], {
    CARGO_BUILD_JOBS: "99",
    CARGO_INCREMENTAL: "1",
    VELN_AGENT_TIMEOUT: "99h",
    VELN_AGENT_JAVA_OPTIONS: "-Xmx99g",
    VELN_AGENT_MEMORY_MB: "99999",
  });

  assert.equal(result.status, 0);
  assert.equal(
    result.stdout,
    "99\n1\n540s\n6144\n-XX:+PerfDisableSharedMem -Xmx256m -XX:ActiveProcessorCount=2\n<cargo><test><--workspace>\n",
  );
});

test("rejects a missing command", () => {
  const result = runCiCommand([]);

  assert.equal(result.status, 64);
  assert.match(result.stderr, /usage: bash scripts\/ci-run/);
});
