import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  inspectPhase,
  validateDocuments,
  validateRange,
  validateRepository,
  validateWorkflow,
} from "./check-agent-language-platform-matrix.mjs";

const agentPath = "docs/proposals/agent-language-services.md";
const lifecyclePath = "docs/proposals/agent-language-services-lifecycle-migration.md";

const expectedCaseIds = [
  "E01",
  "L00",
  "L01",
  "L04",
  "I01",
  "I08",
  "H01C01",
  "H04C05",
  "K01",
  "K03",
  "K05",
  "K10",
  "K12",
  "K14",
  "K16",
  "K18",
  "K20",
  "F01M",
  "F01E",
  "F01R",
  "F01W",
  "F01P",
  "F01G",
  "F01D",
  "F01U",
  "F01V",
  "FREV",
  "R01M01",
  "R01M02",
  "R01M15",
  "R01M16",
  "R01M17",
  "R01M18",
  "R01M19",
  "P01P01",
  "X-I09",
  "X-U01",
  "X-U03",
  "X-U07",
  "X-CRLF",
  "X-PIPE",
  "S00-S00",
  "S00-S01",
  "S01-S00",
  "S01-S01",
  "S03-S01",
  "T00",
  "T01",
  "T10",
  "T11",
  "T12",
  "T24",
  "T25",
  "T26",
  "T27",
  "T28",
  "W00",
  "W01",
  "W03",
  "W07",
  "W08",
  "W10",
  "W12",
  "W17",
  "W22",
  "W23",
  "W24",
  "W25",
  "W26",
  "W27",
  "D01",
  "D02",
  "D03",
];

const generatedCaseIds = [
  "E01",
  ...["L00", "L01", "L04"],
  ...["I01", "I08"],
  ...["H01C01", "H04C05"],
  ...["K01", "K03", "K05", "K10", "K12", "K14", "K16", "K18", "K20"],
  ...["F01M", "F01E", "F01R", "F01W", "F01P", "F01G", "F01D", "F01U", "F01V", "FREV"],
  ...["R01M01", "R01M02", "R01M15", "R01M16", "R01M17", "R01M18", "R01M19", "P01P01"],
  ...["X-I09", "X-U01", "X-U03", "X-U07", "X-CRLF", "X-PIPE"],
  ...["S00-S00", "S00-S01", "S01-S00", "S01-S01", "S03-S01"],
  ...["T00", "T01", "T10", "T11", "T12", "T24", "T25", "T26", "T27", "T28"],
  ...["W00", "W01", "W03", "W07", "W08", "W10", "W12", "W17", "W22", "W23", "W24", "W25", "W26", "W27"],
  ...["D01", "D02", "D03"],
];

test("E01 repository matrix documents validate", () => {
  assert.deepEqual(generatedCaseIds, expectedCaseIds);
  const result = validateRepository({ repoRoot: process.cwd() });
  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("L and I cases reject displaced or malformed matrix layout", () => {
  const valid = repositoryDocuments();
  assertValid("L00", valid);
  assertRejects("L01", mutate(valid, (agent) => agent.replace("## Agent Plugin", "## Wrong Owner")), "move the closed matrix");
  assertRejects("L04", mutate(valid, (agent) => agent.replace("## Safety And Privacy", "### Later Subsection\n\nextra\n\n## Safety And Privacy")), "final Agent Plugin subsection");
  assertRejects("I01", mutate(valid, (agent) => agent.replace("Closed client-platform row count: `2`.", "```\nhidden\n```\n\nClosed client-platform row count: `2`.")), "unexpected fence");
  assertRejects("I08", mutate(valid, (agent) => agent.replace("#### Matrix Reference Registry", "| Extra |\n| --- |\n| `x` |\n\n#### Matrix Reference Registry")), "exactly five matrix blocks");
});

test("K cases reject missing duplicate reordered or nonliteral membership", () => {
  const valid = repositoryDocuments();
  assertRejects("K01", mutate(valid, (agent) => agent.replace(codexRow(), "")), "exactly 2 rows");
  assertRejects("K03", mutate(valid, (agent) => agent.replace("`claude-code` | `x86_64-unknown-linux-gnu`", "`codex` | `x86_64-unknown-linux-gnu`")), "duplicate");
  assertRejects("K05", mutate(valid, (agent) => agent.replace(`${codexRow()}${claudeRow()}`, `${claudeRow()}${codexRow()}`)), "restore codex");
  assertRejects("K10", mutate(valid, (agent) => agent.replace("`x86_64-unknown-linux-gnu`", "`x86_64-*`")), "ranges");
  assertRejects("K12", mutate(valid, (agent) => agent.replace("`codex`", "`*`")), "wildcards");
  assertRejects("K14", mutate(valid, (agent) => agent.replace("`codex`", "`TBD`")), "placeholders");
  assertRejects("K16", mutate(valid, (agent) => agent.replace("`codex`", "`all`")), "catch-alls");
  assertRejects("K18", mutate(valid, (agent) => agent.replace("`codex`", "`zed`")), "unexpected literals");
  assertRejects("K20", mutate(valid, (agent) => agent.replace("| `codex` |", "| codex |")), "exact inline-code cells");
});

test("F cases reject missing malformed or nonliteral compatibility values", () => {
  const valid = repositoryDocuments();
  assertRejects("F01M", mutate(valid, (agent) => agent.replace(" | validator-version |", "")), "headers");
  assertRejects("F01E", mutate(valid, (agent) => agent.replace("`linux-x86_64-host-contract-v1`", "``")), "nonempty exact literal");
  assertRejects("F01R", mutate(valid, (agent) => agent.replace("`mcp-contract-v1`", "`mcp-contract-v1..v2`")), "ranges");
  assertRejects("F01W", mutate(valid, (agent) => agent.replace("`mcp-contract-v1`", "`mcp-contract-*`")), "wildcards");
  assertRejects("F01P", mutate(valid, (agent) => agent.replace("`mcp-contract-v1`", "`TBD`")), "placeholders");
  assertRejects("F01G", mutate(valid, (agent) => agent.replace("`2f5c36e1d4a9b8c7e0f123456789abcd2f5c36e1d4a9b8c7e0f123456789abcd`", "`2f5c36e1`")), "64 lowercase hexadecimal");
  assertRejects("F01D", mutate(valid, (agent) => agent.replace("`claude-code` | `x86_64-unknown-linux-gnu`", "`codex` | `x86_64-unknown-linux-gnu`")), "duplicate");
  assertRejects("F01U", mutate(valid, (agent) => agent.replace("manifest-schema", "manifest")), "headers");
  assertRejects("F01V", mutate(valid, (agent) => agent.replace("`codex-plugin-manifest-v1`", "`codex-plugin-manifest-v2`")), "restore codex-plugin-manifest-v1");
  assertRejects("FREV", mutate(valid, reverseCompatibilityColumns), "headers");
});

test("R and P cases reject missing duplicate wrong or unregistered references", () => {
  const valid = repositoryDocuments();
  const source = "[Closed Client-Platform Matrix](#closed-client-platform-matrix \"matrix-ref:agent-plugin-server-lifecycle\")";
  assertRejects("R01M01", mutate(valid, (agent) => agent.replace(source, "Closed Client-Platform Matrix")), "restore exactly one");
  assertRejects("R01M02", mutate(valid, (agent) => agent.replace(source, `${source} ${source}`)), "restore exactly one");
  assertRejects("R01M15", mutate(valid, (agent) => agent.replace(source, source.replace("Closed Client-Platform Matrix", "Closed Matrix"))), "restore exact link source");
  assertRejects("R01M16", mutate(valid, (agent) => agent.replace(source, source.replace("#closed-client-platform-matrix", "#wrong"))), "restore exact link source");
  assertRejects("R01M17", mutate(valid, (agent) => agent.replace(source, source.replace("agent-plugin-server-lifecycle", "wrong"))), "reference wrong");
  assertRejects("R01M18", mutate(valid, (agent) => agent.replace("## Safety And Privacy", "[Closed Client-Platform Matrix](#closed-client-platform-matrix \"matrix-ref:extra\")\n\n## Safety And Privacy")), "unregistered");
  assertRejects("R01M19", mutate(valid, (agent) => agent.replace("These shared instructions are not a client-platform membership source.", "These shared instructions apply to all supported platforms.")), "unbound supported-platform phrase");
  assertRejects("P01P01", mutate(valid, (agent) => agent.replace(source, "Closed Client-Platform Matrix").replace("| `agent-plugin-server-lifecycle` | `agent-language-services.md` | `## Agent Plugin` | paragraph 3 | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |\n", "")), "restore exactly one");
});

test("H, X, and S cases reject hidden evidence and classify phase state", () => {
  const valid = repositoryDocuments();
  assertRejects("H01C01", mutate(valid, (agent) => agent.replace("| `codex` | `x86_64-unknown-linux-gnu` | `linux-x86_64-host-contract-v1` | `codex-plugin-manifest-v1` | `agent-platform-matrix-validator-v1` | `2f5c36e1d4a9b8c7e0f123456789abcd2f5c36e1d4a9b8c7e0f123456789abcd` | `veln-toolchain-contract-v1` | `mcp-contract-v1` | `codex-lsp-disabled-contract-v1` | `language-service-contract-v1` | `reference-schema-contract-v1` |", "<!-- | `codex` | `x86_64-unknown-linux-gnu` | -->")), "unexpected html");
  assertRejects("H04C05", mutate(valid, (agent) => agent.replace("Matrix closure phase: `agent-language-services-platform-matrix-closed`.", "> Matrix closure phase: `agent-language-services-platform-matrix-closed`.")), "unexpected blockquote");
  assert.equal(inspectPhase(valid.get("agent-language-services.md")), "present");
  assert.equal(inspectPhase(valid.get("agent-language-services.md").replace("Matrix closure phase: `agent-language-services-platform-matrix-closed`.", "")), "invalid");
  assert.equal(inspectPhase(valid.get("agent-language-services.md").replace("### Closed Client-Platform Matrix", "### Open Client-Platform Matrix")), "absent");
});

test("W cases validate the exact workflow registration", () => {
  const workflow = fs.readFileSync(".github/workflows/workflow--test-scripts.yaml", "utf8");
  assert.deepEqual(validateWorkflow(workflow), []);
  assertWorkflowRejects("W01", workflow.replace("      - name: Validate the closed agent language platform matrix\n", ""), "exactly one step");
  assertWorkflowRejects("W03", workflow.replace("test-workflow-scripts:", "other-job:"), "test-workflow-scripts");
  assertWorkflowRejects("W07", workflow.replace("node workflow-scripts/check-agent-language-platform-matrix.mjs", "node wrong.mjs"), "production contract");
  assertWorkflowRejects("W08", workflow.replace("          AGENT_PLATFORM_MATRIX_BASE_SHA: ${{ github.event.pull_request.base.sha || github.event.before }}\n", ""), "BASE_SHA");
  assertWorkflowRejects("W10", workflow.replace("          AGENT_PLATFORM_MATRIX_HEAD_SHA: ${{ github.sha }}\n", ""), "HEAD_SHA");
  assertWorkflowRejects("W12", workflow.replace("        run: node workflow-scripts/check-agent-language-platform-matrix.mjs", "        if: ${{ false }}\n        run: node workflow-scripts/check-agent-language-platform-matrix.mjs"), "remove if");
  assertWorkflowRejects("W17", workflow.replace("  pull_request:", "  pull_request_removed:"), "pull_request");
});

test("T and W range cases validate phase-aware closure guard", () => {
  using repo = tempGitRepo();
  const base = repo.commitBase();
  repo.writeClosureFiles();
  const head = repo.commit("closure");
  assert.deepEqual(validateRange({ repoRoot: repo.root, baseSha: base, headSha: head }), []);
  assert.equal(validateRange({ repoRoot: repo.root }).length, 0);
  assertRejectsRange("W22", repo.root, undefined, head, "set AGENT_PLATFORM_MATRIX_BASE_SHA");
  assertRejectsRange("W23", repo.root, "0000000000000000000000000000000000000000", head, "all-zero");
  assertRejectsRange("W24", repo.root, "0123456789012345678901234567890123456789", head, "not readable");

  using extraRepo = tempGitRepo();
  const extraBase = extraRepo.commitBase();
  extraRepo.writeClosureFiles();
  extraRepo.write("docs/extra.md", "extra\n");
  const extraHead = extraRepo.commit("extra");
  assertRejectsRange("T10", extraRepo.root, extraBase, extraHead, "docs/extra.md");

  using renameRepo = tempGitRepo();
  const renameBase = renameRepo.commitBase();
  renameRepo.writeClosureFiles();
  renameRepo.rename("docs/proposals/README.md", "docs/proposals/README-renamed.md");
  const renameHead = renameRepo.commit("rename");
  assertRejectsRange("T11", renameRepo.root, renameBase, renameHead, "rename, copy, or wrong-status");

  using modeRepo = tempGitRepo();
  const modeBase = modeRepo.commitBase();
  modeRepo.writeClosureFiles();
  modeRepo.executable("docs/proposals/agent-language-services.md");
  const modeHead = modeRepo.commit("mode");
  assertRejectsRange("T12", modeRepo.root, modeBase, modeHead, "Git type");

  using postRepo = tempGitRepo();
  postRepo.commitBase();
  postRepo.writeClosureFiles();
  const closed = postRepo.commit("closure");
  postRepo.write("docs/unrelated.md", "later\n");
  const later = postRepo.commit("later");
  assert.deepEqual(validateRange({ repoRoot: postRepo.root, baseSha: closed, headSha: later }), []);
});

function repositoryDocuments() {
  return new Map([
    ["agent-language-services.md", fs.readFileSync(agentPath, "utf8")],
    ["agent-language-services-lifecycle-migration.md", fs.readFileSync(lifecyclePath, "utf8")],
  ]);
}

function mutate(documents, editAgent, editLifecycle = (text) => text) {
  return new Map([
    ["agent-language-services.md", editAgent(documents.get("agent-language-services.md"))],
    ["agent-language-services-lifecycle-migration.md", editLifecycle(documents.get("agent-language-services-lifecycle-migration.md"))],
  ]);
}

function assertValid(id, documents) {
  const result = validateDocuments(documents);
  assert.deepEqual(result, [], id);
}

function assertRejects(id, documents, needle) {
  const result = validateDocuments(documents);
  assert.ok(result.some((error) => error.includes(needle)), `${id}: ${result.join("\n")}`);
}

function assertWorkflowRejects(id, workflow, needle) {
  const result = validateWorkflow(workflow);
  assert.ok(result.some((error) => error.includes(needle)), `${id}: ${result.join("\n")}`);
}

function assertRejectsRange(id, repoRoot, baseSha, headSha, needle) {
  const result = validateRange({ repoRoot, baseSha, headSha });
  assert.ok(result.some((error) => error.includes(needle)), `${id}: ${result.join("\n")}`);
}

function codexRow() {
  return "| `codex` | `x86_64-unknown-linux-gnu` | `linux-x86_64-host-contract-v1` | `codex-plugin-manifest-v1` | `agent-platform-matrix-validator-v1` | `2f5c36e1d4a9b8c7e0f123456789abcd2f5c36e1d4a9b8c7e0f123456789abcd` | `veln-toolchain-contract-v1` | `mcp-contract-v1` | `codex-lsp-disabled-contract-v1` | `language-service-contract-v1` | `reference-schema-contract-v1` |\n";
}

function claudeRow() {
  return "| `claude-code` | `x86_64-unknown-linux-gnu` | `linux-x86_64-host-contract-v1` | `claude-code-plugin-manifest-v1` | `agent-platform-matrix-validator-v1` | `7a4e29c0b1d3f85690abcdef123456787a4e29c0b1d3f85690abcdef12345678` | `veln-toolchain-contract-v1` | `mcp-contract-v1` | `lsp-contract-v1` | `language-service-contract-v1` | `reference-schema-contract-v1` |\n";
}

function reverseCompatibilityColumns(agent) {
  return agent.replace(
    "| client | platform | host-build | manifest-schema | validator-version | validator-integrity | veln-contract | mcp-contract | lsp-contract | language-service-contract | reference-schema-contract |",
    "| reference-schema-contract | language-service-contract | lsp-contract | mcp-contract | veln-contract | validator-integrity | validator-version | manifest-schema | host-build | platform | client |",
  );
}

function tempGitRepo() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "agent-platform-matrix-"));
  runGit(root, ["init"]);
  runGit(root, ["config", "user.email", "test@example.invalid"]);
  runGit(root, ["config", "user.name", "Test User"]);
  return {
    root,
    [Symbol.dispose]() {
      fs.rmSync(root, { force: true, recursive: true });
    },
    write(file, text) {
      fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
      fs.writeFileSync(path.join(root, file), text);
    },
    remove(file) {
      fs.rmSync(path.join(root, file), { force: true });
    },
    rename(from, to) {
      fs.mkdirSync(path.dirname(path.join(root, to)), { recursive: true });
      fs.renameSync(path.join(root, from), path.join(root, to));
    },
    executable(file) {
      fs.chmodSync(path.join(root, file), 0o755);
    },
    commit(message) {
      runGit(root, ["add", "-A"]);
      runGit(root, ["commit", "-m", message]);
      return runGit(root, ["rev-parse", "HEAD"]).stdout.trim();
    },
    commitBase() {
      this.write(".github/workflows/workflow--test-scripts.yaml", "base\n");
      this.write("docs/proposals/README.md", "base\n");
      this.write("docs/proposals/agent-language-services-lifecycle-migration.md", "base\n");
      this.write("docs/proposals/agent-language-services-platform-matrix-closure.md", "base\n");
      this.write("docs/proposals/agent-language-services.md", "# Agent Language Services\n\n## Agent Plugin\n\nbase\n");
      this.write("docs/reference/implemented-proposals/README.md", "base\n");
      return this.commit("base");
    },
    writeClosureFiles() {
      this.write(".github/workflows/workflow--test-scripts.yaml", "head\n");
      this.write("docs/proposals/README.md", "head\n");
      this.write("docs/proposals/agent-language-services-lifecycle-migration.md", "head\n");
      this.remove("docs/proposals/agent-language-services-platform-matrix-closure.md");
      this.write("docs/proposals/agent-language-services.md", minimalClosedAgentDoc());
      this.write("docs/reference/implemented-proposals/README.md", "head\n");
      this.write("docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md", "head\n");
      this.write("workflow-scripts/check-agent-language-platform-matrix.mjs", "head\n");
      this.write("workflow-scripts/check-agent-language-platform-matrix.test.mjs", "head\n");
    },
  };
}

function minimalClosedAgentDoc() {
  return [
    "# Agent Language Services",
    "",
    "## Agent Plugin",
    "",
    "p1",
    "",
    "p2",
    "",
    "p3",
    "",
    "p4",
    "",
    "p5",
    "",
    "p6",
    "",
    "p7",
    "",
    "p8",
    "",
    "p9",
    "",
    "### Closed Client-Platform Matrix",
    "",
    "Closed client-platform row count: `2`.",
    "",
    "Matrix closure phase: `agent-language-services-platform-matrix-closed`.",
    "",
    "| client | platform | host-build | manifest-schema | validator-version | validator-integrity | veln-contract | mcp-contract | lsp-contract | language-service-contract | reference-schema-contract |",
    "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    codexRow().trimEnd(),
    claudeRow().trimEnd(),
    "",
    "#### Matrix Reference Registry",
    "",
    "| Reference ID | Document | Heading | Block | Label | Destination |",
    "| --- | --- | --- | --- | --- | --- |",
    "| `agent-plugin-server-lifecycle` | `agent-language-services.md` | `## Agent Plugin` | paragraph 3 | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |",
    "",
    "## Safety And Privacy",
    "",
  ].join("\n");
}

function runGit(cwd, args) {
  const result = spawnSync("git", args, { cwd, encoding: "utf8" });
  assert.equal(result.status, 0, `git ${args.join(" ")}\n${result.stderr}`);
  return result;
}
