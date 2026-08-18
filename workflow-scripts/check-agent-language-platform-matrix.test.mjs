import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  inspectPhaseInText,
  shouldRunRangeGuard,
  validateDocuments,
  validateRawOperations,
  validateRepository,
  validateWorkflow,
} from "./check-agent-language-platform-matrix.mjs";

const ids = [
  "L00", "L01",
  "F01", "F02", "F03", "F04", "F05",
  "K01", "K02", "K03", "K04", "K05", "K06",
  "R01", "R02", "R03", "R04", "P01", "P02",
  "I01", "I02", "H01", "H02",
  "X-U01", "X-U02", "X-CRLF", "X-PIPE",
  "S00-S01", "S01-S01", "S01-S00", "S02-S01", "S02", "S03", "S04",
  "T00", "T01", "T10", "T25", "T26", "T27", "T28",
  "W00", "W01", "W07", "W08", "W10", "W17", "W18", "W27",
  "E01", "E02", "D01", "D02", "D03",
];

test("case manifest has frozen unique IDs", () => {
  assert.equal(new Set(ids).size, ids.length);
  for (const required of ["L00", "F01", "K01", "R01", "S00-S01", "T00", "W00", "E01", "D03"]) {
    assert.ok(ids.includes(required), required);
  }
});

test("E01 validates repository documents", () => {
  const result = validateDocuments(repositoryDocs());
  assert.deepEqual(result, []);
});

test("L and K cases reject moved matrix and nonliteral membership", () => {
  const docs = repositoryDocs();
  assert.match(
    validateDocuments({ ...docs, "agent-language-services.md": docs["agent-language-services.md"].replace("## Agent Plugin", "## Wrong Owner") }).join("\n"),
    /Closed Client-Platform Matrix/,
  );
  assert.match(
    validateDocuments({ ...docs, "agent-language-services.md": docs["agent-language-services.md"].replace("`codex` | `x86_64-unknown-linux-gnu`", "`codex` | `*`") }).join("\n"),
    /membership row 1/,
  );
  assert.match(
    validateDocuments({ ...docs, "agent-language-services.md": docs["agent-language-services.md"].replace("| `claude-code` | `x86_64-unknown-linux-gnu` |\n", "") }).join("\n"),
    /membership/,
  );
});

test("F cases reject reordered, missing, and value-bearing compatibility fields", () => {
  const docs = repositoryDocs();
  assert.match(
    validateDocuments({ ...docs, "agent-language-services.md": docs["agent-language-services.md"].replace("| `host-build` |\n", "") }).join("\n"),
    /compatibility field/,
  );
  assert.match(
    validateDocuments({ ...docs, "agent-language-services.md": docs["agent-language-services.md"].replace("| `client` |", "| `client` | `1` |") }).join("\n"),
    /compatibility field row/,
  );
});

test("R and P cases reject missing, duplicate, moved, and coordinated unregistered references", () => {
  const docs = repositoryDocs();
  assert.match(
    validateDocuments({ ...docs, "agent-language-services.md": docs["agent-language-services.md"].replace(" \"matrix-ref:q21-plugin-matrix\"", " \"matrix-ref:wrong\"") }).join("\n"),
    /matrix-ref:q21-plugin-matrix/,
  );
  assert.match(
    validateDocuments({ ...docs, "agent-language-services.md": docs["agent-language-services.md"].replace("matrix-ref:q22-gate-totality", "matrix-ref:q21-plugin-matrix") }).join("\n"),
    /matrix-ref:q21-plugin-matrix/,
  );
  assert.match(
    validateDocuments({ ...docs, "agent-language-services-lifecycle-migration.md": docs["agent-language-services-lifecycle-migration.md"].replace("matrix-ref:lifecycle-prerequisite-acceptance", "matrix-ref:coordinated") }).join("\n"),
    /matrix-ref:lifecycle-prerequisite-acceptance/,
  );
});

test("I, H, and X cases reject hidden or malformed evidence without stalling", () => {
  const docs = repositoryDocs();
  assert.match(
    validateDocuments({ ...docs, "agent-language-services.md": docs["agent-language-services.md"].replace("### Closed Client-Platform Matrix", "> ### Closed Client-Platform Matrix") }).join("\n"),
    /Closed Client-Platform Matrix/,
  );
  assert.match(
    validateDocuments({ ...docs, "agent-language-services.md": `${docs["agent-language-services.md"]}\n<!-- ${phaseText()} -->\n` }).join("\n"),
    /hidden or displaced/,
  );
  assert.doesNotThrow(() => inspectPhaseInText(docs["agent-language-services.md"].replace("\n| --- | --- |", "\n| --- | ---")));
});

test("S cases classify phase and decide range-guard retirement", () => {
  const docs = repositoryDocs();
  const present = docs["agent-language-services.md"];
  const absent = present.replace(`${phaseText()}\n\n`, "");
  const duplicate = present.replace(phaseText(), `${phaseText()}\n\n${phaseText()}`);
  assert.equal(inspectPhaseInText(absent), "absent");
  assert.equal(inspectPhaseInText(present), "present");
  assert.equal(inspectPhaseInText(duplicate), "invalid");
  assert.equal(shouldRunRangeGuard("absent", "present"), true);
  assert.equal(shouldRunRangeGuard("present", "present"), false);
  assert.equal(shouldRunRangeGuard("present", "absent"), true);
  assert.equal(shouldRunRangeGuard("invalid", "present"), true);
});

test("T cases enforce exact closure operations and retire after closure", () => {
  const exact = [
    op("M", "100644", "100644", ".github/workflows/workflow--test-scripts.yaml"),
    op("M", "100644", "100644", "docs/proposals/README.md"),
    op("M", "100644", "100644", "docs/proposals/agent-language-services-lifecycle-migration.md"),
    op("D", "100644", "000000", "docs/proposals/agent-language-services-platform-matrix-closure.md"),
    op("M", "100644", "100644", "docs/proposals/agent-language-services.md"),
    op("M", "100644", "100644", "docs/reference/implemented-proposals/README.md"),
    op("A", "000000", "100644", "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md"),
    op("A", "000000", "100644", "workflow-scripts/check-agent-language-platform-matrix.mjs"),
    op("A", "000000", "100644", "workflow-scripts/check-agent-language-platform-matrix.test.mjs"),
  ];
  assert.deepEqual(validateRawOperations(exact), []);
  assert.match(validateRawOperations(exact.slice(1)).join("\n"), /workflow--test-scripts/);
  assert.match(validateRawOperations([...exact, op("M", "100644", "100644", "docs/README.md")]).join("\n"), /docs\/README.md/);
  assert.match(validateRawOperations([op("M", "120000", "100644", ".github/workflows/workflow--test-scripts.yaml"), ...exact.slice(1)]).join("\n"), /100644->100644/);
  assert.equal(shouldRunRangeGuard("present", "present"), false);
});

test("W cases validate workflow registration shape", () => {
  const workflow = fs.readFileSync(path.resolve(".github/workflows/workflow--test-scripts.yaml"), "utf8");
  assert.deepEqual(validateWorkflow(workflow), []);
  assert.match(validateWorkflow(workflow.replace("Validate the closed agent language platform matrix", "Other")).join("\n"), /exactly one step/);
  assert.match(validateWorkflow(workflow.replace("node workflow-scripts/check-agent-language-platform-matrix.mjs", "node other.mjs")).join("\n"), /run:/);
  assert.match(validateWorkflow(workflow.replace("AGENT_PLATFORM_MATRIX_BASE_SHA", "OTHER_BASE")).join("\n"), /AGENT_PLATFORM_MATRIX_BASE_SHA/);
  assert.match(validateWorkflow(workflow.replace("  pull_request:", "  pull_request:\n    types: [opened]")).join("\n"), /pull_request|triggers/);
});

test("E02 invokes production entry point on a temporary closure range", () => {
  using fixture = tempRepo("platform-matrix-range");
  fixture.write("docs/proposals/agent-language-services.md", minimalAgentDoc(false));
  fixture.write(".github/workflows/workflow--test-scripts.yaml", "name: old workflow\n");
  fixture.write("docs/proposals/README.md", "# Old Proposals\n");
  fixture.write("docs/proposals/agent-language-services-lifecycle-migration.md", "# Old Lifecycle\n");
  fixture.write("docs/proposals/agent-language-services-platform-matrix-closure.md", "# Old Closure Proposal\n");
  fixture.write("docs/reference/implemented-proposals/README.md", "# Old Records\n");
  fixture.git("init");
  fixture.git("config", "user.email", "agent@example.test");
  fixture.git("config", "user.name", "Agent");
  fixture.git("add", ".");
  fixture.git("commit", "-m", "base");
  const base = fixture.git("rev-parse", "HEAD").stdout.trim();

  for (const [file, text] of Object.entries(repositoryFilesystemSubset())) {
    fixture.write(file, text);
  }
  fixture.remove("docs/proposals/agent-language-services-platform-matrix-closure.md");
  fixture.git("add", ".");
  fixture.git("commit", "-m", "head");
  const head = fixture.git("rev-parse", "HEAD").stdout.trim();

  const result = spawnSync(
    process.execPath,
    [path.resolve("workflow-scripts/check-agent-language-platform-matrix.mjs")],
    {
      cwd: fixture.root,
      env: { ...process.env, AGENT_PLATFORM_MATRIX_BASE_SHA: base, AGENT_PLATFORM_MATRIX_HEAD_SHA: head },
      encoding: "utf8",
    },
  );

  assert.equal(result.status, 0, result.stderr);
});

test("D cases validate implementation-record structure through repository validators", () => {
  const result = validateRepository(process.cwd());
  assert.deepEqual(result.errors.filter((error) => /implementation-record|frontmatter|proposal lifecycle/.test(error)), []);
});

function repositoryDocs() {
  return {
    "agent-language-services.md": fs.readFileSync(path.resolve("docs/proposals/agent-language-services.md"), "utf8"),
    "agent-language-services-lifecycle-migration.md": fs.readFileSync(path.resolve("docs/proposals/agent-language-services-lifecycle-migration.md"), "utf8"),
  };
}

function repositoryFilesystemSubset() {
  const files = [
    ".github/workflows/workflow--test-scripts.yaml",
    "docs/proposals/README.md",
    "docs/proposals/agent-language-services-lifecycle-migration.md",
    "docs/proposals/agent-language-services.md",
    "docs/reference/implemented-proposals/README.md",
    "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md",
    "workflow-scripts/check-agent-language-platform-matrix.mjs",
    "workflow-scripts/check-agent-language-platform-matrix.test.mjs",
  ];
  const entries = {};
  for (const file of files) entries[file] = fs.readFileSync(path.resolve(file), "utf8");
  return entries;
}

function minimalAgentDoc(withPhase) {
  return [
    "---",
    "role: proposal",
    "update-when: The minimal test changes.",
    "---",
    "",
    "# Agent Language Services",
    "",
    "## Agent Plugin",
    "",
    withPhase ? `### Closed Client-Platform Matrix\n\n${phaseText()}\n` : "No matrix yet.",
    "",
    "## Safety And Privacy",
    "",
  ].join("\n");
}

function phaseText() {
  return "Matrix closure phase: `agent-language-services-platform-matrix-closed`.";
}

function op(status, oldMode, newMode, file) {
  return { status, oldMode, newMode, file };
}

function tempRepo(name) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-`));
  return {
    root,
    write(file, text) {
      const target = path.join(root, file);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, text);
    },
    remove(file) {
      fs.rmSync(path.join(root, file), { force: true });
    },
    git(...args) {
      const result = spawnSync("git", args, { cwd: root, encoding: "utf8" });
      assert.equal(result.status, 0, `${args.join(" ")}\n${result.stderr}`);
      return result;
    },
    [Symbol.dispose]() {
      fs.rmSync(root, { recursive: true, force: true });
    },
  };
}
