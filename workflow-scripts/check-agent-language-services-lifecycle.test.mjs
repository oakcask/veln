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
  validateLedgerStructure,
  writeGeneratedArtifacts,
} from "./check-agent-language-services-lifecycle.mjs";

test("repository lifecycle artifacts validate", () => {
  const result = validateArtifacts({ repoRoot: ".", ...readRepoArtifacts() });

  assert.equal(result.valid, true, result.errors.join("\n"));
});

test("accepts freshly generated artifacts", () => {
  using fixture = tempRepo();
  copyFile(fixture.root, "docs/proposals/agent-language-services.md");
  copyFile(fixture.root, "docs/specification/mcp.md");
  copyFile(fixture.root, "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md");
  copyFile(fixture.root, "examples/specification/mcp/workspace-lifecycle/case.toml");
  const generated = generateArtifacts({ repoRoot: fixture.root });
  writeJson(path.join(fixture.root, "docs/reference/agent-language-services-lifecycle/source-universe.json"), generated.universe);

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
    ledgerSchema: artifacts.ledgerSchema,
  });

  assert.match(errors.join("\n"), /not the parent/);
  assert.match(errors.join("\n"), /duplicate mapping/);
  assert.match(errors.join("\n"), /ranges, wildcards, and catch-all/);
  assert.match(errors.join("\n"), /do not remove a leaf/);
  assert.match(errors.join("\n"), /add exactly one ledger mapping/);
});

test("rejects malformed migration ledger schema", () => {
  const artifacts = readRepoArtifacts();
  artifacts.ledgerSchema.properties.entries.items.properties.lifecycle.enum = ["current", "planned"];
  artifacts.ledgerSchema.properties.entries.items.properties.leaf_id.pattern = "[";

  const result = validateArtifacts({ repoRoot: ".", ...artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /migration ledger schema|leaf_id_pattern|lifecycle enum/);
});

test("runs structural ledger cases through schema and semantic validation", () => {
  const artifacts = readRepoArtifacts();
  const valid = structuredClone(artifacts.ledgerFixture);
  const invalidCases = [
    ["missing entries", { schema_version: 1 }],
    ["duplicate evidence", withFirstEntry(valid, (entry) => {
      entry.destination.evidence = [
        "docs/proposals/agent-language-services.md",
        "docs/proposals/agent-language-services.md",
      ];
    })],
    ["wildcard leaf", withFirstEntry(valid, (entry) => {
      entry.leaf_id = "*";
    })],
    ["unsupported field", withFirstEntry(valid, (entry) => {
      entry.range = "ALS-S0001..ALS-S0004";
    })],
    ["invalid destination path", withFirstEntry(valid, (entry) => {
      entry.destination.path = "tmp/not-a-doc.txt";
    })],
  ];

  assert.deepEqual(validateLedgerStructure({ ledger: valid, ledgerSchema: artifacts.ledgerSchema }), []);
  assert.deepEqual(validateLedger({
    ledger: valid,
    inventory: artifacts.inventory,
    manifest: artifacts.manifest,
    ledgerSchema: artifacts.ledgerSchema,
  }), []);

  for (const [name, ledger] of invalidCases) {
    const schemaErrors = validateLedgerStructure({ ledger, ledgerSchema: artifacts.ledgerSchema });
    const semanticErrors = validateLedger({
      ledger,
      inventory: artifacts.inventory,
      manifest: artifacts.manifest,
      ledgerSchema: artifacts.ledgerSchema,
    });

    assert.notEqual(schemaErrors.length, 0, name);
    assert.notEqual(semanticErrors.length, 0, name);
  }
});

test("rejects unresolved ledger destinations and duplicate evidence", () => {
  const artifacts = readRepoArtifacts();
  const entry = structuredClone(artifacts.ledgerFixture.entries[0]);
  entry.destination.path = "docs/proposals/missing-agent-language-services.md";
  entry.destination.anchor = "#missing-anchor";
  entry.destination.evidence = [
    "examples/specification/mcp/workspace-lifecycle/case.toml",
    "examples/specification/mcp/workspace-lifecycle/case.toml",
    "tmp/not-checked.txt",
  ];
  artifacts.ledgerFixture.entries[0] = entry;

  const result = validateArtifacts({ repoRoot: ".", ...artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /resolve destination path/);
  assert.match(result.errors.join("\n"), /list each checked evidence path once/);
  assert.match(result.errors.join("\n"), /resolve checked evidence/);
});

test("rejects synchronized finite identity deletion from universe and source decisions", () => {
  const artifacts = readRepoArtifacts();
  artifacts.universe.identities = artifacts.universe.identities.filter((identity) => identity.kind !== "tool");
  artifacts.sourceDecisions.identities = artifacts.sourceDecisions.identities.filter((identity) => identity.kind !== "tool");

  const result = validateArtifacts({ repoRoot: ".", ...artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /preserve the named finite tool identity/);
});

test("rejects synchronized finite identity occurrence deletion from universe and source decisions", () => {
  const artifacts = readRepoArtifacts();
  const deleted = artifacts.universe.identities.find((identity, index, identities) => identities.some((other, otherIndex) => otherIndex !== index && other.kind === identity.kind && other.name === identity.name));
  artifacts.universe.identities = artifacts.universe.identities.filter((identity) => JSON.stringify(identity) !== JSON.stringify(deleted));
  artifacts.sourceDecisions.identities = artifacts.sourceDecisions.identities.filter((identity) => JSON.stringify(identity) !== JSON.stringify(deleted));

  const result = validateArtifacts({ repoRoot: ".", ...artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /source-bound finite identity occurrence/);
});

test("does not overwrite reviewed source decisions when generating artifacts", () => {
  using fixture = tempRepo();
  copyFile(fixture.root, "docs/proposals/agent-language-services.md");
  const reviewPath = path.join(fixture.root, "docs/reference/agent-language-services-lifecycle-review/source-decisions.json");
  fs.mkdirSync(path.dirname(reviewPath), { recursive: true });
  fs.writeFileSync(reviewPath, "{\"sentinel\":true}\n");

  writeGeneratedArtifacts(fixture.root);

  assert.equal(fs.readFileSync(reviewPath, "utf8"), "{\"sentinel\":true}\n");
});

test("rejects bootstrap override without checked provenance refs", () => {
  const previousBootstrap = process.env.AGENT_LANGUAGE_SERVICES_BOOTSTRAP;
  const previousBase = process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA;
  const previousHead = process.env.AGENT_LANGUAGE_SERVICES_HEAD_SHA;
  process.env.AGENT_LANGUAGE_SERVICES_BOOTSTRAP = "1";
  delete process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA;
  delete process.env.AGENT_LANGUAGE_SERVICES_HEAD_SHA;
  try {
    const result = validateDiffScope({
      repoRoot: ".",
      paths: [
        "docs/reference/agent-language-services-lifecycle/frozen-inventory.json",
        "examples/specification/mcp/workspace-lifecycle/case.toml",
      ],
    });

    assert.equal(result.valid, false);
    assert.match(result.errors.join("\n"), /provide concrete base and head refs/);
  } finally {
    if (previousBootstrap === undefined) {
      delete process.env.AGENT_LANGUAGE_SERVICES_BOOTSTRAP;
    } else {
      process.env.AGENT_LANGUAGE_SERVICES_BOOTSTRAP = previousBootstrap;
    }
    restoreEnv("AGENT_LANGUAGE_SERVICES_BASE_SHA", previousBase);
    restoreEnv("AGENT_LANGUAGE_SERVICES_HEAD_SHA", previousHead);
  }
});

test("rejects bootstrap paths outside the frozen-inventory scope", () => {
  const previousBase = process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA;
  const previousHead = process.env.AGENT_LANGUAGE_SERVICES_HEAD_SHA;
  process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA = "a4a3b874928a713f1078a302311bb2b22103e2ee";
  process.env.AGENT_LANGUAGE_SERVICES_HEAD_SHA = "62ea5beb1a5763bb4db6b62419cdf7204de695ff";
  try {
    const result = validateDiffScope({
      repoRoot: ".",
      paths: [
        "docs/reference/agent-language-services-lifecycle/frozen-inventory.json",
        "examples/specification/mcp/workspace-lifecycle/case.toml",
      ],
    });

    assert.equal(result.valid, false);
    assert.match(result.errors.join("\n"), /prerequisite must already exist|mixing unrelated paths/);
  } finally {
    restoreEnv("AGENT_LANGUAGE_SERVICES_BASE_SHA", previousBase);
    restoreEnv("AGENT_LANGUAGE_SERVICES_HEAD_SHA", previousHead);
  }
});

test("rejects post-bootstrap frozen lifecycle changes", () => {
  const previousBase = process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA;
  process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA = "0".repeat(39) + "1";
  try {
    const result = validateDiffScope({
      repoRoot: ".",
      paths: [
        "docs/reference/agent-language-services-lifecycle/frozen-inventory.json",
      ],
    });

    assert.equal(result.valid, false);
    assert.match(result.errors.join("\n"), /immutable after the provenance base has merged/);
  } finally {
    if (previousBase === undefined) {
      delete process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA;
    } else {
      process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA = previousBase;
    }
  }
});

function readRepoArtifacts() {
  return {
    universe: readJson("docs/reference/agent-language-services-lifecycle/source-universe.json"),
    inventory: readJson("docs/reference/agent-language-services-lifecycle/frozen-inventory.json"),
    manifest: readJson("docs/reference/agent-language-services-lifecycle/lifecycle-manifest.json"),
    ledgerSchema: readJson("docs/reference/agent-language-services-lifecycle/migration-ledger.schema.json"),
    ledgerFixture: readJson("docs/reference/agent-language-services-lifecycle/migration-ledger.schema-fixture.json"),
    provenance: readJson("docs/reference/agent-language-services-lifecycle/target-provenance.json"),
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

function withFirstEntry(ledger, mutate) {
  const copy = structuredClone(ledger);
  mutate(copy.entries[0]);
  return copy;
}

function restoreEnv(name, value) {
  if (value === undefined) {
    delete process.env[name];
  } else {
    process.env[name] = value;
  }
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(relativePath, "utf8"));
}

function writeJson(file, value) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
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
