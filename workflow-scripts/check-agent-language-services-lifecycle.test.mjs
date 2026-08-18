import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  generateArtifacts,
  validateArtifacts,
  validateDiffScope,
  validateLedger,
  writeGeneratedArtifacts,
} from "./check-agent-language-services-lifecycle.mjs";

test("repository lifecycle artifacts validate", () => {
  const result = validateArtifacts({ repoRoot: ".", ...readRepoArtifacts() });

  assert.equal(result.valid, true, result.errors.join("\n"));
});

test("accepts freshly generated artifacts", () => {
  using fixture = tempRepo();
  copyFile(fixture.root, "docs/proposals/agent-language-services.md");
  const generated = generateArtifacts({ repoRoot: fixture.root });

  const result = validateArtifacts({ repoRoot: fixture.root, ...generated });

  assert.equal(result.valid, true, result.errors.join("\n"));
});

test("rejects changed digest and missing or duplicate inventory items", () => {
  const artifacts = readRepoArtifacts();
  artifacts.inventory.roots[0].digest = "0".repeat(64);
  artifacts.inventory.roots.splice(1, 1);
  artifacts.inventory.roots.push(structuredClone(artifacts.inventory.roots[2]));

  const result = validateArtifacts({ repoRoot: ".", ...artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /digest/);
  assert.match(result.errors.join("\n"), /missing source inventory item/);
  assert.match(result.errors.join("\n"), /duplicate item/);
});

test("rejects missing child and span gap overlap or out of range", () => {
  const artifacts = readRepoArtifacts();
  const root = firstParent(artifacts.inventory);
  root.child_count += 1;
  root.children[0].spans[0][1] = root.children[0].spans[0][0];
  root.children[1].spans[0][0] = root.children[0].spans[0][0];
  root.children[1].spans[0][1] = root.span[1] + 1;

  const result = validateArtifacts({ repoRoot: ".", ...artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /child_count|contiguous child ID/);
  assert.match(result.errors.join("\n"), /inside|overlaps|uncovered/);
});

test("rejects wrong or mixed lifecycle inventory leaves", () => {
  const artifacts = readRepoArtifacts();
  const parent = firstParent(artifacts.inventory);
  parent.children = [{
    id: `${parent.id}.1`,
    lifecycle: "planned",
    spans: [parent.span],
    digest: parent.digest,
  }];
  parent.child_count = 1;

  const result = validateArtifacts({ repoRoot: ".", ...artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /mixes lifecycle statements|lifecycle equal/);
});

test("rejects direct parent, missing, duplicate, wildcard, and invalid removed ledger leaves", () => {
  const artifacts = readRepoArtifacts();
  const parent = firstParent(artifacts.inventory);
  const leaf = artifacts.manifest.leaves[0];
  const errors = validateLedger({
    ledger: {
      schema_version: 1,
      entries: [
        ledgerEntry(parent.id, "planned"),
        ledgerEntry(leaf.id, "removed"),
        ledgerEntry(leaf.id, leaf.lifecycle),
        ledgerEntry("ALS-S0001..ALS-S9999", "planned"),
        ledgerEntry("*", "planned"),
      ],
    },
    inventory: artifacts.inventory,
    manifest: artifacts.manifest,
  });

  assert.match(errors.join("\n"), /not the parent/);
  assert.match(errors.join("\n"), /duplicate mapping/);
  assert.match(errors.join("\n"), /ranges, wildcards, and catch-all/);
  assert.match(errors.join("\n"), /do not remove a leaf/);
  assert.match(errors.join("\n"), /add exactly one ledger mapping/);
});

test("rejects bootstrap paths outside the frozen-inventory scope", () => {
  const result = validateDiffScope({
    repoRoot: ".",
    paths: [
      "docs/reference/agent-language-services-lifecycle/frozen-inventory.json",
      "examples/specification/mcp/workspace-lifecycle/case.toml",
    ],
  });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /mixing unrelated paths/);
});

function readRepoArtifacts() {
  return {
    universe: readJson("docs/reference/agent-language-services-lifecycle/source-universe.json"),
    inventory: readJson("docs/reference/agent-language-services-lifecycle/frozen-inventory.json"),
    manifest: readJson("docs/reference/agent-language-services-lifecycle/lifecycle-manifest.json"),
    ledgerFixture: readJson("docs/reference/agent-language-services-lifecycle/migration-ledger.schema-fixture.json"),
    sourceDecisions: readJson("docs/reference/agent-language-services-lifecycle-review/source-decisions.json"),
  };
}

function firstParent(inventory) {
  return inventory.roots.find((root) => root.children?.length > 1);
}

function ledgerEntry(leafId, lifecycle) {
  return {
    leaf_id: leafId,
    lifecycle,
    destination: {
      kind: lifecycle === "current" ? "specification" : lifecycle === "completed" ? "implementation-record" : lifecycle === "removed" ? "removed" : "proposal",
      path: "docs/proposals/agent-language-services.md",
      anchor: "#agent-language-services",
      evidence: ["docs/proposals/agent-language-services.md"],
      rationale: "duplicate supporting explanation",
    },
  };
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(relativePath, "utf8"));
}

function copyFile(repoRoot, relativePath) {
  const target = path.join(repoRoot, relativePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(relativePath, target);
}

function tempRepo() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "als-lifecycle-"));
  return {
    root,
    [Symbol.dispose]() {
      fs.rmSync(root, { recursive: true, force: true });
    },
  };
}
