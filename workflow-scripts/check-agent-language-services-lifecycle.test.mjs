import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  parseMarkdownSource,
  runCommand,
  validateAuthorityShape,
  validateContent,
} from "./check-agent-language-services-lifecycle.mjs";

test("accepts the committed reviewed authority", () => {
  const result = runCommand({ repoRoot: ".", command: "validate" });

  assert.equal(result.valid, true, result.errors.join("\n"));
});

test("accepts the committed frozen inventory and ledger fixture corpus", () => {
  const result = runCommand({ repoRoot: ".", command: "validate" });
  const invalidFixtures = fs.readdirSync("docs/reference/agent-language-services-lifecycle/ledger-fixtures/invalid").sort();

  assert.equal(result.valid, true, result.errors.join("\n"));
  assert.deepEqual(invalidFixtures, [
    "catch_all_leaf.json",
    "direct_parent_duplicate_guard.json",
    "duplicate_leaf.json",
    "invalid_destination_shape.json",
    "invalid_removed_conformance.json",
    "lifecycle_mismatch.json",
    "missing_leaf.json",
    "parent_mapping.json",
    "range_leaf.json",
    "unknown_leaf.json",
    "wildcard_leaf.json",
  ]);
});

test("rejects frozen source drift and inventory root omissions or duplicates", () => {
  using fixture = frozenFixture("als-frozen-roots");
  const universe = readJsonFile(fixture.path, "docs/reference/agent-language-services-lifecycle/source-universe.json");
  universe.roots[0].digest = "0".repeat(64);
  writeJsonFile(fixture.path, "docs/reference/agent-language-services-lifecycle/source-universe.json", universe);
  assert.match(runCommand({ repoRoot: fixture.path, command: "validate" }).errors.join("\n"), /digest changed/);

  using missing = frozenFixture("als-frozen-missing-root");
  const missingInventory = readJsonFile(missing.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json");
  missingInventory.roots.shift();
  writeJsonFile(missing.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json", missingInventory);
  assert.match(runCommand({ repoRoot: missing.path, command: "validate" }).errors.join("\n"), /missing inventory root|expected .* roots/);

  using duplicate = frozenFixture("als-frozen-duplicate-root");
  const duplicateInventory = readJsonFile(duplicate.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json");
  duplicateInventory.roots[1] = structuredClone(duplicateInventory.roots[0]);
  writeJsonFile(duplicate.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json", duplicateInventory);
  assert.match(runCommand({ repoRoot: duplicate.path, command: "validate" }).errors.join("\n"), /duplicate inventory root/);
});

test("rejects missing children, bad child spans, and wrong frozen lifecycle", () => {
  using missingChild = frozenFixture("als-frozen-missing-child");
  const missingChildInventory = readJsonFile(missingChild.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json");
  missingChildInventory.roots[0].children = [];
  writeJsonFile(missingChild.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json", missingChildInventory);
  assert.match(runCommand({ repoRoot: missingChild.path, command: "validate" }).errors.join("\n"), /children must match reviewed leaves|uncovered lifecycle statement/);

  using gap = frozenFixture("als-frozen-gap");
  const gapInventory = readJsonFile(gap.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json");
  gapInventory.roots[0].children[0].spans = [[1, 2]];
  writeJsonFile(gap.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json", gapInventory);
  assert.match(runCommand({ repoRoot: gap.path, command: "validate" }).errors.join("\n"), /uncovered lifecycle statement/);

  using overlap = frozenFixture("als-frozen-overlap");
  const overlapInventory = readJsonFile(overlap.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json");
  const child = structuredClone(overlapInventory.roots[0].children[0]);
  child.id = `${overlapInventory.roots[0].id}-L02`;
  overlapInventory.roots[0].child_count = 2;
  overlapInventory.roots[0].children.push(child);
  writeJsonFile(overlap.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json", overlapInventory);
  assert.match(runCommand({ repoRoot: overlap.path, command: "validate" }).errors.join("\n"), /overlaps another child/);

  using outOfRange = frozenFixture("als-frozen-out-of-range");
  const outOfRangeInventory = readJsonFile(outOfRange.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json");
  outOfRangeInventory.roots[0].children[0].spans = [[0, [...outOfRangeInventory.roots[0].text].length + 1]];
  writeJsonFile(outOfRange.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json", outOfRangeInventory);
  assert.match(runCommand({ repoRoot: outOfRange.path, command: "validate" }).errors.join("\n"), /out of range/);

  using wrongLifecycle = frozenFixture("als-frozen-wrong-lifecycle");
  const wrongInventory = readJsonFile(wrongLifecycle.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json");
  wrongInventory.roots[0].children[0].lifecycle = wrongInventory.roots[0].children[0].lifecycle === "planned" ? "current" : "planned";
  writeJsonFile(wrongLifecycle.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json", wrongInventory);
  assert.match(runCommand({ repoRoot: wrongLifecycle.path, command: "validate" }).errors.join("\n"), /lifecycle differs from reviewed authority/);
});

test("rejects required migration ledger schema fixture mutations", () => {
  using fixture = frozenFixture("als-ledger-invalid");
  const invalidDirectory = path.join(fixture.path, "docs/reference/agent-language-services-lifecycle/ledger-fixtures/invalid");
  for (const file of fs.readdirSync(invalidDirectory)) {
    const fixtureJson = readJsonFile(fixture.path, `docs/reference/agent-language-services-lifecycle/ledger-fixtures/invalid/${file}`);
    const validLedger = readJsonFile(fixture.path, "docs/reference/agent-language-services-lifecycle/ledger-fixtures/valid-ledger.json");
    writeJsonFile(fixture.path, "docs/reference/agent-language-services-lifecycle/ledger-fixtures/valid-ledger.json", fixtureJson.ledger);
    const result = runCommand({ repoRoot: fixture.path, command: "validate" });
    assert.equal(result.valid, false, file);
    assert.match(result.errors.join("\n"), new RegExp(escapeRegExp(fixtureJson.expected_error)), file);
    writeJsonFile(fixture.path, "docs/reference/agent-language-services-lifecycle/ledger-fixtures/valid-ledger.json", validLedger);
  }
});

test("rejects changed digests and missing or duplicate inventory roots", () => {
  const authority = repositoryAuthority();
  const parsedRoots = repositoryRoots();
  const changedDigest = structuredClone(authority);
  changedDigest.roots[0].digest = "0".repeat(64);
  const missingRoot = structuredClone(authority);
  missingRoot.roots.splice(1, 1);
  const duplicateRoot = structuredClone(authority);
  duplicateRoot.roots[1] = structuredClone(duplicateRoot.roots[0]);

  assert.match(validateAuthorityShape({ authority: changedDigest, parsedRoots, sourcePath: authority.source_path }).join("\n"), /digest changed/);
  assert.match(validateAuthorityShape({ authority: missingRoot, parsedRoots, sourcePath: authority.source_path }).join("\n"), /expected .* roots|missing reviewed source root/);
  assert.match(validateAuthorityShape({ authority: duplicateRoot, parsedRoots, sourcePath: authority.source_path }).join("\n"), /duplicate root id/);
});

test("rejects missing child spans, span gaps, overlaps, and out-of-range spans", () => {
  const authority = repositoryAuthority();
  const parsedRoots = repositoryRoots();
  const missingChild = structuredClone(authority);
  missingChild.roots[0].leaves = [];
  const gap = structuredClone(authority);
  gap.roots[0].leaves[0].spans = [[1, 2]];
  const overlap = structuredClone(authority);
  const span = gapSafeSpan(authority.roots[0]);
  overlap.roots[0].leaves = [
    { id: `${authority.roots[0].id}-L01`, lifecycle: "planned", spans: [span] },
    { id: `${authority.roots[0].id}-L02`, lifecycle: "planned", spans: [span] },
  ];
  const outOfRange = structuredClone(authority);
  outOfRange.roots[0].leaves[0].spans = [[0, [...outOfRange.roots[0].text].length + 1]];

  assert.match(validateAuthorityShape({ authority: missingChild, parsedRoots, sourcePath: authority.source_path }).join("\n"), /at least one semantic leaf/);
  assert.match(validateAuthorityShape({ authority: gap, parsedRoots, sourcePath: authority.source_path }).join("\n"), /do not cover scalar/);
  assert.match(validateAuthorityShape({ authority: overlap, parsedRoots, sourcePath: authority.source_path }).join("\n"), /overlaps another semantic leaf/);
  assert.match(validateAuthorityShape({ authority: outOfRange, parsedRoots, sourcePath: authority.source_path }).join("\n"), /out of range/);
});

test("rejects wrong lifecycle, removed conformance leaves, and detached identities", () => {
  const authority = repositoryAuthority();
  const parsedRoots = repositoryRoots();
  const wrongLifecycle = structuredClone(authority);
  wrongLifecycle.roots[0].leaves[0].lifecycle = "future";
  const removedConformance = structuredClone(authority);
  const conformanceRoot = removedConformance.roots.find((root) => root.source_class === "conformance");
  conformanceRoot.leaves[0].lifecycle = "removed";
  const detachedIdentity = structuredClone(authority);
  detachedIdentity.identities[0].leaf = "ALS-R9999-L01";

  assert.match(validateAuthorityShape({ authority: wrongLifecycle, parsedRoots, sourcePath: authority.source_path }).join("\n"), /lifecycle must be/);
  assert.match(validateAuthorityShape({ authority: removedConformance, parsedRoots, sourcePath: authority.source_path }).join("\n"), /conformance leaf cannot be removed/);
  assert.match(validateAuthorityShape({ authority: detachedIdentity, parsedRoots, sourcePath: authority.source_path }).join("\n"), /is not reviewed/);
});

test("covers continuation list items and non-BMP scalar spans", () => {
  const source = [
    "---",
    "role: proposal",
    "---",
    "",
    "# Title",
    "",
    "- first line",
    "  continued 😀 line",
    "- second line",
    "",
  ].join("\n");
  const roots = parseMarkdownSource({ source });

  assert.equal(roots.length, 2);
  assert.equal(roots[0].text, "- first line\n  continued 😀 line");
  assert.equal([...roots[0].text].includes("😀"), true);
});

test("structural skeleton writer omits reviewed semantic fields and preserves authority", () => {
  using fixture = tempDirectory("als-writer");
  copyFile("docs/proposals/agent-language-services.md", path.join(fixture.path, "docs/proposals/agent-language-services.md"));
  copyFile("docs/reference/agent-language-services-lifecycle-review/source-decisions.json", path.join(fixture.path, "docs/reference/agent-language-services-lifecycle-review/source-decisions.json"));
  const before = fs.readFileSync(path.join(fixture.path, "docs/reference/agent-language-services-lifecycle-review/source-decisions.json"), "utf8");
  const result = runCommand({
    repoRoot: fixture.path,
    command: "write-structural-skeleton",
    argv: ["--output", "tmp/skeleton.json"],
  });
  const skeleton = JSON.parse(fs.readFileSync(path.join(fixture.path, "tmp/skeleton.json"), "utf8"));
  const after = fs.readFileSync(path.join(fixture.path, "docs/reference/agent-language-services-lifecycle-review/source-decisions.json"), "utf8");

  assert.equal(result.valid, true, result.errors.join("\n"));
  assert.equal(after, before);
  assert.equal("source_class" in skeleton.roots[0], false);
  assert.equal("leaves" in skeleton.roots[0], false);
});

test("rejects structural writer attempts to overwrite reviewed authority", () => {
  const result = runCommand({
    repoRoot: ".",
    command: "write-structural-skeleton",
    argv: ["--output", "docs/reference/agent-language-services-lifecycle-review/source-decisions.json"],
  });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /reviewed source decisions/);
});

test("requires every lifecycle range input", () => {
  using fixture = tempRepo("als-missing-range");
  const result = runCommand({ repoRoot: fixture.path, command: "validate-range", argv: [] });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /--base/);
  assert.match(result.errors.join("\n"), /--head/);
  assert.match(result.errors.join("\n"), /--event-base-ref/);
  assert.match(result.errors.join("\n"), /--default-ref/);
});

test("accepts an exact G0 to G1 review-gate transition", () => {
  using fixture = tempRepo("als-g0-g1");
  const base = writeG0(fixture.path);
  git(fixture.path, ["switch", "--create", "gate"]);
  writeG1(fixture.path);
  const head = commitAll(fixture.path, "complete review gate");

  const result = runCommand({
    repoRoot: fixture.path,
    command: "validate-range",
    argv: ["--base", base, "--head", head, "--event-base-ref", "main", "--default-ref", "main"],
  });

  assert.equal(result.valid, true, result.errors.join("\n"));
  assert.match(result.summary, /G0 -> G1/);
});

test("accepts sequenced G0 to G2 history with a valid G1 checkpoint", () => {
  using fixture = tempRepo("als-g0-g2-sequenced");
  const base = writeG0(fixture.path);
  git(fixture.path, ["switch", "--create", "combined"]);
  writeG1(fixture.path);
  writeFrozenDestinationFiles(fixture.path);
  const g1 = commitAll(fixture.path, "complete review gate");
  const writeResult = runCommand({ repoRoot: fixture.path, command: "write-frozen-artifacts", argv: ["--base-commit", g1] });
  assert.equal(writeResult.valid, true, writeResult.errors.join("\n"));
  writeFile(fixture.path, "docs/reference/README.md", "---\nrole: routing\nupdate-when: Routes change.\n---\n\n# Reference\n\n- [Lifecycle](agent-language-services-lifecycle/README.md).\n");
  writeFile(fixture.path, "docs/reference/implemented-proposals/README.md", "---\nrole: routing\nupdate-when: Records change.\n---\n\n# Records\n\n- [Frozen](agent-language-services-frozen-source-inventory.md).\n");
  writeFile(fixture.path, "docs/reference/implemented-proposals/agent-language-services-frozen-source-inventory.md", "---\nrole: implementation-record\nauthority: supporting\nupdate-when: The frozen inventory record is superseded.\n---\n\n# Agent Language Services Frozen Source Inventory\n");
  const head = commitAll(fixture.path, "freeze inventory");

  const result = runCommand({
    repoRoot: fixture.path,
    command: "validate-range",
    argv: ["--base", base, "--head", head, "--event-base-ref", "main", "--default-ref", "main"],
  });

  assert.equal(result.valid, true, result.errors.join("\n"));
  assert.match(result.summary, /G0 -> G2/);
});

test("rejects unsequenced G0 to G2 history and direct forbidden frozen paths", () => {
  using fixture = tempRepo("als-g0-g2");
  const base = writeG0(fixture.path);
  git(fixture.path, ["switch", "--create", "combined"]);
  writeG1(fixture.path);
  writeFile(fixture.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json", "{}\n");
  const head = commitAll(fixture.path, "combine gate and inventory");

  const result = runCommand({
    repoRoot: fixture.path,
    command: "validate-range",
    argv: ["--base", base, "--head", head, "--event-base-ref", "main", "--default-ref", "main"],
  });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /valid G1 checkpoint/);
});

test("rejects non-default or stacked event base names", () => {
  using fixture = tempRepo("als-non-default");
  const base = writeG0(fixture.path);
  git(fixture.path, ["switch", "--create", "gate"]);
  writeG1(fixture.path);
  const head = commitAll(fixture.path, "complete review gate");

  const result = runCommand({
    repoRoot: fixture.path,
    command: "validate-range",
    argv: ["--base", base, "--head", head, "--event-base-ref", "feature-base", "--default-ref", "main"],
  });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /event base/);
});

test("rejects paths outside the G0 to G1 allowlist", () => {
  using fixture = tempRepo("als-allowlist");
  const base = writeG0(fixture.path);
  git(fixture.path, ["switch", "--create", "gate"]);
  writeG1(fixture.path);
  writeFile(fixture.path, "docs/reference/unrelated.md", "---\nrole: reference\nauthority: supporting\nupdate-when: The unrelated reference changes.\n---\n\n# Unrelated\n");
  const head = commitAll(fixture.path, "complete review gate with extra path");

  const result = runCommand({
    repoRoot: fixture.path,
    command: "validate-range",
    argv: ["--base", base, "--head", head, "--event-base-ref", "main", "--default-ref", "main"],
  });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /outside the closed review-gate allowlist/);
});

test("accepts a scoped G1 to G2 frozen-inventory bootstrap transition", () => {
  using fixture = tempRepo("als-g1-g2");
  writeG0(fixture.path);
  git(fixture.path, ["switch", "--create", "gate"]);
  writeG1(fixture.path);
  writeFrozenDestinationFiles(fixture.path);
  const base = commitAll(fixture.path, "complete review gate");
  git(fixture.path, ["branch", "--force", "main", base]);
  git(fixture.path, ["switch", "--create", "inventory"]);
  const writeResult = runCommand({ repoRoot: fixture.path, command: "write-frozen-artifacts", argv: ["--base-commit", base] });
  assert.equal(writeResult.valid, true, writeResult.errors.join("\n"));
  writeFile(fixture.path, "docs/reference/README.md", "---\nrole: routing\nupdate-when: Routes change.\n---\n\n# Reference\n\n- [Lifecycle](agent-language-services-lifecycle/README.md).\n");
  writeFile(fixture.path, "docs/reference/implemented-proposals/README.md", "---\nrole: routing\nupdate-when: Records change.\n---\n\n# Records\n\n- [Frozen](agent-language-services-frozen-source-inventory.md).\n");
  writeFile(fixture.path, "docs/reference/implemented-proposals/agent-language-services-frozen-source-inventory.md", "---\nrole: implementation-record\nauthority: supporting\nupdate-when: The frozen inventory record is superseded.\n---\n\n# Agent Language Services Frozen Source Inventory\n");
  const head = commitAll(fixture.path, "freeze inventory");

  const result = runCommand({
    repoRoot: fixture.path,
    command: "validate-range",
    argv: ["--base", base, "--head", head, "--event-base-ref", "main", "--default-ref", "main"],
  });

  assert.equal(result.valid, true, result.errors.join("\n"));
  assert.match(result.summary, /G1 -> G2/);
});

test("rejects G1 to G2 bootstrap changes outside the frozen allowlist", () => {
  using fixture = tempRepo("als-g1-g2-extra");
  writeG0(fixture.path);
  git(fixture.path, ["switch", "--create", "gate"]);
  writeG1(fixture.path);
  writeFrozenDestinationFiles(fixture.path);
  const base = commitAll(fixture.path, "complete review gate");
  git(fixture.path, ["branch", "--force", "main", base]);
  git(fixture.path, ["switch", "--create", "inventory"]);
  runCommand({ repoRoot: fixture.path, command: "write-frozen-artifacts", argv: ["--base-commit", base] });
  writeFile(fixture.path, "docs/proposals/agent-language-services.md", `${sampleUmbrella()}\nExtra reorganization.\n`);
  const head = commitAll(fixture.path, "freeze inventory with source edit");

  const result = runCommand({
    repoRoot: fixture.path,
    command: "validate-range",
    argv: ["--base", base, "--head", head, "--event-base-ref", "main", "--default-ref", "main"],
  });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /outside the frozen-inventory bootstrap allowlist/);
});

test("rejects frozen artifact and workflow registration edits after G2", () => {
  using frozenEdit = g2Fixture("als-g2-frozen-edit");
  git(frozenEdit.path, ["switch", "--create", "docs-change"]);
  const inventory = readJsonFile(frozenEdit.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json");
  inventory.roots[0].digest = "0".repeat(64);
  writeJsonFile(frozenEdit.path, "docs/reference/agent-language-services-lifecycle/frozen-inventory.json", inventory);
  const frozenHead = commitAll(frozenEdit.path, "edit frozen inventory");
  const frozenResult = runCommand({
    repoRoot: frozenEdit.path,
    command: "validate-range",
    argv: ["--base", frozenEdit.base, "--head", frozenHead, "--event-base-ref", "main", "--default-ref", "main"],
  });
  assert.equal(frozenResult.valid, false);
  assert.match(frozenResult.errors.join("\n"), /immutable after bootstrap/);

  using workflowEdit = g2Fixture("als-g2-workflow-edit");
  git(workflowEdit.path, ["switch", "--create", "workflow-change"]);
  writeFile(workflowEdit.path, ".github/workflows/workflow--test-scripts.yaml", "name: changed lifecycle registration\n");
  const workflowHead = commitAll(workflowEdit.path, "edit workflow registration");
  const workflowResult = runCommand({
    repoRoot: workflowEdit.path,
    command: "validate-range",
    argv: ["--base", workflowEdit.base, "--head", workflowHead, "--event-base-ref", "main", "--default-ref", "main"],
  });
  assert.equal(workflowResult.valid, false);
  assert.match(workflowResult.errors.join("\n"), /immutable after bootstrap/);
});

test("workflow registers lifecycle validation for documentation checks", () => {
  const workflow = fs.readFileSync(".github/workflows/workflow--test-scripts.yaml", "utf8");

  assert.equal(workflow.includes("check-agent-language-services-lifecycle.mjs validate-range"), true);
  assert.match(workflow, /AGENT_LANGUAGE_SERVICES_BASE_SHA/);
  assert.equal(workflow.includes("github.event.pull_request.base.ref"), true);
  assert.equal(workflow.includes("docs/**/*.json"), true);
});

function repositoryAuthority() {
  return JSON.parse(fs.readFileSync("docs/reference/agent-language-services-lifecycle-review/source-decisions.json", "utf8"));
}

function repositoryRoots() {
  return parseMarkdownSource({ source: fs.readFileSync("docs/proposals/agent-language-services.md", "utf8") });
}

function gapSafeSpan(root) {
  const end = Math.min([...root.text].length, 4);
  return [0, Math.max(1, end)];
}

function writeG0(repoRoot) {
  writeFile(repoRoot, "docs/proposals/agent-language-services.md", sampleUmbrella());
  writeFile(repoRoot, "docs/proposals/agent-language-services-inventory-review-gate.md", "---\nrole: proposal\nupdate-when: The gate changes.\n---\n\n# Gate\n");
  writeFile(repoRoot, "docs/proposals/README.md", "# Proposals\n\n## Ready\n\n- [Gate](agent-language-services-inventory-review-gate.md#g0-to-g1-review-gate).\n");
  writeFile(repoRoot, "docs/proposals/agent-language-services-lifecycle-migration.md", "---\nrole: proposal\nupdate-when: The lifecycle migration changes.\n---\n\n# Lifecycle\n");
  writeFile(repoRoot, "docs/specification/mcp.md", "---\nrole: specification\nauthority: normative\nupdate-when: The MCP specification changes.\n---\n\n# MCP Workspace Projects, Diagnostics, And Definitions\n");
  writeFile(repoRoot, "examples/specification/README.md", "# Specification Examples\n");
  writeFile(repoRoot, ".github/workflows/workflow--test-scripts.yaml", "name: workflow / test scripts\n");
  writeFile(repoRoot, "docs/reference/implemented-proposals/README.md", "---\nrole: routing\nupdate-when: Records change.\n---\n\n# Records\n");
  writeFile(repoRoot, "docs/reference/README.md", "---\nrole: routing\nupdate-when: Routes change.\n---\n\n# Reference\n");
  writeFile(repoRoot, "docs/reference/proposal-target-readiness/manifest.json", "{\"schema_version\":1,\"entries\":[]}\n");
  return commitAll(repoRoot, "g0");
}

function writeG1(repoRoot) {
  fs.rmSync(path.join(repoRoot, "docs/proposals/agent-language-services-inventory-review-gate.md"));
  writeFile(repoRoot, "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md", "---\nrole: implementation-record\nauthority: supporting\nupdate-when: The gate completion evidence is superseded.\n---\n\n# Gate Record\n");
  writeFile(repoRoot, "workflow-scripts/check-agent-language-services-lifecycle.mjs", fs.readFileSync("workflow-scripts/check-agent-language-services-lifecycle.mjs", "utf8"));
  writeFile(repoRoot, "workflow-scripts/check-agent-language-services-lifecycle.test.mjs", "// fixture test placeholder\n");
  const source = fs.readFileSync(path.join(repoRoot, "docs/proposals/agent-language-services.md"), "utf8");
  const parsedRoots = parseMarkdownSource({ source });
  writeFile(repoRoot, "docs/reference/agent-language-services-lifecycle-review/source-decisions.json", `${JSON.stringify(sampleAuthority(parsedRoots), null, 2)}\n`);
}

function writeFrozenDestinationFiles(repoRoot) {
  writeFile(repoRoot, "docs/specification/mcp.md", "---\nrole: specification\nauthority: normative\nupdate-when: The MCP specification changes.\n---\n\n# MCP Workspace Projects, Diagnostics, And Definitions\n");
  writeFile(repoRoot, "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md", "---\nrole: implementation-record\nauthority: supporting\nupdate-when: The gate completion evidence is superseded.\n---\n\n# Agent Language Services Inventory Review Gate\n");
  writeFile(repoRoot, "examples/specification/README.md", "# Specification Examples\n");
}

function g2Fixture(name) {
  const fixture = tempRepo(name);
  writeG0(fixture.path);
  git(fixture.path, ["switch", "--create", "gate"]);
  writeG1(fixture.path);
  writeFrozenDestinationFiles(fixture.path);
  const g1 = commitAll(fixture.path, "complete review gate");
  git(fixture.path, ["branch", "--force", "main", g1]);
  git(fixture.path, ["switch", "--create", "inventory"]);
  const writeResult = runCommand({ repoRoot: fixture.path, command: "write-frozen-artifacts", argv: ["--base-commit", g1] });
  assert.equal(writeResult.valid, true, writeResult.errors.join("\n"));
  const g2 = commitAll(fixture.path, "freeze inventory");
  git(fixture.path, ["branch", "--force", "main", g2]);
  return {
    path: fixture.path,
    base: g2,
    [Symbol.dispose]() {
      fixture[Symbol.dispose]();
    },
  };
}

function sampleUmbrella() {
  return [
    "---",
    "role: proposal",
    "update-when: The sample changes.",
    "---",
    "",
    "# Agent Language Services",
    "",
    "The server currently exposes one tool. Broader reference work remains planned.",
    "",
    "| Case | Evidence |",
    "| --- | --- |",
    "| Q01 anonymous diagnostics | Evidence. |",
    "| Q02 descendant ownership | Evidence. |",
    "| Q03 rediscovery | Evidence. |",
    "| Q04 filesystem identity | Evidence. |",
    "| Q05 stable capture | Evidence. |",
    "| Q06 schemas and errors | Evidence. |",
    "| Q07 coordinates | Evidence. |",
    "| Q08 reference universe | Evidence. |",
    "| Q09 cursors | Evidence. |",
    "| Q10 resource lifetime | Evidence. |",
    "| Q11 package digest | Evidence. |",
    "| Q12 distribution set | Evidence. |",
    "| Q13 portable domains | Evidence. |",
    "| Q14 URI spelling | Evidence. |",
    "| Q15 disclosure | Evidence. |",
    "| Q16 document identity | Evidence. |",
    "| Q17 language catalog | Evidence. |",
    "| Q18 generation failure | Evidence. |",
    "| Q19 search and reads | Evidence. |",
    "| Q20 executable binding | Evidence. |",
    "| Q21 plugin matrix | Evidence. |",
    "| Q22 gate totality | Evidence. |",
    "",
    "- saved reference one",
    "- saved reference two",
    "- saved reference three",
    "- saved reference four",
    "- saved reference five",
    "- saved reference six",
    "- navigation row",
    "- topic row",
    "",
    "`check_project` `definition` `references` source resources modules UTF-8 plugin-cell acceptance-row",
    "",
  ].join("\n");
}

function sampleAuthority(parsedRoots) {
  const roots = parsedRoots.map((root) => ({
    ...root,
    source_class: "conformance",
    leaves: [{ id: `${root.id}-L01`, lifecycle: "planned", spans: [[0, Math.max(1, [...root.text].length)]] }],
  }));
  const identities = [];
  const root = roots[0];
  for (let index = 1; index <= 22; index += 1) {
    const name = `Q${String(index).padStart(2, "0")}`;
    const owner = roots.find((candidate) => candidate.text.includes(name)) ?? root;
    identities.push({ kind: "evidence_gate", name, root: owner.id, leaf: `${owner.id}-L01`, span: [0, Math.min(2, [...owner.text].length)] });
  }
  for (let index = 1; index <= 6; index += 1) {
    identities.push({ kind: "saved_reference_row", name: `saved-${index}`, root: root.id, leaf: `${root.id}-L01`, span: [0, 1] });
  }
  for (const kind of ["navigation_matrix_row", "topic_matrix_row", "tool_kind", "resource_kind", "package_document_declaration_kind", "lsp_encoding", "plugin_compatibility_cell", "unresolved_acceptance_row"]) {
    identities.push({ kind, name: kind, root: root.id, leaf: `${root.id}-L01`, span: [0, 1] });
  }
  return { schema_version: 1, source_path: "docs/proposals/agent-language-services.md", roots, identities };
}

function tempRepo(name) {
  const fixture = tempDirectory(name);
  git(fixture.path, ["init", "-b", "main"]);
  git(fixture.path, ["config", "user.name", "Test User"]);
  git(fixture.path, ["config", "user.email", "test@example.invalid"]);
  return fixture;
}

function tempDirectory(name) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-`));
  return {
    path: directory,
    [Symbol.dispose]() {
      fs.rmSync(directory, { recursive: true, force: true });
    },
  };
}

function copyFile(source, destination) {
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.copyFileSync(source, destination);
}

function copyDirectory(source, destination) {
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.cpSync(source, destination, { recursive: true });
}

function frozenFixture(name) {
  const fixture = tempDirectory(name);
  copyFile("docs/proposals/agent-language-services.md", path.join(fixture.path, "docs/proposals/agent-language-services.md"));
  copyFile("docs/reference/agent-language-services-lifecycle-review/source-decisions.json", path.join(fixture.path, "docs/reference/agent-language-services-lifecycle-review/source-decisions.json"));
  copyDirectory("docs/reference/agent-language-services-lifecycle", path.join(fixture.path, "docs/reference/agent-language-services-lifecycle"));
  copyFile("docs/specification/mcp.md", path.join(fixture.path, "docs/specification/mcp.md"));
  copyFile("docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md", path.join(fixture.path, "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md"));
  copyFile("examples/specification/README.md", path.join(fixture.path, "examples/specification/README.md"));
  return fixture;
}

function readJsonFile(repoRoot, file) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, file), "utf8"));
}

function writeJsonFile(repoRoot, file, value) {
  writeFile(repoRoot, file, `${JSON.stringify(value, null, 2)}\n`);
}

function escapeRegExp(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function writeFile(repoRoot, file, content) {
  const destination = path.join(repoRoot, file);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, content);
}

function commitAll(repoRoot, message) {
  git(repoRoot, ["add", "."]);
  git(repoRoot, ["commit", "-m", message]);
  return git(repoRoot, ["rev-parse", "HEAD"]).stdout.trim();
}

function git(repoRoot, args) {
  const result = spawnSync("git", args, { cwd: repoRoot, encoding: "utf8" });
  assert.equal(result.status, 0, `git ${args.join(" ")}\n${result.stderr}`);
  return result;
}
