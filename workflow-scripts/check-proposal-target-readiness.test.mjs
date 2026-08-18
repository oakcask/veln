import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
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

const manifestSchema = {
  $id: "https://veln-lang.invalid/schemas/proposal-target-readiness-manifest.schema.json",
};

test("repository proposal readiness manifest matches the catalog", () => {
  const manifest = JSON.parse(fs.readFileSync("docs/reference/proposal-target-readiness/manifest.json", "utf8"));
  const result = validateCatalogCoverage({ repoRoot: ".", manifest });

  assert.deepEqual(result, []);
});

test("accepts a ready proposal target at its committed base", () => {
  using fixture = tempRepo("proposal-ready");
  writeProposal(fixture.root, "ready.md", "Ready");
  writeCatalog(fixture.root, { ready: ["ready.md"], blocked: [] });
  writeManifest(fixture.root, [manifestEntry("ready.md", "Ready", "ready")]);
  const baseCommit = commitAll(fixture.root, "ready base");
  const metadata = targetMetadata({ baseCommit });

  assert.deepEqual(validateTargetReadiness({ repoRoot: fixture.root, metadata, manifestSchema }), []);
});

test("rejects blocked and unlisted proposal targets at the committed base", () => {
  using fixture = tempRepo("proposal-blocked");
  writeProposal(fixture.root, "ready.md", "Ready");
  writeProposal(fixture.root, "blocked.md", "Blocked");
  writeProposal(fixture.root, "prerequisite.md", "Prerequisite");
  writeCatalog(fixture.root, { ready: ["ready.md"], blocked: ["blocked.md"] });
  writeManifest(fixture.root, [
    manifestEntry("ready.md", "Ready", "ready"),
    manifestEntry("blocked.md", "Blocked", "blocked", ["docs/proposals/prerequisite.md"]),
  ]);
  const baseCommit = commitAll(fixture.root, "blocked base");
  const blocked = targetMetadata({
    baseCommit,
    proposalPath: "docs/proposals/blocked.md",
    proposalAnchor: "#blocked",
    prerequisites: ["docs/proposals/prerequisite.md"],
  });
  const unlisted = { ...blocked, proposal_path: "docs/proposals/missing.md", proposal_anchor: "#missing", prerequisites: [] };

  const blockedErrors = validateTargetReadiness({ repoRoot: fixture.root, metadata: blocked, manifestSchema });
  assert.match(blockedErrors.join("\n"), /select a Ready prerequisite/);
  assert.match(blockedErrors.join("\n"), /complete docs\/proposals\/prerequisite\.md on the declared base/);
  assert.match(blockedErrors.join("\n"), /add docs\/reference\/implemented-proposals\/prerequisite\.md on the declared base/);
  assert.match(validateTargetReadiness({ repoRoot: fixture.root, metadata: unlisted, manifestSchema }).join("\n"), /absent from its readiness manifest/);
});

test("rejects no-target while the committed base still has a Ready entry", () => {
  using fixture = tempRepo("proposal-no-target");
  writeProposal(fixture.root, "ready.md", "Ready");
  writeCatalog(fixture.root, { ready: ["ready.md"], blocked: [] });
  writeManifest(fixture.root, [manifestEntry("ready.md", "Ready", "ready")]);
  const baseCommit = commitAll(fixture.root, "nonempty ready base");
  const metadata = targetMetadata({ baseCommit, targetKind: "no-target" });

  assert.match(validateTargetReadiness({ repoRoot: fixture.root, metadata, manifestSchema }).join("\n"), /declared base still contains/);
});

test("rejects a nonexistent or abbreviated base commit", () => {
  using fixture = tempRepo("proposal-missing-base");
  const nonexistent = targetMetadata({ baseCommit: "a".repeat(40) });
  const abbreviated = { ...nonexistent, base_commit: "abcdef1" };
  const schema = {
    $id: "https://veln-lang.invalid/schemas/proposal-target-readiness-target.schema.json",
    additionalProperties: false,
    properties: { target_kind: { enum: ["proposal", "proposal-section", "no-target"] } },
  };

  assert.match(validateTargetReadiness({ repoRoot: fixture.root, metadata: nonexistent, manifestSchema }).join("\n"), /existing full base commit/);
  assert.match(validateTargetShape({ metadata: abbreviated, schema }).join("\n"), /40-character hexadecimal base commit/);
});

test("rejects a prerequisite completed only in the working tree", () => {
  using fixture = tempRepo("proposal-working-tree-prerequisite");
  writeProposal(fixture.root, "ready.md", "Ready");
  writeProposal(fixture.root, "prerequisite.md", "Prerequisite");
  writeCatalog(fixture.root, { ready: ["ready.md"], blocked: [] });
  writeManifest(fixture.root, [manifestEntry("ready.md", "Ready", "ready", ["docs/proposals/prerequisite.md"])]);
  const baseCommit = commitAll(fixture.root, "base with active prerequisite");
  fs.rmSync(path.join(fixture.root, "docs/proposals/prerequisite.md"));
  writeImplementedRecord(fixture.root, "prerequisite.md");
  const metadata = targetMetadata({ baseCommit, prerequisites: ["docs/proposals/prerequisite.md"] });

  const errors = validateTargetReadiness({ repoRoot: fixture.root, metadata, manifestSchema });
  assert.match(errors.join("\n"), /complete docs\/proposals\/prerequisite\.md on the declared base/);
  assert.match(errors.join("\n"), /add docs\/reference\/implemented-proposals\/prerequisite\.md on the declared base/);
});

test("rejects a working branch whose merge base differs from target metadata", () => {
  using fixture = tempRepo("proposal-stale-base");
  writeProposal(fixture.root, "ready.md", "Ready");
  writeCatalog(fixture.root, { ready: ["ready.md"], blocked: [] });
  writeManifest(fixture.root, [manifestEntry("ready.md", "Ready", "ready")]);
  const declaredBase = commitAll(fixture.root, "declared base");
  fs.writeFileSync(path.join(fixture.root, "later.txt"), "later\n");
  commitAll(fixture.root, "later default branch");
  git(fixture.root, ["switch", "--create", "target"]);
  const metadata = targetMetadata({ baseCommit: declaredBase });

  assert.match(validateTargetReadiness({ repoRoot: fixture.root, metadata, manifestSchema }).join("\n"), /working branch merge base is .* not the declared base/);
});

test("rejects malformed manifest and target metadata", () => {
  const manifestErrors = validateManifestShape({
    schema: manifestSchema,
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

test("rejects catalog, role, heading, and manifest drift", () => {
  using fixture = tempRepo("proposal-catalog-drift");
  writeProposal(fixture.root, "ready.md", "Ready");
  writeProposal(fixture.root, "blocked.md", "Blocked", "reference");
  writeCatalog(fixture.root, { ready: ["ready.md#missing-heading"], blocked: ["blocked.md"] });
  const manifest = {
    entries: [
      { proposal_path: "docs/proposals/ready.md", proposal_anchor: "#missing-heading", state: "blocked", prerequisites: [] },
      { proposal_path: "docs/proposals/extra.md", proposal_anchor: "#extra", state: "ready", prerequisites: [] },
    ],
  };

  const errors = validateCatalogCoverage({ repoRoot: fixture.root, manifest });

  assert.match(errors.join("\n"), /restore the linked heading/);
  assert.match(errors.join("\n"), /only active proposals can be selected/);
  assert.match(errors.join("\n"), /set state to ready/);
  assert.match(errors.join("\n"), /add docs\/proposals\/blocked\.md#blocked/);
  assert.match(errors.join("\n"), /remove docs\/proposals\/extra\.md#extra/);
});

function targetMetadata({
  baseCommit,
  proposalPath = "docs/proposals/ready.md",
  proposalAnchor = "#ready",
  prerequisites = [],
  targetKind = "proposal",
}) {
  return {
    proposal_path: proposalPath,
    proposal_anchor: proposalAnchor,
    default_branch: "main",
    base_commit: baseCommit,
    prerequisites,
    target_kind: targetKind,
  };
}

function manifestEntry(file, title, state, prerequisites = []) {
  return {
    proposal_path: `docs/proposals/${file}`,
    proposal_anchor: `#${title.toLowerCase().replaceAll(" ", "-")}`,
    state,
    prerequisites,
  };
}

function writeCatalog(repoRoot, { ready, blocked }) {
  fs.mkdirSync(path.join(repoRoot, "docs/proposals"), { recursive: true });
  fs.writeFileSync(
    path.join(repoRoot, "docs/proposals/README.md"),
    [
      "# Proposals",
      "",
      "## Ready",
      "",
      ...ready.map((target) => `- [Ready](${target}).`),
      "",
      "## Blocked",
      "",
      ...blocked.map((target) => `- [Blocked](${target}).`),
      "",
    ].join("\n"),
  );
}

function writeManifest(repoRoot, entries) {
  const directory = path.join(repoRoot, "docs/reference/proposal-target-readiness");
  fs.mkdirSync(directory, { recursive: true });
  fs.writeFileSync(path.join(directory, "manifest.json"), `${JSON.stringify({ schema_version: 1, entries }, null, 2)}\n`);
}

function writeProposal(repoRoot, file, title, role = "proposal") {
  fs.mkdirSync(path.join(repoRoot, "docs/proposals"), { recursive: true });
  fs.writeFileSync(
    path.join(repoRoot, "docs/proposals", file),
    ["---", `role: ${role}`, "update-when: The fixture changes.", "---", "", `# ${title}`, ""].join("\n"),
  );
}

function writeImplementedRecord(repoRoot, file) {
  const directory = path.join(repoRoot, "docs/reference/implemented-proposals");
  fs.mkdirSync(directory, { recursive: true });
  fs.writeFileSync(path.join(directory, file), "# Implemented\n");
}

function commitAll(repoRoot, message) {
  git(repoRoot, ["add", "."]);
  git(repoRoot, ["commit", "-m", message]);
  return git(repoRoot, ["rev-parse", "HEAD"]).stdout.trim();
}

function git(repoRoot, args) {
  const result = spawnSync("git", args, { cwd: repoRoot, encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  return result;
}

function tempRepo(name) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-`));
  git(root, ["init", "--initial-branch", "main"]);
  git(root, ["config", "user.name", "Fixture"]);
  git(root, ["config", "user.email", "fixture@example.invalid"]);
  return {
    root,
    [Symbol.dispose]() {
      fs.rmSync(root, { recursive: true, force: true });
    },
  };
}
