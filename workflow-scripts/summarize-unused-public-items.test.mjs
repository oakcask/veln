import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { renderSummary } from "./summarize-unused-public-items.mjs";

const script = fileURLToPath(new URL("./summarize-unused-public-items.mjs", import.meta.url));
const candidate = { file: "src/lib.rs", line: 3, kind: "fn", name: "unused", evidence: "no-references" };
const report = (findings = []) => ({ schema_version: 1, status: "advisory", files: 2,
  public_definitions: findings.length, findings });

test("summary reports counts, definition locations and evidence with unreferenced items first", () => {
  const summary = renderSummary(report([{ ...candidate, name: "test_helper", evidence: "test-only-references" }, candidate]));
  assert.match(summary, /Rust files analyzed: 2/);
  assert.match(summary, /Public definitions analyzed: 2/);
  assert.match(summary, /Suspected dead public items: 2/);
  assert.match(summary, /No references: 1/);
  assert.match(summary, /Test references only: 1/);
  assert.match(summary, /<code>src\/lib.rs:3<\/code>.*No references/);
  assert.ok(summary.indexOf("<code>unused</code>") < summary.indexOf("<code>test_helper</code>"));
  assert.match(summary, /before removing code/);
  assert.match(summary, /Findings are advisory/);
});

test("empty reports describe the scan scope rather than asserting there is no dead code", () => {
  const summary = renderSummary(report());
  assert.match(summary, /Suspected dead public items: 0/);
  assert.match(summary, /No suspected dead public items were found in the scanned roots/);
  assert.doesNotMatch(summary, /\| Definition/);
});

test("summary truncates rows while preserving totals and the complete artifact reference", () => {
  const findings = Array.from({ length: 52 }, (_, i) => ({ ...candidate, name: `candidate_${i}` }));
  const summary = renderSummary(report(findings));
  assert.match(summary, /Suspected dead public items: 52/);
  assert.match(summary, /2 more candidate\(s\) omitted/);
  assert.match(summary, /<code>candidate_49<\/code>/);
  assert.doesNotMatch(summary, /<code>candidate_50<\/code>/);
  assert.match(summary, /unused-public-items\*\* artifact/);
});

test("source paths cannot inject HTML, table cells or extra rows", () => {
  const summary = renderSummary(report([{ ...candidate, file: "src/<img>&|`\n.rs" }]));
  assert.match(summary, /src\/&lt;img&gt;&amp;&#124;` .rs:3/);
  assert.doesNotMatch(summary, /<img>/);
});

test("malformed reports fail before producing a misleading summary", () => {
  for (const invalid of [null, {}, { ...report(), schema_version: 2 }, { ...report(), files: -1 },
    { ...report(), findings: [candidate] }, report([{ ...candidate, line: 0 }]),
    report([{ ...candidate, evidence: "unknown" }]), report([null])]) {
    assert.throws(() => renderSummary(invalid));
  }
});

test("CLI appends the summary, prints locally, and preserves the summary on invalid input", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "veln-unused-pub-summary-"));
  try {
    const input = path.join(dir, "report.json");
    const output = path.join(dir, "summary.md");
    const data = report([candidate]);
    fs.writeFileSync(input, JSON.stringify(data));
    fs.writeFileSync(output, "Existing summary\n");
    const run = (summary = output) => spawnSync(process.execPath, [script, input], {
      encoding: "utf8", env: { ...process.env, GITHUB_STEP_SUMMARY: summary },
    });
    assert.equal(run().status, 0);
    assert.equal(fs.readFileSync(output, "utf8"), `Existing summary\n${renderSummary(data)}`);
    assert.equal(run("").stdout, renderSummary(data));
    const before = fs.readFileSync(output, "utf8");
    fs.writeFileSync(input, "invalid JSON");
    const failed = run();
    assert.equal(failed.status, 1);
    assert.match(failed.stderr, /Regenerate the unused-public-items JSON report/);
    assert.equal(fs.readFileSync(output, "utf8"), before);
  } finally {
    fs.rmSync(dir, { recursive: true });
  }
});
