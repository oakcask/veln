import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { workflowPathInventory } from "./workflow-path-inventory.mjs";

function repository(context) {
  const root = mkdtempSync(join(tmpdir(), "workflow-path-inventory-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  execFileSync("git", ["init", "--quiet", root]);
  return root;
}

test("inventory contains existing tracked files, including names requiring JSON escaping", (context) => {
  const root = repository(context);
  const paths = ["present.rs", 'quote".rs', "line\nbreak.rs", "removed.rs", "directory"];
  for (const path of paths) writeFileSync(join(root, path), "");
  execFileSync("git", ["add", "--all"], { cwd: root });
  rmSync(join(root, "removed.rs"));
  rmSync(join(root, "directory"));
  mkdirSync(join(root, "directory"));
  writeFileSync(join(root, "untracked.rs"), "");
  assert.deepEqual(workflowPathInventory(root), {
    workflow_repository_files: { "present.rs": true, 'quote".rs': true, "line\nbreak.rs": true },
  });
});

test("CLI writes a JSON inventory relative to its checkout even from another working directory", (context) => {
  const root = repository(context);
  mkdirSync(join(root, "workflow-scripts"));
  const script = join(root, "workflow-scripts/workflow-path-inventory.mjs");
  copyFileSync(new URL("./workflow-path-inventory.mjs", import.meta.url), script);
  writeFileSync(join(root, "input.txt"), "");
  execFileSync("git", ["add", "--all"], { cwd: root });
  const output = join(root, "inventory.json");
  const result = spawnSync(process.execPath, [script, output], {
    cwd: tmpdir(), encoding: "utf8", timeout: 5000,
  });
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(readFileSync(output, "utf8")), {
    workflow_repository_files: { "input.txt": true, "workflow-scripts/workflow-path-inventory.mjs": true },
  });
});

test("CLI fails when no inventory destination is supplied", () => {
  const result = spawnSync(process.execPath, [new URL("./workflow-path-inventory.mjs", import.meta.url).pathname], {
    encoding: "utf8", timeout: 5000,
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /usage:.*<output.json>/);
});
