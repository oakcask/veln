import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  validateAgentLanguagePlatformMatrix,
  validateTransitionDiffScope,
} from "./check-agent-language-platform-matrix.mjs";

const proposalPath = path.resolve("docs/proposals/agent-language-services.md");

test("repository agent language-services matrix is valid", () => {
  const result = validateAgentLanguagePlatformMatrix(
    fs.readFileSync(proposalPath, "utf8"),
  );

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects matrix row count, order, duplicates, and compatibility values", () => {
  const valid = fs.readFileSync(proposalPath, "utf8");

  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid.replace("| claude-code | x86_64-unknown-linux-gnu |\n", ""),
    ),
    "restore exactly 2 matrix row(s)",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid
        .replace("| codex | x86_64-unknown-linux-gnu |", "| TEMP | x86_64-unknown-linux-gnu |")
        .replace("| claude-code | x86_64-unknown-linux-gnu |", "| codex | x86_64-unknown-linux-gnu |")
        .replace("| TEMP | x86_64-unknown-linux-gnu |", "| claude-code | x86_64-unknown-linux-gnu |"),
    ),
    "restore row 1 to codex/x86_64-unknown-linux-gnu",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid.replace("| claude-code | x86_64-unknown-linux-gnu |", "| codex | x86_64-unknown-linux-gnu |"),
    ),
    "remove duplicate client-platform key",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid
        .replace("| Client | Platform |", "| Client | Platform | host-build |")
        .replace("| --- | --- |", "| --- | --- | --- |")
        .replace("| codex | x86_64-unknown-linux-gnu |", "| codex | x86_64-unknown-linux-gnu | pinned |"),
    ),
    "remove compatibility values from matrix row 1",
  );
});

test("rejects wildcard, placeholder, displaced, and hidden matrix tables", () => {
  const valid = fs.readFileSync(proposalPath, "utf8");

  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid.replace("| codex | x86_64-unknown-linux-gnu |", "| codex | * |"),
    ),
    "replace ranged, wildcard, placeholder, or catch-all key",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid.replace("| claude-code | x86_64-unknown-linux-gnu |", "| claude-code | future-platform |"),
    ),
    "replace ranged, wildcard, placeholder, or catch-all key",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid.replace("### Closed Client-Platform Matrix", "### Other Matrix"),
    ),
    "restore exactly one ### Closed Client-Platform Matrix heading",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid.replace(
        "### Closed Client-Platform Matrix",
        "> ### Closed Client-Platform Matrix",
      ),
    ),
    "restore exactly one ### Closed Client-Platform Matrix heading",
  );
});

test("rejects missing, reordered, duplicate, unexpected, and value-bearing field identities", () => {
  const valid = fs.readFileSync(proposalPath, "utf8");

  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid.replace("| validator-integrity |\n", ""),
    ),
    "restore exactly 11 compatibility field identities",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid
        .replace("| validator-version |", "| TEMP |")
        .replace("| validator-integrity |", "| validator-version |")
        .replace("| TEMP |", "| validator-integrity |"),
    ),
    "restore compatibility field 5 to \"validator-version\"",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid.replace("| validator-integrity |", "| validator-version |"),
    ),
    "remove duplicate compatibility field",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid.replace("| validator-integrity |", "| integrity-digest |"),
    ),
    "restore compatibility field 6 to \"validator-integrity\"",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(
      valid
        .replace("| Field |", "| Field | Value |")
        .replace("| --- |", "| --- | --- |")
        .replace("| client |", "| client | codex |"),
    ),
    "remove compatibility values from field identity row 1",
  );
});

test("rejects missing, hidden, mistitled, and wrong-destination matrix references", () => {
  const valid = fs.readFileSync(proposalPath, "utf8");
  const link = '[Closed Client-Platform Matrix](#closed-client-platform-matrix "matrix-ref:q21-plugin-matrix")';

  assertHasError(
    validateAgentLanguagePlatformMatrix(valid.replace(link, "Closed matrix")),
    "restore exactly one titled matrix link for q21-plugin-matrix",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(valid.replace(link, `\`${link}\``)),
    "restore exactly one titled matrix link for q21-plugin-matrix",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(valid.replace(link, link.replace("Closed Client-Platform Matrix", "matrix"))),
    "restore exactly one titled matrix link for q21-plugin-matrix",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(valid.replace(link, link.replace("#closed-client-platform-matrix", "#other"))),
    "restore exactly one titled matrix link for q21-plugin-matrix",
  );
  assertHasError(
    validateAgentLanguagePlatformMatrix(valid.replace(link, `!${link}`)),
    "restore exactly one titled matrix link for q21-plugin-matrix",
  );
});

test("rejects unbound supported-platform references", () => {
  const valid = fs.readFileSync(proposalPath, "utf8");
  const result = validateAgentLanguagePlatformMatrix(`${valid}\nEvery supported platform must pass.\n`);

  assertHasError(result, "route this platform-set reference to the closed matrix table");
});

test("transition diff guard accepts only the matrix closure manifest while active", () => {
  const entries = [
    entry("M", ".github/workflows/workflow--test-scripts.yaml"),
    entry("M", "docs/proposals/README.md"),
    entry("M", "docs/proposals/agent-language-services-lifecycle-migration.md"),
    entry("D", "docs/proposals/agent-language-services-platform-matrix-closure.md"),
    entry("M", "docs/proposals/agent-language-services.md"),
    entry("M", "docs/reference/implemented-proposals/README.md"),
    entry("A", "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.mjs"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.test.mjs"),
  ];

  assert.deepEqual(validateTransitionDiffScope({
    baseHasMatrix: false,
    headHasMatrix: true,
    entries,
  }), []);
});

test("transition diff guard rejects extra paths, renames, git type changes, and executable changes", () => {
  assertHasDiffError([
    entry("M", ".github/workflows/workflow--test-scripts.yaml"),
    entry("M", "docs/proposals/README.md"),
    entry("M", "docs/proposals/agent-language-services-lifecycle-migration.md"),
    entry("D", "docs/proposals/agent-language-services-platform-matrix-closure.md"),
    entry("M", "docs/proposals/agent-language-services.md"),
    entry("M", "docs/reference/implemented-proposals/README.md"),
    entry("A", "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.mjs"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.test.mjs"),
    entry("M", ".github/workflows/unrelated.yaml"),
  ], "remove this path from the matrix-closure transition");

  assertHasDiffError([
    entry("M", ".github/workflows/workflow--test-scripts.yaml"),
    entry("M", "docs/proposals/README.md"),
    entry("M", "docs/proposals/agent-language-services-lifecycle-migration.md"),
    entry("D", "docs/proposals/agent-language-services-platform-matrix-closure.md"),
    entry("D", "docs/proposals/agent-language-services.md"),
    entry("M", "docs/reference/implemented-proposals/README.md"),
    entry("A", "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.mjs"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.test.mjs"),
  ], "restore operation M");

  assertHasDiffError([
    entry("M", ".github/workflows/workflow--test-scripts.yaml"),
    entry("M", "docs/proposals/README.md"),
    entry("M", "docs/proposals/agent-language-services-lifecycle-migration.md"),
    entry("D", "docs/proposals/agent-language-services-platform-matrix-closure.md"),
    entry("M", "docs/proposals/agent-language-services.md", { newType: "symlink" }),
    entry("M", "docs/reference/implemented-proposals/README.md"),
    entry("A", "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.mjs"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.test.mjs"),
  ], "keep the head path as a regular file");

  assertHasDiffError([
    entry("M", ".github/workflows/workflow--test-scripts.yaml"),
    entry("M", "docs/proposals/README.md"),
    entry("M", "docs/proposals/agent-language-services-lifecycle-migration.md"),
    entry("D", "docs/proposals/agent-language-services-platform-matrix-closure.md"),
    entry("M", "docs/proposals/agent-language-services.md", { newExecutable: true }),
    entry("M", "docs/reference/implemented-proposals/README.md"),
    entry("A", "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.mjs"),
    entry("A", "workflow-scripts/check-agent-language-platform-matrix.test.mjs"),
  ], "keep this transition non-executable");
});

test("transition diff guard retires after the closure", () => {
  const errors = validateTransitionDiffScope({
    baseHasMatrix: true,
    headHasMatrix: true,
    entries: [entry("M", "docs/README.md")],
  });

  assert.deepEqual(errors, []);
});

function entry(status, path, overrides = {}) {
  return {
    status,
    path,
    oldType: status === "A" ? undefined : "blob",
    newType: status === "D" ? undefined : "blob",
    oldExecutable: false,
    newExecutable: false,
    ...overrides,
  };
}

function assertHasError(result, substring) {
  assert.equal(result.valid, false);
  assert(
    result.errors.some((error) => error.includes(substring)),
    `expected an error containing ${JSON.stringify(substring)}, got:\n${result.errors.join("\n")}`,
  );
}

function assertHasDiffError(entries, substring) {
  const errors = validateTransitionDiffScope({
    baseHasMatrix: false,
    headHasMatrix: true,
    entries,
  });
  assert(
    errors.some((error) => error.includes(substring)),
    `expected a diff error containing ${JSON.stringify(substring)}, got:\n${errors.join("\n")}`,
  );
}
