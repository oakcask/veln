import assert from "node:assert/strict";
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { githubYamlExtensionViolations } from "./check-github-yaml-extensions.mjs";

function repository(context, paths) {
  const root = mkdtempSync(join(tmpdir(), "github-yaml-extensions-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  for (const directory of [".github/workflows", ".github/actions"]) {
    mkdirSync(join(root, directory), { recursive: true });
  }
  for (const path of paths) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), "name: example\n");
  }
  return root;
}

test("accepts .yaml workflows and actions without restricting other repository files", (context) => {
  const root = repository(context, [
    ".github/workflows/test.yaml",
    ".github/actions/setup/action.yaml",
    ".github/actions/setup/README.md",
    ".github/actions/setup/run.sh",
    "fixtures/input.yml",
  ]);
  assert.deepEqual(githubYamlExtensionViolations(root), []);
});

test("rejects .yml workflows and nested action manifests even alongside .yaml files", (context) => {
  const root = repository(context, [
    ".github/workflows/test.yaml",
    ".github/workflows/test.yml",
    ".github/actions/group/setup/action.yml",
    ".github/actions/group/setup/action.yaml",
  ]);
  assert.deepEqual(githubYamlExtensionViolations(root), [
    ".github/actions/group/setup/action.yml",
    ".github/workflows/test.yml",
  ]);
});

test("rejects YAML extension case variants", (context) => {
  const root = repository(context, [".github/workflows/test.YAML"]);
  assert.deepEqual(githubYamlExtensionViolations(root), [".github/workflows/test.YAML"]);
});

test("CLI fails with the offending path and repair guidance", (context) => {
  const root = repository(context, [".github/actions/setup/action.yml"]);
  const script = join(root, "workflow-scripts/check-github-yaml-extensions.mjs");
  mkdirSync(dirname(script), { recursive: true });
  copyFileSync(new URL("./check-github-yaml-extensions.mjs", import.meta.url), script);
  const result = spawnSync(process.execPath, [script], { encoding: "utf8", timeout: 5000 });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Rename.*\.yaml/);
  assert.match(result.stderr, /update their references/);
  assert.match(result.stderr, /\.github\/actions\/setup\/action\.yml/);
});
