import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  validateClosureRange,
  validateMatrixDocument,
  validateMatrixReferences,
  validatePathOperations,
  validateWorkflowRegistration,
} from "./check-agent-language-platform-matrix.mjs";

const proposalFile = "docs/proposals/agent-language-services.md";
const lifecycleFile = "docs/proposals/agent-language-services-lifecycle-migration.md";
const keys = [
  ["codex", "x86_64-unknown-linux-gnu"],
  ["claude-code", "x86_64-unknown-linux-gnu"],
];
const fields = [
  "client",
  "platform",
  "host-build",
  "manifest-schema",
  "validator-version",
  "validator-integrity",
  "veln-contract",
  "mcp-contract",
  "lsp-contract",
  "language-service-contract",
  "reference-schema-contract",
];
const refs = [
  ["agent-plugin-server-lifecycle", proposalFile, "## Agent Plugin", "paragraph 3", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["agent-plugin-installer-boundary", proposalFile, "## Agent Plugin", "paragraph 7", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["agent-plugin-compatibility-authority", proposalFile, "## Agent Plugin", "paragraph 8", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["agent-plugin-native-validation", proposalFile, "## Agent Plugin", "paragraph 9", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["conformance-requirement-coverage", proposalFile, "## Conformance Contract", "paragraph 1", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["conformance-capability-membership", proposalFile, "## Conformance Contract", "paragraph 2", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["q21-plugin-matrix", proposalFile, "## Conformance Contract", "table row `Q21 plugin matrix`", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["q22-gate-totality", proposalFile, "## Conformance Contract", "table row `Q22 gate totality`", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["umbrella-completion", proposalFile, "## Conformance Contract", "paragraph 5", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["plugin-acceptance-completion", proposalFile, "### Plugin", "table row `Run the proposal completion gate`", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["lifecycle-conformance-cells", lifecycleFile, "## Preserved Finite Inputs", "list item 6", "Closed Client-Platform Matrix", "agent-language-services.md#closed-client-platform-matrix"],
  ["lifecycle-compatibility-cells", lifecycleFile, "## Preserved Finite Inputs", "list item 7", "Closed Client-Platform Matrix", "agent-language-services.md#closed-client-platform-matrix"],
  ["lifecycle-prerequisite-acceptance", lifecycleFile, "## Acceptance Model", "table row `Close the prerequisite client-platform set`", "Closed Client-Platform Matrix", "agent-language-services.md#closed-client-platform-matrix"],
];

function matrixSection(overrides = {}) {
  const rows = overrides.rows ?? keys.map(([client, platform]) => `| \`${client}\` | \`${platform}\` |`);
  const fieldRows = overrides.fields ?? fields.map((field) => `| \`${field}\` |`);
  const registryRows = overrides.refs ?? refs.map((row) => `| \`${row[0]}\` | \`${path.basename(row[1])}\` | \`${row[2]}\` | ${row[3]} | \`${row[4]}\` | \`${row[5]}\` |`);
  return [
    "### Closed Client-Platform Matrix",
    "",
    overrides.rowCount ?? "Closed client-platform row count: `2`.",
    "",
    overrides.phase ?? "Matrix closure phase: `agent-language-services-platform-matrix-closed`.",
    "",
    "| Client | Platform |",
    overrides.membershipDelimiter ?? "| --- | --- |",
    ...rows,
    "",
    "#### Compatibility Field Identities",
    "",
    "| Compatibility field |",
    overrides.fieldDelimiter ?? "| --- |",
    ...fieldRows,
    "",
    "#### Matrix Reference Registry",
    "",
    "| Reference ID | Document | Heading | Block | Label | Destination |",
    overrides.registryDelimiter ?? "| --- | --- | --- | --- | --- | --- |",
    ...registryRows,
  ].join("\n");
}

function link(row) {
  return `[${row[4]}](${row[5]} "matrix-ref:${row[0]}")`;
}

function proposalDocument(options = {}) {
  return [
    "# Agent Language Services",
    "",
    "## Agent Plugin",
    "",
    "One plugin source may contain client-specific manifests and shared components:",
    "",
    "```text",
    "plugins/veln/",
    "```",
    "",
    "Client startup validates workspace roots.",
    "",
    `Server lifecycle uses ${link(refs[0])}.`,
    "",
    "An explicit client executable setting takes precedence.",
    "",
    "The shared skill instructs agents to:",
    "",
    "- search the published reference.",
    "",
    "The first capability supplies validated plugin artifacts.",
    "",
    `Installer boundary uses ${link(refs[1])}.`,
    "",
    `Compatibility authority uses ${link(refs[2])}.`,
    "",
    `Native validation uses ${link(refs[3])}.`,
    "",
    options.matrix ?? matrixSection(options.matrixOptions),
    "",
    "## Safety And Privacy",
    "",
    "## Conformance Contract",
    "",
    `Coverage uses ${link(refs[4])}.`,
    "",
    `Membership uses ${link(refs[5])}.`,
    "",
    "| Decision | Required evidence |",
    "| --- | --- |",
    `| Q21 plugin matrix | ${link(refs[6])} |`,
    `| Q22 gate totality | ${link(refs[7])} |`,
    "",
    "Q11 is implemented.",
    "",
    "The gate also covers request bounds.",
    "",
    `Completion uses ${link(refs[8])}.`,
    "",
    "## Acceptance Model",
    "",
    "### Plugin",
    "",
    "| Case | Expected result | Planned evidence |",
    "| --- | --- | --- |",
    `| Run the proposal completion gate | ${link(refs[9])} | Q22 |`,
  ].join("\n");
}

function lifecycleDocument(options = {}) {
  return [
    "# Agent Language Services Lifecycle Migration",
    "",
    "## Preserved Finite Inputs",
    "",
    "- row one",
    "- row two",
    "- row three",
    "- row four",
    "- row five",
    `- conformance cells ${link(refs[10])}`,
    `- compatibility cells ${link(refs[11])}`,
    "",
    "## Acceptance Model",
    "",
    "| Case | Expected result | Planned evidence |",
    "| --- | --- | --- |",
    `| Close the prerequisite client-platform set | ${options.prerequisite ?? link(refs[12])} | record |`,
  ].join("\n");
}

test("accepts the independent minimal closed matrix contract", () => {
  assert.deepEqual(validateMatrixDocument(proposalDocument()), []);
  assert.deepEqual(validateMatrixReferences(new Map([
    [proposalFile, proposalDocument()],
    [lifecycleFile, lifecycleDocument()],
  ])), []);
});

test("rejects matrix membership, field, registry, and row-count mutations", () => {
  const cases = [
    ["missing heading", proposalDocument({ matrix: matrixSection().replace("### Closed Client-Platform Matrix", "### Matrix") }), /exactly one/],
    ["duplicate heading", proposalDocument({ matrix: `${matrixSection()}\n\n### Closed Client-Platform Matrix` }), /exactly one|block order/],
    ["bad delimiter", proposalDocument({ matrixOptions: { membershipDelimiter: "| --- |" } }), /delimiter/],
    ["missing key", proposalDocument({ matrixOptions: { rows: ["| `codex` | `x86_64-unknown-linux-gnu` |"] } }), /missing client-platform key/],
    ["duplicate key", proposalDocument({ matrixOptions: { rows: ["| `codex` | `x86_64-unknown-linux-gnu` |", "| `codex` | `x86_64-unknown-linux-gnu` |"] } }), /duplicate client-platform key|restore client-platform key/],
    ["reordered key", proposalDocument({ matrixOptions: { rows: [...keys].reverse().map(([client, platform]) => `| \`${client}\` | \`${platform}\` |`) } }), /restore client-platform key/],
    ["empty key", proposalDocument({ matrixOptions: { rows: ["| `` | `x86_64-unknown-linux-gnu` |", "| `claude-code` | `x86_64-unknown-linux-gnu` |"] } }), /nonempty client-platform key|exact inline-code client and platform literals/],
    ["range key", proposalDocument({ matrixOptions: { rows: ["| `codex` | `x86_64-*` |", "| `claude-code` | `x86_64-unknown-linux-gnu` |"] } }), /restore client-platform key|ranges/],
    ["wildcard key", proposalDocument({ matrixOptions: { rows: ["| `codex` | `*` |", "| `claude-code` | `x86_64-unknown-linux-gnu` |"] } }), /restore client-platform key/],
    ["catch-all key", proposalDocument({ matrixOptions: { rows: ["| `all` | `supported-platforms` |", "| `claude-code` | `x86_64-unknown-linux-gnu` |"] } }), /restore client-platform key/],
    ["placeholder key", proposalDocument({ matrixOptions: { rows: ["| `todo` | `x86_64-unknown-linux-gnu` |", "| `claude-code` | `x86_64-unknown-linux-gnu` |"] } }), /restore client-platform key/],
    ["bad row count", proposalDocument({ matrixOptions: { rowCount: "Closed client-platform row count: `3`." } }), /row count/],
    ["missing phase", proposalDocument({ matrixOptions: { phase: "" } }), /phase identity|block order/],
    ["duplicate phase", proposalDocument({ matrixOptions: { phase: "Matrix closure phase: `agent-language-services-platform-matrix-closed`.\n\nMatrix closure phase: `agent-language-services-platform-matrix-closed`." } }), /block order/],
    ["mutated phase identity", proposalDocument({ matrixOptions: { phase: "Matrix closure phase: `agent-language-services-platform-matrix-open`." } }), /phase identity/],
    ["missing field", proposalDocument({ matrixOptions: { fields: fields.slice(1).map((field) => `| \`${field}\` |`) } }), /compatibility field/],
    ["duplicate field", proposalDocument({ matrixOptions: { fields: ["client", ...fields].map((field) => `| \`${field}\` |`) } }), /duplicate compatibility field|restore compatibility field/],
    ["reordered field", proposalDocument({ matrixOptions: { fields: [...fields].reverse().map((field) => `| \`${field}\` |`) } }), /restore compatibility field/],
    ["unexpected field", proposalDocument({ matrixOptions: { fields: [...fields, "extra"].map((field) => `| \`${field}\` |`) } }), /unexpected compatibility field|remove/],
    ["value bearing field", proposalDocument({ matrixOptions: { fields: fields.map((field) => field === "client" ? "| `client`: codex |" : `| \`${field}\` |`) } }), /no value/],
    ["digest value", proposalDocument({ matrixOptions: { fields: fields.map((field) => field === "validator-integrity" ? "| `validator-integrity` = abc |" : `| \`${field}\` |`) } }), /no value|digest/],
    ["missing registry row", proposalDocument({ matrixOptions: { refs: refs.slice(1).map((row) => `| \`${row[0]}\` | \`${path.basename(row[1])}\` | \`${row[2]}\` | ${row[3]} | \`${row[4]}\` | \`${row[5]}\` |`) } }), /registry row/],
    ["duplicate registry row", proposalDocument({ matrixOptions: { refs: [refs[0], ...refs].map((row) => `| \`${row[0]}\` | \`${path.basename(row[1])}\` | \`${row[2]}\` | ${row[3]} | \`${row[4]}\` | \`${row[5]}\` |`) } }), /duplicate matrix reference registry row|restore matrix reference registry row/],
    ["reordered registry row", proposalDocument({ matrixOptions: { refs: [...refs].reverse().map((row) => `| \`${row[0]}\` | \`${path.basename(row[1])}\` | \`${row[2]}\` | ${row[3]} | \`${row[4]}\` | \`${row[5]}\` |`) } }), /registry row/],
    ["unexpected registry row", proposalDocument({ matrixOptions: { refs: [...refs, ["unexpected", proposalFile, "## Agent Plugin", "paragraph 10", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"]].map((row) => `| \`${row[0]}\` | \`${path.basename(row[1])}\` | \`${row[2]}\` | ${row[3]} | \`${row[4]}\` | \`${row[5]}\` |`) } }), /unexpected matrix reference registry row|remove/],
  ];
  for (const [name, input, pattern] of cases) {
    assert.match(validateMatrixDocument(input).join("\n"), pattern, name);
  }
});

test("rejects hidden and malformed source coincidences", () => {
  const hidden = [
    ["fence", ["```", matrixSection(), "```"].join("\n")],
    ["indented", matrixSection().split("\n").map((line) => `    ${line}`).join("\n")],
    ["quote", matrixSection().split("\n").map((line) => `> ${line}`).join("\n")],
    ["comment", `<!--\n${matrixSection()}\n-->`],
    ["html", `<div>\n${matrixSection()}\n</div>`],
  ];
  for (const [name, matrix] of hidden) {
    assert.match(validateMatrixDocument(proposalDocument({ matrix })).join("\n"), /exactly one/, name);
  }
  assert.match(validateMatrixDocument(proposalDocument({ matrix: `${matrixSection()}\n\n| Extra | Table |\n| --- | --- |` })).join("\n"), /block order/);
  assert.match(validateMatrixDocument(proposalDocument({ matrixOptions: { fieldDelimiter: "| -- |" } })).join("\n"), /delimiter/);
});

test("rejects missing, duplicate, displaced, hidden, and wrong matrix links", () => {
  const validProposal = proposalDocument();
  const validLifecycle = lifecycleDocument();
  const titleSwap = validProposal
    .replace(link(refs[6]), "TITLE_SWAP_Q21")
    .replace(link(refs[7]), link(refs[7]).replace("matrix-ref:q22-gate-totality", "matrix-ref:q21-plugin-matrix"))
    .replace("TITLE_SWAP_Q21", link(refs[6]).replace("matrix-ref:q21-plugin-matrix", "matrix-ref:q22-gate-totality"));
  const cases = [
    ["missing", validProposal.replace(link(refs[0]), "Closed Client-Platform Matrix"), /restore matrix reference/],
    ["duplicate", validProposal.replace(link(refs[0]), `${link(refs[0])} ${link(refs[0])}`), /restore matrix reference/],
    ["wrong title", validProposal.replace("matrix-ref:q21-plugin-matrix", "matrix-ref:q22-gate-totality"), /restore matrix reference/],
    ["wrong target", validProposal.replace("(#closed-client-platform-matrix \"matrix-ref:q21-plugin-matrix\")", "(#other \"matrix-ref:q21-plugin-matrix\")"), /restore matrix reference/],
    ["image alt text", validProposal.replace(link(refs[0]), `![${refs[0][4]}](${refs[0][5]} "matrix-ref:${refs[0][0]}")`), /restore matrix reference/],
    ["link destination coincidence", validProposal.replace(link(refs[0]), `[Other](#closed-client-platform-matrix "not-a-matrix-ref")`), /restore matrix reference/],
    ["link title coincidence", validProposal.replace(link(refs[0]), `[Other](#other "matrix-ref:${refs[0][0]}")`), /restore matrix reference/],
    ["hidden", validProposal.replace(link(refs[0]), `\`${link(refs[0])}\``), /restore matrix reference/],
    ["wrong section", validProposal.replace(link(refs[0]), "Server lifecycle uses moved.").replace("## Safety And Privacy", `## Safety And Privacy\n\nServer lifecycle uses ${link(refs[0])}.`), /restore matrix reference/],
    ["wrong block", validLifecycle.replace("- row five\n- conformance", `- row five\n- extra ${link(refs[10])}\n- conformance`), /restore matrix reference/],
    ["same-kind title swap", titleSwap, /restore matrix reference/],
    ["paired deletion", validProposal.replace(link(refs[6]), "Q21").replace("`q21-plugin-matrix`", "`q21-plugin-matrix-missing`"), /restore matrix reference|registry row/],
  ];
  for (const [name, proposalOrLifecycle, pattern] of cases) {
    const documents = name === "wrong block"
      ? new Map([[proposalFile, validProposal], [lifecycleFile, proposalOrLifecycle]])
      : new Map([[proposalFile, proposalOrLifecycle], [lifecycleFile, validLifecycle]]);
    assert.match(validateMatrixReferences(documents).join("\n"), pattern, name);
  }
});

test("checks closure path operations and protected type changes", () => {
  const valid = [
    ["M", "100644", "100644", ".github/workflows/workflow--test-scripts.yaml"],
    ["M", "100644", "100644", "docs/proposals/README.md"],
    ["M", "100644", "100644", "docs/proposals/agent-language-services-lifecycle-migration.md"],
    ["D", "100644", "000000", "docs/proposals/agent-language-services-platform-matrix-closure.md"],
    ["M", "100644", "100644", "docs/proposals/agent-language-services.md"],
    ["M", "100644", "100644", "docs/reference/implemented-proposals/README.md"],
    ["A", "000000", "100644", "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md"],
    ["A", "000000", "100644", "workflow-scripts/check-agent-language-platform-matrix.mjs"],
    ["A", "000000", "100644", "workflow-scripts/check-agent-language-platform-matrix.test.mjs"],
  ].map(([status, oldMode, newMode, file]) => ({ status, oldMode, newMode, path: file }));
  assert.deepEqual(validatePathOperations(valid), []);
  assert.match(validatePathOperations(valid.slice(1)).join("\n"), /required/);
  assert.match(validatePathOperations([...valid, { status: "M", oldMode: "100644", newMode: "100644", path: "docs/extra.md" }]).join("\n"), /remove this/);
  assert.match(validatePathOperations([{ ...valid[0], status: "R" }, ...valid.slice(1)]).join("\n"), /replace Git type/);
  assert.match(validatePathOperations([{ ...valid[0], newMode: "120000" }, ...valid.slice(1)]).join("\n"), /restore M 100644->100644/);
  assert.match(validatePathOperations([{ ...valid[1], path: "docs/proposals/README-renamed.md" }, ...valid.slice(2)]).join("\n"), /docs\/proposals\/README.md: restore required|README-renamed.md: remove this/);
});

test("rejects missing, all-zero, and unreadable closure ranges", () => {
  const repo = fs.mkdtempSync(path.join(fs.realpathSync.native("/tmp"), "platform-range-"));
  git(repo, "init");
  git(repo, "config", "user.name", "Veln Test");
  git(repo, "config", "user.email", "veln-test@example.invalid");
  write(repo, proposalFile, proposalDocument());
  git(repo, "add", ".");
  git(repo, "commit", "-m", "base");
  const head = rev(repo);

  assert.match(validateClosureRange({ repoRoot: repo }).join("\n"), /set AGENT_PLATFORM_MATRIX_BASE_SHA/);
  assert.match(validateClosureRange({ repoRoot: repo, baseSha: "0000000000000000000000000000000000000000", headSha: head }).join("\n"), /replace all-zero closure range revision/);
  assert.match(validateClosureRange({ repoRoot: repo, baseSha: "1111111111111111111111111111111111111111", headSha: head }).join("\n"), /cannot read|unable to read closure tree delta/);
});

test("validates the exact workflow step registration", () => {
  const repo = fs.mkdtempSync(path.join(fs.realpathSync.native("/tmp"), "platform-workflow-"));
  const workflow = [
    "name: workflow / test scripts",
    "jobs:",
    "  test-workflow-scripts:",
    "    steps:",
    "      - name: Validate the closed agent language platform matrix",
    "        env:",
    "          AGENT_PLATFORM_MATRIX_BASE_SHA: ${{ github.event.pull_request.base.sha || github.event.before }}",
    "          AGENT_PLATFORM_MATRIX_HEAD_SHA: ${{ github.sha }}",
    "        run: node workflow-scripts/check-agent-language-platform-matrix.mjs",
  ].join("\n");
  write(repo, ".github/workflows/workflow--test-scripts.yaml", workflow);
  assert.deepEqual(validateWorkflowRegistration({ repoRoot: repo }), []);
  write(repo, ".github/workflows/workflow--test-scripts.yaml", workflow.replace("github.sha", "github.event.after"));
  assert.match(validateWorkflowRegistration({ repoRoot: repo }).join("\n"), /base\/head/);
});

test("activates closure guard once and retires on later documentation changes", () => {
  const repo = fs.mkdtempSync(path.join(fs.realpathSync.native("/tmp"), "platform-matrix-"));
  git(repo, "init");
  git(repo, "config", "user.name", "Veln Test");
  git(repo, "config", "user.email", "veln-test@example.invalid");
  write(repo, proposalFile, "# Agent Language Services\n\n## Agent Plugin\n\nno phase\n");
  git(repo, "add", ".");
  git(repo, "commit", "-m", "base");
  const base = rev(repo);

  write(repo, ".github/workflows/workflow--test-scripts.yaml", "name: test\n");
  write(repo, "docs/proposals/README.md", "# Proposals\nchanged\n");
  write(repo, "docs/proposals/agent-language-services-lifecycle-migration.md", "# Life\nchanged\n");
  write(repo, "docs/proposals/agent-language-services-platform-matrix-closure.md", "# Old\n");
  write(repo, "docs/reference/implemented-proposals/README.md", "# Records\n");
  git(repo, "add", ".");
  git(repo, "commit", "-m", "seed paths");
  const seeded = rev(repo);

  write(repo, ".github/workflows/workflow--test-scripts.yaml", "name: test\nchanged\n");
  write(repo, "docs/proposals/README.md", "# Proposals\nchanged again\n");
  write(repo, "docs/proposals/agent-language-services-lifecycle-migration.md", "# Life\nchanged again\n");
  write(repo, proposalFile, `${proposalDocument()}\nchanged\n`);
  remove(repo, "docs/proposals/agent-language-services-platform-matrix-closure.md");
  write(repo, "docs/reference/implemented-proposals/README.md", "# Records\nchanged\n");
  write(repo, "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md", "# Record\n");
  write(repo, "workflow-scripts/check-agent-language-platform-matrix.mjs", "export {};\n");
  write(repo, "workflow-scripts/check-agent-language-platform-matrix.test.mjs", "import test from 'node:test';\n");
  git(repo, "add", ".");
  git(repo, "commit", "-m", "close matrix");
  const closed = rev(repo);
  assert.deepEqual(validateClosureRange({ repoRoot: repo, baseSha: seeded, headSha: closed }), []);

  write(repo, "docs/notes.md", "# Notes\n");
  git(repo, "add", ".");
  git(repo, "commit", "-m", "later docs");
  const later = rev(repo);
  assert.deepEqual(validateClosureRange({ repoRoot: repo, baseSha: closed, headSha: later }), []);

  write(repo, "docs/unexpected.md", "# Unexpected\n");
  git(repo, "add", ".");
  git(repo, "commit", "-m", "bad closure");
  const bad = rev(repo);
  assert.match(validateClosureRange({ repoRoot: repo, baseSha: base, headSha: bad }).join("\n"), /docs\/unexpected.md|required/);
});

test("closure guard rejects protected path moves and Git type changes in real ranges", () => {
  const moved = seedOpenMatrixRepository();
  applyClosureManifest(moved);
  fs.renameSync(path.join(moved, ".github/workflows/workflow--test-scripts.yaml"), path.join(moved, ".github/workflows/workflow--test-scripts-renamed.yaml"));
  git(moved, "add", ".");
  git(moved, "commit", "-m", "move protected path");
  assert.match(validateClosureRange({ repoRoot: moved, baseSha: firstCommit(moved), headSha: rev(moved) }).join("\n"), /workflow--test-scripts.yaml: restore required|workflow--test-scripts-renamed.yaml: remove this/);

  const symlinked = seedOpenMatrixRepository();
  applyClosureManifest(symlinked);
  remove(symlinked, "workflow-scripts/check-agent-language-platform-matrix.mjs");
  fs.symlinkSync("../docs/proposals/agent-language-services.md", path.join(symlinked, "workflow-scripts/check-agent-language-platform-matrix.mjs"));
  git(symlinked, "add", ".");
  git(symlinked, "commit", "-m", "symlink validator");
  assert.match(validateClosureRange({ repoRoot: symlinked, baseSha: firstCommit(symlinked), headSha: rev(symlinked) }).join("\n"), /restore A 000000->100644|120000/);

  const executable = seedOpenMatrixRepository();
  applyClosureManifest(executable);
  fs.chmodSync(path.join(executable, "workflow-scripts/check-agent-language-platform-matrix.test.mjs"), 0o755);
  git(executable, "add", ".");
  git(executable, "commit", "-m", "make test executable");
  assert.match(validateClosureRange({ repoRoot: executable, baseSha: firstCommit(executable), headSha: rev(executable) }).join("\n"), /restore A 000000->100644|100755/);
});

function write(repo, file, text) {
  const target = path.join(repo, file);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, text);
}

function remove(repo, file) {
  fs.rmSync(path.join(repo, file));
}

function seedOpenMatrixRepository() {
  const repo = fs.mkdtempSync(path.join(fs.realpathSync.native("/tmp"), "platform-paths-"));
  git(repo, "init");
  git(repo, "config", "user.name", "Veln Test");
  git(repo, "config", "user.email", "veln-test@example.invalid");
  write(repo, proposalFile, "# Agent Language Services\n\n## Agent Plugin\n\nno phase\n");
  write(repo, ".github/workflows/workflow--test-scripts.yaml", "name: test\n");
  write(repo, "docs/proposals/README.md", "# Proposals\n");
  write(repo, "docs/proposals/agent-language-services-lifecycle-migration.md", "# Life\n");
  write(repo, "docs/proposals/agent-language-services-platform-matrix-closure.md", "# Old\n");
  write(repo, "docs/reference/implemented-proposals/README.md", "# Records\n");
  git(repo, "add", ".");
  git(repo, "commit", "-m", "base");
  return repo;
}

function applyClosureManifest(repo) {
  write(repo, ".github/workflows/workflow--test-scripts.yaml", "name: test\nchanged\n");
  write(repo, "docs/proposals/README.md", "# Proposals\nchanged\n");
  write(repo, "docs/proposals/agent-language-services-lifecycle-migration.md", "# Life\nchanged\n");
  write(repo, proposalFile, proposalDocument());
  remove(repo, "docs/proposals/agent-language-services-platform-matrix-closure.md");
  write(repo, "docs/reference/implemented-proposals/README.md", "# Records\nchanged\n");
  write(repo, "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md", "# Record\n");
  write(repo, "workflow-scripts/check-agent-language-platform-matrix.mjs", "export {};\n");
  write(repo, "workflow-scripts/check-agent-language-platform-matrix.test.mjs", "import test from 'node:test';\n");
}

function git(repo, ...args) {
  const result = spawnSync("git", args, { cwd: repo, encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}

function rev(repo) {
  return git(repo, "rev-parse", "HEAD");
}

function firstCommit(repo) {
  return git(repo, "rev-list", "--max-parents=0", "HEAD");
}
