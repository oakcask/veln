import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  validateCatalogCoverage,
  validateManifestShape,
  validateTargetReadiness,
  validateTargetShape,
} from "./check-proposal-target-readiness.mjs";

test("repository proposal readiness manifest matches the catalog", () => {
  const manifest = JSON.parse(fs.readFileSync("docs/reference/proposal-target-readiness/manifest.json", "utf8"));
  const result = validateCatalogCoverage({ repoRoot: ".", manifest });

  assert.deepEqual(result, []);
});

test("accepts a ready proposal target", () => {
  using fixture = tempRepo("proposal-ready");
  const manifest = readinessFixture(fixture.root);
  const metadata = {
    proposal_path: "docs/proposals/ready.md",
    proposal_anchor: "#ready",
    default_branch: "main",
    base_commit: "abcdef1",
    prerequisites: [],
    target_kind: "proposal",
  };

  assert.deepEqual(validateTargetReadiness({ repoRoot: fixture.root, manifest, metadata }), []);
});

test("rejects blocked and unlisted proposal targets", () => {
  using fixture = tempRepo("proposal-blocked");
  const manifest = readinessFixture(fixture.root);
  const blocked = {
    proposal_path: "docs/proposals/blocked.md",
    proposal_anchor: "#blocked",
    default_branch: "main",
    base_commit: "abcdef1",
    prerequisites: ["docs/proposals/prerequisite.md"],
    target_kind: "proposal",
  };
  const unlisted = { ...blocked, proposal_path: "docs/proposals/missing.md", proposal_anchor: "#missing", prerequisites: [] };

  assert.match(validateTargetReadiness({ repoRoot: fixture.root, manifest, metadata: blocked }).join("\n"), /select a Ready prerequisite/);
  assert.match(validateTargetReadiness({ repoRoot: fixture.root, manifest, metadata: blocked }).join("\n"), /complete docs\/proposals\/prerequisite\.md/);
  assert.match(validateTargetReadiness({ repoRoot: fixture.root, manifest, metadata: unlisted }).join("\n"), /absent from the readiness manifest/);
});

test("rejects no-target while Ready still has entries", () => {
  using fixture = tempRepo("proposal-no-target");
  const manifest = readinessFixture(fixture.root);
  const metadata = {
    proposal_path: "docs/proposals/ready.md",
    proposal_anchor: "#ready",
    default_branch: "main",
    base_commit: "abcdef1",
    prerequisites: [],
    target_kind: "no-target",
  };

  assert.match(validateTargetReadiness({ repoRoot: fixture.root, manifest, metadata }).join("\n"), /Ready still contains/);
});

test("rejects malformed manifest and target metadata", () => {
  const manifestErrors = validateManifestShape({
    schema: { $id: "https://veln-lang.invalid/schemas/proposal-target-readiness-manifest.schema.json" },
    manifest: {
      entries: [
        { proposal_path: "/docs/proposals/ready.md", proposal_anchor: "ready", state: "future", prerequisites: ["../blocked.md", "../blocked.md"] },
      ],
    },
  });
  const targetErrors = validateTargetShape({
    schema: {
      $id: "https://veln-lang.invalid/schemas/proposal-target-readiness-target.schema.json",
      additionalProperties: false,
      properties: { target_kind: { enum: ["proposal", "proposal-section", "no-target"] } },
    },
    metadata: {
      proposal_path: "docs/proposals/ready.md",
      proposal_anchor: "",
      default_branch: "",
      base_commit: "not-a-commit",
      prerequisites: ["docs/proposals/prerequisite.md", "docs/proposals/prerequisite.md"],
      target_kind: "blocked",
    },
  });

  assert.match(manifestErrors.join("\n"), /proposal_path/);
  assert.match(manifestErrors.join("\n"), /proposal heading anchor/);
  assert.match(manifestErrors.join("\n"), /ready or blocked/);
  assert.match(manifestErrors.join("\n"), /duplicate prerequisite/);
  assert.match(targetErrors.join("\n"), /target_kind/);
  assert.match(targetErrors.join("\n"), /base commit/);
  assert.match(targetErrors.join("\n"), /duplicate prerequisite/);
});

test("rejects catalog and manifest drift", () => {
  using fixture = tempRepo("proposal-catalog-drift");
  writeProposal(fixture.root, "ready.md", "Ready");
  writeProposal(fixture.root, "blocked.md", "Blocked");
  fs.writeFileSync(
    path.join(fixture.root, "docs/proposals/README.md"),
    [
      "# Proposals",
      "",
      "## Ready",
      "",
      "- [Ready](ready.md).",
      "",
      "## Blocked",
      "",
      "- [Blocked](blocked.md).",
    ].join("\n"),
  );
  const manifest = {
    entries: [
      { proposal_path: "docs/proposals/ready.md", proposal_anchor: "#ready", state: "blocked", prerequisites: [] },
      { proposal_path: "docs/proposals/extra.md", proposal_anchor: "#extra", state: "ready", prerequisites: [] },
    ],
  };

  const errors = validateCatalogCoverage({ repoRoot: fixture.root, manifest });

  assert.match(errors.join("\n"), /set state to ready/);
  assert.match(errors.join("\n"), /add docs\/proposals\/blocked\.md#blocked/);
  assert.match(errors.join("\n"), /remove docs\/proposals\/extra\.md#extra/);
});

function readinessFixture(repoRoot) {
  writeProposal(repoRoot, "ready.md", "Ready");
  writeProposal(repoRoot, "blocked.md", "Blocked");
  writeProposal(repoRoot, "prerequisite.md", "Prerequisite");
  return {
    entries: [
      { proposal_path: "docs/proposals/ready.md", proposal_anchor: "#ready", state: "ready", prerequisites: [] },
      { proposal_path: "docs/proposals/blocked.md", proposal_anchor: "#blocked", state: "blocked", prerequisites: ["docs/proposals/prerequisite.md"] },
    ],
  };
}

function writeProposal(repoRoot, file, title) {
  fs.mkdirSync(path.join(repoRoot, "docs/proposals"), { recursive: true });
  fs.writeFileSync(
    path.join(repoRoot, "docs/proposals", file),
    ["---", "role: proposal", "update-when: The fixture changes.", "---", "", `# ${title}`, ""].join("\n"),
  );
}

function tempRepo(name) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-`));
  return {
    root,
    [Symbol.dispose]() {
      fs.rmSync(root, { recursive: true, force: true });
    },
  };
}
