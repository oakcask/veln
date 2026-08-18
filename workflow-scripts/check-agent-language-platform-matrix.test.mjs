import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  agentLanguageProposalPath,
  expectedClientPlatformRows,
  expectedCompatibilityFields,
  expectedReferenceLinks,
  renderGitHubErrorAnnotation,
  validateAgentLanguagePlatformMatrix,
  validateClosureDiffScope,
} from "./check-agent-language-platform-matrix.mjs";

const repositoryProposalText = fs.readFileSync(agentLanguageProposalPath, "utf8");
const expectedOperationPaths = [
  ".github/workflows/workflow--test-scripts.yaml",
  "docs/proposals/README.md",
  "docs/proposals/agent-language-services-lifecycle-migration.md",
  "docs/proposals/agent-language-services-platform-matrix-closure.md",
  "docs/proposals/agent-language-services.md",
  "docs/reference/implemented-proposals/README.md",
];

test("accepts the repository platform matrix and registered references", () => {
  const result = validateAgentLanguagePlatformMatrix({ text: repositoryProposalText });

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects missing, duplicate, reordered, and nonliteral client-platform rows", () => {
  assertIncludesError(
    validateText(repositoryProposalText.replace("| claude-code | x86_64-unknown-linux-gnu |\n", "")),
    "restore exactly 2 data rows",
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace("| claude-code | x86_64-unknown-linux-gnu |", "| codex | x86_64-unknown-linux-gnu |")),
    "remove duplicate key",
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace(matrixRows(), matrixRows().split("\n").reverse().join("\n"))),
    "restore codex/x86_64-unknown-linux-gnu",
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace("| codex | x86_64-unknown-linux-gnu |", "| codex | all supported platforms |")),
    "ranges, wildcards, placeholders, and catch-all rows",
  );
});

test("rejects displaced, hidden, and duplicated matrix tables", () => {
  assertIncludesError(
    validateText(repositoryProposalText.replace("### Closed Client-Platform Matrix", "### Matrix")),
    "restore the unique level-3 heading",
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace("### Closed Client-Platform Matrix", "### Closed Client-Platform Matrix\n\n### Closed Client-Platform Matrix")),
    "remove duplicate level-3 heading",
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace("| Client | Platform |", "```markdown\n| Client | Platform |")),
    "add the literal Client/Platform table",
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace(matrixRows(), `> ${matrixRows().replaceAll("\n", "\n> ")}`)),
    "restore exactly 2 data rows",
  );
});

test("rejects compatibility field identity mutations and values", () => {
  assertIncludesError(
    validateText(repositoryProposalText.replace("| validator-integrity |\n", "")),
    `restore exactly ${expectedCompatibilityFields.length} field rows`,
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace("| mcp-contract |\n| lsp-contract |", "| lsp-contract |\n| mcp-contract |")),
    'restore "mcp-contract"',
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace("| reference-schema-contract |", "| reference-schema-contract | v1 |")),
    "remove compatibility values",
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace("| validator-version |", "| validator-version: 1.0 |")),
    "remove value-looking text",
  );
});

test("rejects hidden, mistitled, and wrong-destination reference links", () => {
  const reference = expectedReferenceLinks[0];
  const required = `[Closed Client-Platform Matrix](#closed-client-platform-matrix "matrix-ref:${reference.id}")`;
  assertIncludesError(
    validateText(repositoryProposalText.replace(required, "")),
    `${reference.id}: add`,
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace(required, `[Closed Client-Platform Matrix](#closed-client-platform-matrix "matrix-ref:wrong")`)),
    `${reference.id}: add`,
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace(required, `[Closed Client-Platform Matrix](#other "matrix-ref:${reference.id}")`)),
    `${reference.id}: add`,
  );
  assertIncludesError(
    validateText(repositoryProposalText.replace(required, `![Closed Client-Platform Matrix](#closed-client-platform-matrix "matrix-ref:${reference.id}")`)),
    `${reference.id}: add`,
  );
});

test("keeps GitHub annotation punctuation readable", () => {
  assert.equal(
    renderGitHubErrorAnnotation("docs/example.md:4: restore matrix\nnext"),
    "::error title=Invalid agent language platform matrix::docs/example.md:4: restore matrix%0Anext",
  );
});

test("accepts the exact closure diff transition", () => {
  using fixture = tempRepo("agent-language-platform-transition");
  const base = fixture.commitBaseWithoutClosure();
  fixture.writeAllowedClosureFiles();
  const head = fixture.commit("feat: close matrix");

  assert.deepEqual(validateClosureDiffScope({ repoRoot: fixture.root, baseSha: base, headSha: head }), []);
});

test("rejects extra paths and unrelated CI remediation during the closure transition", () => {
  using fixture = tempRepo("agent-language-platform-extra-path");
  const base = fixture.commitBaseWithoutClosure();
  fixture.writeAllowedClosureFiles();
  fixture.write("docs/proposals/unrelated.md", "---\nrole: proposal\nupdate-when: Its scope changes.\n---\n\n# Unrelated\n");
  const head = fixture.commit("feat: close matrix with extra path");

  assertIncludesError(
    { errors: validateClosureDiffScope({ repoRoot: fixture.root, baseSha: base, headSha: head }) },
    "docs/proposals/unrelated.md: remove this extra path",
  );
});

test("retires the closure allowlist after the matrix exists in the base", () => {
  using fixture = tempRepo("agent-language-platform-retired");
  fixture.commitBaseWithoutClosure();
  fixture.writeAllowedClosureFiles();
  const closed = fixture.commit("feat: close matrix");
  fixture.write("docs/proposals/unrelated.md", "---\nrole: proposal\nupdate-when: Its scope changes.\n---\n\n# Unrelated\n");
  const head = fixture.commit("docs: add unrelated proposal");

  assert.deepEqual(validateClosureDiffScope({ repoRoot: fixture.root, baseSha: closed, headSha: head }), []);
});

test("rejects protected-path renames and Git type changes during the closure transition", () => {
  using renameFixture = tempRepo("agent-language-platform-rename");
  const renameBase = renameFixture.commitBaseWithoutClosure();
  renameFixture.writeAllowedClosureFiles();
  renameFixture.remove("docs/proposals/agent-language-services-lifecycle-migration.md");
  renameFixture.write("docs/proposals/renamed-lifecycle.md", "renamed\n");
  const renameHead = renameFixture.commit("feat: close matrix with rename");
  assertIncludesError(
    { errors: validateClosureDiffScope({ repoRoot: renameFixture.root, baseSha: renameBase, headSha: renameHead }) },
    "docs/proposals/renamed-lifecycle.md: remove this extra path",
  );

  using typeFixture = tempRepo("agent-language-platform-type");
  const typeBase = typeFixture.commitBaseWithoutClosure();
  typeFixture.writeAllowedClosureFiles();
  typeFixture.remove("docs/reference/implemented-proposals/README.md");
  typeFixture.symlink("../README.md", "docs/reference/implemented-proposals/README.md");
  const typeHead = typeFixture.commit("feat: close matrix with type change");
  assertIncludesError(
    { errors: validateClosureDiffScope({ repoRoot: typeFixture.root, baseSha: typeBase, headSha: typeHead }) },
    "restore operation M",
  );
});

function validateText(text) {
  return validateAgentLanguagePlatformMatrix({ text });
}

function matrixRows() {
  return expectedClientPlatformRows.map(([client, platform]) => `| ${client} | ${platform} |`).join("\n");
}

function assertIncludesError(result, expectedSubstring) {
  assert.equal(
    result.errors.some((error) => error.includes(expectedSubstring)),
    true,
    `expected one error to include ${JSON.stringify(expectedSubstring)} in:\n${result.errors.join("\n")}`,
  );
}

function tempRepo(name) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-`));
  return {
    root,
    git(...args) {
      const result = spawnSync("git", args, { cwd: root, encoding: "utf8" });
      assert.equal(result.status, 0, result.stderr);
      return result.stdout.trim();
    },
    write(relativePath, text) {
      const target = path.join(root, relativePath);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, text);
    },
    remove(relativePath) {
      fs.rmSync(path.join(root, relativePath), { force: true, recursive: true });
    },
    symlink(target, relativePath) {
      const link = path.join(root, relativePath);
      fs.mkdirSync(path.dirname(link), { recursive: true });
      fs.symlinkSync(target, link);
    },
    commit(message) {
      this.git("add", "-A");
      this.git("commit", "-m", message);
      return this.git("rev-parse", "HEAD");
    },
    commitBaseWithoutClosure() {
      this.git("init");
      this.git("config", "user.email", "agent@example.test");
      this.git("config", "user.name", "Agent");
      this.write("docs/proposals/agent-language-services.md", baseProposalWithoutClosure());
      for (const file of expectedOperationPaths) {
        if (file !== agentLanguageProposalPath) {
          this.write(file, `${file}\n`);
        }
      }
      return this.commit("docs: add base");
    },
    writeAllowedClosureFiles() {
      this.write(agentLanguageProposalPath, repositoryProposalText);
      this.write(".github/workflows/workflow--test-scripts.yaml", "workflow\nchanged\n");
      this.write("docs/proposals/README.md", "proposal index\nchanged\n");
      this.write("docs/proposals/agent-language-services-lifecycle-migration.md", "lifecycle\nchanged\n");
      this.remove("docs/proposals/agent-language-services-platform-matrix-closure.md");
      this.write("docs/reference/implemented-proposals/README.md", "implemented index\nchanged\n");
      this.write("docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md", "implemented record\n");
      this.write("workflow-scripts/check-agent-language-platform-matrix.mjs", "validator\n");
      this.write("workflow-scripts/check-agent-language-platform-matrix.test.mjs", "tests\n");
    },
    [Symbol.dispose]() {
      fs.rmSync(root, { force: true, recursive: true });
    },
  };
}

function baseProposalWithoutClosure() {
  return repositoryProposalText
    .replace(/### Closed Client-Platform Matrix[\s\S]*?#### Compatibility Field Identities[\s\S]*?(?=\n## Safety And Privacy)/, "")
    .replaceAll(/ row in the\n\[Closed Client-Platform Matrix\]\(#closed-client-platform-matrix "matrix-ref:[^)]+?\)/g, " supported client-platform cell")
    .replaceAll(/\[Closed Client-Platform Matrix\]\(#closed-client-platform-matrix "matrix-ref:[^)]+?\)/g, "supported platforms");
}
