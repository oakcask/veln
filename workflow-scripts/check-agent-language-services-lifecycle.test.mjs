import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  buildSourceDecisionArtifact,
  parseUmbrellaProposal,
  validateDiffScope,
  validateRepository,
} from "./check-agent-language-services-lifecycle.mjs";

test("repository reviewed source decisions match the umbrella proposal", () => {
  assert.deepEqual(validateRepository({ repoRoot: "." }), []);
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
  const errors = validateDiffScope({
    changedPaths: ["docs/proposals/agent-language-services.md"],
    hasFrozenArtifact: true,
    isBootstrap: true,
  });

  assert.match(errors.join("\n"), /outside the reviewed allowlist/);
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

  assert.match(validateRepository({ repoRoot: fixture.root, artifact }).join("\n"), /add tracked target provenance/);
});

function tempRepo() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "als-lifecycle-"));
  return {
    root,
    [Symbol.dispose]() {
      fs.rmSync(root, { recursive: true, force: true });
    },
  };
}
