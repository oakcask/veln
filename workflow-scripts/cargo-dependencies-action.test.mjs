import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const action = readFileSync(
  new URL("../.github/actions/cargo-dependencies/action.yaml", import.meta.url),
  "utf8",
);

test("dependency cache seeding skips fetch and save after an exact restore", () => {
  const conditionFor = (stepName) => {
    const match = action.match(new RegExp(`- name: ${stepName}\\n      if: (.+)`));
    assert.ok(match, `missing condition for ${stepName}`);
    return match[1];
  };

  assert.equal(conditionFor("Fetch locked Cargo dependencies"), conditionFor("Save Cargo dependency cache"));
});
