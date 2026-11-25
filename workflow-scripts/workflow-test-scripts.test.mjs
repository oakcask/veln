import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(
  new URL("../.github/workflows/workflow--test-scripts.yaml", import.meta.url),
  "utf8",
);

test("keeps Rust workspace tests in the Rust workflow", () => {
  assert.doesNotMatch(workflow, /veln-repo-workflow-scripts/);
  assert.doesNotMatch(workflow, /^\s+uses: \.\/\.github\/actions\/cargo-dependencies$/m);
  assert.doesNotMatch(workflow, /^\s+run: .*\bcargo\s+test\b/m);
  assert.doesNotMatch(workflow, /^\s+- (?:Cargo\.(?:lock|toml)|rust-toolchain\.toml)$/m);
});
