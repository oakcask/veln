import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  buildFrozenArtifacts,
  buildSourceDecisionArtifact,
  parseUmbrellaProposal,
  validateDiffScope,
  validateMigrationLedger,
  validateRepository,
} from "./check-agent-language-services-lifecycle.mjs";

test("repository reviewed source decisions match the umbrella proposal", () => {
  assert.deepEqual(validateRepository({ repoRoot: "." }), []);
});

test("committed frozen artifacts match generated lifecycle artifacts", () => {
  const generated = buildFrozenArtifacts({ repoRoot: "." });
  const actual = {
    sourceUniverse: readJson("docs/reference/agent-language-services-lifecycle/source-universe.json"),
    inventory: readJson("docs/reference/agent-language-services-lifecycle/inventory.json"),
    lifecycleManifest: readJson("docs/reference/agent-language-services-lifecycle/lifecycle-manifest.json"),
    ledgerSchema: readJson("docs/reference/agent-language-services-lifecycle/migration-ledger.schema.json"),
    ledgerFixture: readJson("docs/reference/agent-language-services-lifecycle/migration-ledger.fixture.json"),
  };

  assert.deepEqual(actual, generated);
});

test("structural parser covers source roots without semantic decision fields", () => {
  const parsed = parseUmbrellaProposal({ repoRoot: "." });

  assert.ok(parsed.roots.length > 100);
  assert.ok(parsed.roots.some((root) => root.kind === "list-item"));
  assert.ok(parsed.roots.some((root) => root.kind === "table-row"));
  assert.equal("lifecycle" in parsed.roots[0], false);
  assert.equal("identities" in parsed.roots[0], false);
});

test("rejects changed digest and changed source text", () => {
  const artifact = buildSourceDecisionArtifact({ repoRoot: "." });
  artifact.roots[0].digest = "sha256:bad";

  assert.match(validateRepository({ repoRoot: ".", artifact }).join("\n"), /exact reviewed source text and digest/);
});

test("rejects missing and duplicate inventory roots", () => {
  const missing = buildSourceDecisionArtifact({ repoRoot: "." });
  const removed = missing.roots.pop();
  const duplicate = buildSourceDecisionArtifact({ repoRoot: "." });
  duplicate.roots.push(structuredClone(duplicate.roots[0]));

  assert.match(validateRepository({ repoRoot: ".", artifact: missing }).join("\n"), new RegExp(`${removed.id}: add missing source root`));
  assert.match(validateRepository({ repoRoot: ".", artifact: duplicate }).join("\n"), /remove duplicate source root/);
});

test("rejects missing child and non-contiguous child IDs", () => {
  const missing = buildSourceDecisionArtifact({ repoRoot: "." });
  missing.roots[0].leaf_count = 2;
  const nonContiguous = buildSourceDecisionArtifact({ repoRoot: "." });
  nonContiguous.roots[0].leaves[0].id = `${nonContiguous.roots[0].id}-L03`;

  assert.match(validateRepository({ repoRoot: ".", artifact: missing }).join("\n"), /add missing child leaf/);
  assert.match(validateRepository({ repoRoot: ".", artifact: nonContiguous }).join("\n"), /use contiguous child leaf IDs/);
});

test("rejects gap, overlap, and out-of-range child spans", () => {
  const gap = buildSourceDecisionArtifact({ repoRoot: "." });
  gap.roots[0].leaves[0].spans = gap.roots[0].leaves[0].spans.slice(1);
  const overlap = buildSourceDecisionArtifact({ repoRoot: "." });
  overlap.roots[0].leaves[0].spans.push(structuredClone(overlap.roots[0].leaves[0].spans[0]));
  const outOfRange = buildSourceDecisionArtifact({ repoRoot: "." });
  outOfRange.roots[0].leaves[0].spans[0].end_scalar = 999999;

  assert.match(validateRepository({ repoRoot: ".", artifact: gap }).join("\n"), /cover source scalar/);
  assert.match(validateRepository({ repoRoot: ".", artifact: overlap }).join("\n"), /child spans overlap/);
  assert.match(validateRepository({ repoRoot: ".", artifact: outOfRange }).join("\n"), /outside the source root/);
});

test("rejects wrong, mixed, and invalidly removed lifecycle decisions", () => {
  const wrong = buildSourceDecisionArtifact({ repoRoot: "." });
  wrong.roots.find((root) => root.source_class === "conformance").leaves[0].lifecycle = "unknown";
  const removed = buildSourceDecisionArtifact({ repoRoot: "." });
  removed.roots.find((root) => root.source_class === "conformance").leaves[0].lifecycle = "removed";

  assert.match(validateRepository({ repoRoot: ".", artifact: wrong }).join("\n"), /set lifecycle to current, completed, planned, or removed/);
  assert.match(validateRepository({ repoRoot: ".", artifact: removed }).join("\n"), /conformance leaves may not use removed lifecycle/);
});

test("rejects uncovered parent lifecycle statement", () => {
  const artifact = buildSourceDecisionArtifact({ repoRoot: "." });
  const root = artifact.roots.find((candidate) => candidate.leaves[0].spans.length > 1);
  root.leaves[0].spans = [root.leaves[0].spans[0]];
  root.leaves[0].text = root.leaves[0].text.slice(0, 1);

  assert.match(validateRepository({ repoRoot: ".", artifact }).join("\n"), /cover source scalar/);
});

test("rejects direct parent ledger mapping shape in bootstrap diff scope", () => {
  const { inventory, lifecycleManifest, ledgerFixture } = buildFrozenArtifacts({ repoRoot: "." });
  const parentMapping = structuredClone(ledgerFixture);
  parentMapping.entries[0].source_id = inventory.roots[0].id;

  assert.match(validateMigrationLedger({ ledger: parentMapping, lifecycleManifest, inventory }).join("\n"), /map child leaves, not a parent/);
});

test("rejects missing, duplicate, wildcard, range, catch-all, and invalidly removed ledger leaves", () => {
  const { inventory, lifecycleManifest, ledgerFixture } = buildFrozenArtifacts({ repoRoot: "." });
  const missing = structuredClone(ledgerFixture);
  const removed = missing.entries.pop();
  const duplicate = structuredClone(ledgerFixture);
  duplicate.entries.push(structuredClone(duplicate.entries[0]));
  const wildcard = structuredClone(ledgerFixture);
  wildcard.entries[0].source_id = "ALS-R0001-*";
  const range = structuredClone(ledgerFixture);
  range.entries[0].source_id = "ALS-R0001-L01..ALS-R0002-L01";
  const catchAll = structuredClone(ledgerFixture);
  catchAll.entries[0].source_id = "all remaining leaves";
  const invalidRemoved = structuredClone(ledgerFixture);
  const conformanceLeaf = lifecycleManifest.leaves.find((leaf) => leaf.source_class === "conformance");
  invalidRemoved.entries.find((entry) => entry.source_id === conformanceLeaf.id).lifecycle = "removed";

  assert.match(validateMigrationLedger({ ledger: missing, lifecycleManifest, inventory }).join("\n"), new RegExp(`${removed.source_id}: add missing ledger mapping`));
  assert.match(validateMigrationLedger({ ledger: duplicate, lifecycleManifest, inventory }).join("\n"), /remove duplicate ledger mapping/);
  assert.match(validateMigrationLedger({ ledger: wildcard, lifecycleManifest, inventory }).join("\n"), /ranges, wildcards, and catch-all entries are rejected/);
  assert.match(validateMigrationLedger({ ledger: range, lifecycleManifest, inventory }).join("\n"), /ranges, wildcards, and catch-all entries are rejected/);
  assert.match(validateMigrationLedger({ ledger: catchAll, lifecycleManifest, inventory }).join("\n"), /ranges, wildcards, and catch-all entries are rejected/);
  assert.match(validateMigrationLedger({ ledger: invalidRemoved, lifecycleManifest, inventory }).join("\n"), /conformance leaves may not use removed ledger mappings/);
});

test("rejects lifecycle-manifest disagreement and source-universe omission", () => {
  const generated = buildFrozenArtifacts({ repoRoot: "." });
  const wrongLifecycle = structuredClone(generated.lifecycleManifest);
  wrongLifecycle.leaves[0].lifecycle = wrongLifecycle.leaves[0].lifecycle === "planned" ? "current" : "planned";
  const missingUniverse = structuredClone(generated.sourceUniverse);
  const removed = missingUniverse.roots.pop();

  assert.match(validateRepository({ repoRoot: ".", ...generated, lifecycleManifest: wrongLifecycle }).join("\n"), /restore lifecycle manifest leaf from the inventory/);
  assert.match(validateRepository({ repoRoot: ".", ...generated, sourceUniverse: missingUniverse }).join("\n"), new RegExp(`${removed.id}: add missing source-universe root`));
});

test("rejects bootstrap changes to MCP harness, executable fixtures, and semantic baselines", () => {
  const errors = validateDiffScope({
    changedPaths: [
      "crates/veln-mcp/src/server.rs",
      "examples/specification/mcp/workspace-lifecycle/case.toml",
      "crates/veln-cli/tests/toolchain-case-semantics.baseline",
    ],
    hasFrozenArtifact: true,
    isBootstrap: true,
  });

  assert.match(errors.join("\n"), /must not alter harness code, executable MCP fixtures, or semantic baselines/);
});

test("rejects post-bootstrap frozen artifact and validator registration edits", () => {
  const errors = validateDiffScope({
    changedPaths: [
      "docs/reference/agent-language-services-lifecycle/inventory.json",
      "workflow-scripts/check-agent-language-services-lifecycle.mjs",
      ".github/workflows/workflow--test-scripts.yaml",
    ],
    hasFrozenArtifact: true,
    isBootstrap: false,
  });

  assert.match(errors.join("\n"), /immutable frozen lifecycle artifact/);
});

test("rejects missing, duplicate, wildcard, and detached finite identities", () => {
  const missing = buildSourceDecisionArtifact({ repoRoot: "." });
  for (const root of missing.roots) {
    root.identities = (root.identities ?? []).filter((identity) => !(identity.kind === "evidence-gate" && identity.name === "Q01"));
  }
  const duplicate = buildSourceDecisionArtifact({ repoRoot: "." });
  const identityRoot = duplicate.roots.find((root) => root.identities.length > 0);
  identityRoot.identities.push(structuredClone(identityRoot.identities[0]));
  const wildcard = buildSourceDecisionArtifact({ repoRoot: "." });
  wildcard.roots.find((root) => root.identities.length > 0).identities[0].kind = "*";

  assert.match(validateRepository({ repoRoot: ".", artifact: missing }).join("\n"), /evidence-gate identity Q01/);
  assert.match(validateRepository({ repoRoot: ".", artifact: duplicate }).join("\n"), /remove duplicate identity/);
  assert.match(validateRepository({ repoRoot: ".", artifact: wildcard }).join("\n"), /declared finite identity kind/);
});

test("requires tracked provenance for the frozen-inventory bootstrap", () => {
  using fixture = tempRepo();
  fs.mkdirSync(path.join(fixture.root, "docs/proposals"), { recursive: true });
  fs.cpSync("docs/proposals/agent-language-services.md", path.join(fixture.root, "docs/proposals/agent-language-services.md"));
  const artifact = buildSourceDecisionArtifact({ repoRoot: fixture.root });
  const frozen = buildFrozenArtifacts({ repoRoot: "." });

  assert.match(validateRepository({ repoRoot: fixture.root, artifact, ...frozen }).join("\n"), /add tracked target provenance/);
});

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
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
