import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  buildAcceptanceLedger,
  parseSourceUniverse,
  validateAgentLanguageServicesPlatformMatrix,
  validateAgentLanguageServicesInventory,
  validateDiffScope,
  validateInventory,
  validateMigrationLedgerJsonSchema,
  validateMigrationLedger,
  validateUniverse,
  writeArtifacts,
} from "./check-agent-language-services-inventory.mjs";

const repoRoot = process.cwd();

test("accepts the checked frozen inventory artifacts", () => {
  const result = validateAgentLanguageServicesInventory({ repoRoot, checkDiffScope: false });
  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects a changed digest", () => {
  const fixture = fixtureData();
  fixture.universe.roots[0].digest = "0".repeat(64);
  const errors = validateUniverse({ parsed: fixture.parsed, universe: fixture.universe });
  assert(errors.some((error) => error.includes("wrong digest")));
});

test("rejects missing independent source-universe contract identities", () => {
  const fixture = fixtureData();
  removeIdentity(fixture.universe, "Q21");
  assert(validateUniverse({ parsed: fixture.parsed, universe: fixture.universe }).some((error) => error.includes("evidence_gate identity Q21")));

  const missingTool = fixtureData();
  removeIdentity(missingTool.universe, "references");
  assert(validateUniverse({ parsed: missingTool.parsed, universe: missingTool.universe }).some((error) => error.includes("mcp_tools identity references")));

  const missingResource = fixtureData();
  removeIdentity(missingResource.universe, "veln-pkg:");
  assert(validateUniverse({ parsed: missingResource.parsed, universe: missingResource.universe }).some((error) => error.includes("mcp_resource_schemes identity veln-pkg:")));

  const missingDeclaration = fixtureData();
  removeIdentity(missingDeclaration.universe, "PackageIdentity");
  assert(validateUniverse({ parsed: missingDeclaration.parsed, universe: missingDeclaration.universe }).some((error) => error.includes("package_document_declarations identity PackageIdentity")));

  const missingEncoding = fixtureData();
  removeIdentity(missingEncoding.universe, "veln/virtualDocument");
  assert(validateUniverse({ parsed: missingEncoding.parsed, universe: missingEncoding.universe }).some((error) => error.includes("lsp_encodings identity veln/virtualDocument")));

  const missingPluginCell = fixtureData();
  removeIdentity(missingPluginCell.universe, "Claude Code");
  assert(validateUniverse({ parsed: missingPluginCell.parsed, universe: missingPluginCell.universe }).some((error) => error.includes("plugin_cells identity Claude Code")));

  const missingClientPlatform = fixtureData();
  removeIdentity(missingClientPlatform.universe, "claude-code/x86_64-unknown-linux-gnu");
  assert(validateUniverse({ parsed: missingClientPlatform.parsed, universe: missingClientPlatform.universe }).some((error) => error.includes("plugin_client_platform_keys identity claude-code/x86_64-unknown-linux-gnu")));
});

test("rejects invalid closed client-platform matrix rows and references", () => {
  const proposal = fs.readFileSync(path.join(repoRoot, "docs/proposals/agent-language-services.md"), "utf8");
  assert.deepEqual(validateAgentLanguageServicesPlatformMatrix(proposal), []);

  const missingRow = proposal.replace(/\| `claude-code\/x86_64-unknown-linux-gnu`.+\n/, "");
  assert(validateAgentLanguageServicesPlatformMatrix(missingRow).some((error) => error.includes("expected 2 rows")));

  const duplicateKey = proposal.replace("`claude-code/x86_64-unknown-linux-gnu`", "`codex/x86_64-unknown-linux-gnu`");
  assert(validateAgentLanguageServicesPlatformMatrix(duplicateKey).some((error) => error.includes("duplicate client-platform key")));

  const wildcard = proposal.replace("`codex/x86_64-unknown-linux-gnu`", "`codex/*`");
  assert(validateAgentLanguageServicesPlatformMatrix(wildcard).some((error) => error.includes("exact literal")));

  const badDigest = proposal.replace("`0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef`", "`0123`");
  assert(validateAgentLanguageServicesPlatformMatrix(badDigest).some((error) => error.includes("64 lowercase hexadecimal")));

  const unnamedReference = `${proposal}\nEvery supported client and platform must pass.\n`;
  assert(validateAgentLanguageServicesPlatformMatrix(unnamedReference).some((error) => error.includes("closed client-platform matrix rows")));
});

test("rejects a missing or duplicate inventory item", () => {
  const fixture = fixtureData();
  fixture.inventory.roots.pop();
  assert(validateInventory(fixture).some((error) => error.includes("expected one inventory root")));

  const duplicate = fixtureData();
  duplicate.inventory.roots[1] = structuredClone(duplicate.inventory.roots[0]);
  assert(validateInventory(duplicate).some((error) => error.includes("duplicate inventory root")));
});

test("rejects a missing child", () => {
  const fixture = fixtureData();
  const root = fixture.inventory.roots.find((candidate) => candidate.child_count > 1) ?? fixture.inventory.roots[0];
  root.children.pop();
  assert(validateInventory(fixture).some((error) => error.includes("child_count does not match children")));
});

test("rejects span gaps, overlaps, and out-of-range spans", () => {
  const gap = fixtureData();
  gap.inventory.roots[0].children[0].spans[0].start += 1;
  assert(validateInventory(gap).some((error) => error.includes("span gap or overlap")));

  const overlap = fixtureData();
  overlap.inventory.roots[0].children[0].spans[0].end += 1;
  assert(validateInventory(overlap).some((error) => error.includes("span gap or overlap")));

  const range = fixtureData();
  range.inventory.roots[0].children[0].spans[0].end = 1_000_000;
  assert(validateInventory(range).some((error) => error.includes("out-of-range span")));
});

test("rejects a wrong lifecycle and a manifest mismatch", () => {
  const fixture = fixtureData();
  const child = fixture.inventory.roots[0].children[0];
  child.lifecycle = child.lifecycle === "planned" ? "current" : "planned";
  assert(validateInventory(fixture).some((error) => error.includes("lifecycle differs from reviewed manifest")));
});

test("rejects an uncovered parent lifecycle statement", () => {
  const fixture = fixtureData();
  const lastChild = fixture.inventory.roots[0].children.at(-1);
  const childSpan = lastChild.spans[0];
  childSpan.end -= 1;
  fixture.inventory.roots[0].separator_spans = [];
  assert(validateInventory(fixture).some((error) => error.includes("uncovered source text")));
});

test("validates the generated acceptance ledger", () => {
  const fixture = fixtureData();
  const ledger = buildAcceptanceLedger(fixture);
  assert.deepEqual(validateMigrationLedgerJsonSchema({ ledger }).errors, []);
  const result = validateMigrationLedger({ repoRoot, ledger, inventory: fixture.inventory, manifest: fixture.manifest });
  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects direct parent, missing, duplicate, wildcard, and invalid removed ledger leaves", () => {
  const fixture = fixtureData();
  const ledger = buildAcceptanceLedger(fixture);
  const firstLeaf = ledger.entries[0].source_id;
  const parent = fixture.inventory.roots[0].id;

  const directParent = structuredClone(ledger);
  directParent.entries[0].source_id = parent;
  assert(validateMigrationLedger({ repoRoot, ledger: directParent, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("maps a parent")));

  const missing = structuredClone(ledger);
  missing.entries.pop();
  assert(validateMigrationLedger({ repoRoot, ledger: missing, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("missing leaf mapping")));

  const duplicate = structuredClone(ledger);
  duplicate.entries[1].source_id = firstLeaf;
  assert(validateMigrationLedger({ repoRoot, ledger: duplicate, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("duplicate leaf mapping")));

  const wildcard = structuredClone(ledger);
  wildcard.entries[0].source_id = "agent-language-services/S*.c01";
  assert(validateMigrationLedger({ repoRoot, ledger: wildcard, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("wildcard")));

  const removed = structuredClone(ledger);
  const conformanceEntry = removed.entries.find((entry) => fixture.manifest.leaves.find((leaf) => leaf.source_id === entry.source_id).conformance);
  conformanceEntry.lifecycle = "removed";
  conformanceEntry.destination = { kind: "removed", rationale: "Duplicate explanation." };
  assert(validateMigrationLedger({ repoRoot, ledger: removed, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("cannot be removed")));
});

test("rejects invalid ledger destination evidence", () => {
  const fixture = fixtureData();
  const ledger = buildAcceptanceLedger(fixture);
  const current = ledger.entries.find((entry) => entry.lifecycle === "current");
  current.destination.evidence = ["docs/proposals/agent-language-services.md"];
  assert(validateMigrationLedger({ repoRoot, ledger, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("allowlisted checked route")));
});

test("rejects duplicate current evidence and summary-only planned destinations", () => {
  const fixture = fixtureData();
  const duplicateEvidence = buildAcceptanceLedger(fixture);
  const currentEntries = duplicateEvidence.entries.filter((entry) => entry.lifecycle === "current");
  currentEntries[1].destination.evidence = [...currentEntries[0].destination.evidence];
  assert(validateMigrationLedger({ repoRoot, ledger: duplicateEvidence, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("reused by more than one current leaf")));

  const summaryPlanned = buildAcceptanceLedger(fixture);
  const planned = summaryPlanned.entries.find((entry) => entry.lifecycle === "planned");
  planned.destination.anchor = "acceptance-model";
  assert(validateMigrationLedger({ repoRoot, ledger: summaryPlanned, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("not the acceptance-model summary")));
});

test("validates removed supporting leaves with superseding destinations", () => {
  const sourceId = "agent-language-services/S9999.c01";
  const inventory = {
    roots: [{ id: "agent-language-services/S9999", child_count: 1 }],
  };
  const manifest = {
    leaves: [{ source_id: sourceId, parent_id: "agent-language-services/S9999", lifecycle: "removed", conformance: false }],
  };
  const ledger = {
    schema_version: 1,
    entries: [{
      source_id: sourceId,
      lifecycle: "removed",
      destination: {
        kind: "removed",
        rationale: "Duplicate supporting explanation.",
        supersedes: {
          path: "docs/reference/agent-language-services-frozen-inventory.md",
          anchor: "agent-language-services-frozen-inventory",
        },
      },
    }],
  };
  assert.deepEqual(validateMigrationLedgerJsonSchema({ ledger }).errors, []);
  assert.deepEqual(validateMigrationLedger({ repoRoot, ledger, inventory, manifest }).errors, []);
});

test("keeps ledger schema and semantic validator aligned for structural cases", () => {
  const fixture = fixtureData();
  const base = buildAcceptanceLedger(fixture);
  const cases = [
    (ledger) => delete ledger.schema_version,
    (ledger) => { ledger.extra = true; },
    (ledger) => delete ledger.entries[0].destination,
    (ledger) => { ledger.entries[0].extra = true; },
    (ledger) => { ledger.entries[0].source_id = "agent-language-services/S*.c01"; },
    (ledger) => { ledger.entries[0].lifecycle = "later"; },
    (ledger) => { ledger.entries.find((entry) => entry.lifecycle === "current").destination.kind = "planned"; },
    (ledger) => { delete ledger.entries.find((entry) => entry.lifecycle === "current").destination.evidence; },
    (ledger) => {
      const current = ledger.entries.find((entry) => entry.lifecycle === "current");
      current.destination.evidence = [current.destination.evidence[0], current.destination.evidence[0]];
    },
  ];
  for (const mutate of cases) {
    const ledger = structuredClone(base);
    mutate(ledger);
    const schemaValid = validateMigrationLedgerJsonSchema({ ledger }).valid;
    const semanticValid = validateMigrationLedger({ repoRoot, ledger, inventory: fixture.inventory, manifest: fixture.manifest }).valid;
    assert.equal(schemaValid, semanticValid);
    assert.equal(schemaValid, false);
  }
});

test("rejects destinations with lifecycle-incompatible markdown roles", () => {
  const fixture = fixtureData();
  const plannedAsSpec = buildAcceptanceLedger(fixture);
  const planned = plannedAsSpec.entries.find((entry) => entry.lifecycle === "planned");
  planned.destination.path = "docs/specification/mcp.md";
  planned.destination.anchor = "mcp-workspace-projects-diagnostics-and-definitions";
  assert(validateMigrationLedger({ repoRoot, ledger: plannedAsSpec, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("planned destination must be an active proposal")));

  const completedAsProposal = buildAcceptanceLedger(fixture);
  const completed = completedAsProposal.entries.find((entry) => entry.lifecycle === "completed");
  completed.destination.path = "docs/proposals/agent-language-services.md";
  completed.destination.anchor = "acceptance-model";
  assert(validateMigrationLedger({ repoRoot, ledger: completedAsProposal, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("completed destination must be an implementation record")));

  const currentAsProposal = buildAcceptanceLedger(fixture);
  const current = currentAsProposal.entries.find((entry) => entry.lifecycle === "current");
  current.destination.path = "docs/proposals/agent-language-services.md";
  current.destination.anchor = "acceptance-model";
  assert(validateMigrationLedger({ repoRoot, ledger: currentAsProposal, inventory: fixture.inventory, manifest: fixture.manifest }).errors.some((error) => error.includes("current destination must be a normative specification")));
});

test("enforces the first-pr diff scope guard", () => {
  assert.deepEqual(validateDiffScope({
    baseHasFrozen: false,
    headHasFrozen: true,
    changes: [
      { path: "docs/reference/agent-language-services-frozen-inventory.json" },
      { path: "workflow-scripts/check-agent-language-services-inventory.mjs" },
    ],
  }), []);

  assert(validateDiffScope({
    baseHasFrozen: false,
    headHasFrozen: true,
    changes: [{ path: "docs/proposals/agent-language-services.md" }],
  }).some((error) => error.includes("restore the umbrella proposal")));

  assert(validateDiffScope({
    baseHasFrozen: false,
    headHasFrozen: true,
    changes: [{ path: "crates/veln-mcp/src/lib.rs" }],
  }).some((error) => error.includes("behavior PR")));

  assert.deepEqual(validateDiffScope({
    baseHasFrozen: true,
    headHasFrozen: true,
    changes: [{ path: "docs/proposals/agent-language-services.md" }],
  }), []);
});

function fixtureData() {
  const proposal = fs.readFileSync(path.join(repoRoot, "docs/proposals/agent-language-services.md"), "utf8");
  return {
    repoRoot,
    parsed: parseSourceUniverse(proposal),
    universe: readJson("docs/reference/agent-language-services-source-universe.json"),
    inventory: readJson("docs/reference/agent-language-services-frozen-inventory.json"),
    manifest: readJson("docs/reference/agent-language-services-lifecycle-manifest.json"),
  };
}

function removeIdentity(universe, identity) {
  for (const root of universe.roots) {
    root.identities = (root.identities ?? []).filter((candidate) => candidate !== identity);
  }
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, file), "utf8"));
}
