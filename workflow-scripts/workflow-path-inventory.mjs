import { execFileSync } from "node:child_process";
import { statSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function workflowPathInventory(root) {
  const tracked = execFileSync("git", ["ls-files", "-z"], { cwd: root, encoding: "utf8" });
  const files = tracked.split("\0").filter((path) => {
    if (!path) return false;
    try {
      return statSync(join(root, path)).isFile();
    } catch (error) {
      if (error.code === "ENOENT" || error.code === "ENOTDIR") return false;
      throw error;
    }
  });
  return { workflow_repository_files: Object.fromEntries(files.map((path) => [path, true])) };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const output = process.argv[2];
  if (!output) throw new Error("usage: node workflow-scripts/workflow-path-inventory.mjs <output.json>");
  const root = fileURLToPath(new URL("../", import.meta.url));
  writeFileSync(output, `${JSON.stringify(workflowPathInventory(root))}\n`);
}
