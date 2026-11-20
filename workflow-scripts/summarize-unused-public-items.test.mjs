import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { renderSummary } from "./summarize-unused-public-items.mjs";

const script = fileURLToPath(new URL("./summarize-unused-public-items.mjs", import.meta.url));
const candidate = { file: "src/lib.rs", line: 3, kind: "fn", name: "unused", evidence: "no-references",
  references: [], references_truncated: false };
const testUsed = { ...candidate, name: "test_helper", evidence: "test-only-references",
  references: [{ file: "tests/contract.rs", line: 12 }] };
const report = (findings = []) => ({ schema_version: 1, status: "advisory", files: 2,
  public_definitions: findings.length, findings });

test("summary separates test consumers from dead-code counts and shows reference examples", () => {
  const summary = renderSummary(report([testUsed, candidate]));
  assert.match(summary, /Rust files analyzed: 2/);
  assert.match(summary, /Public definitions analyzed: 2/);
  assert.match(summary, /Suspected dead public items \(no references\): 1/);
  assert.match(summary, /Public APIs referenced only from tests \(informational\): 1/);
  assert.match(summary, /<code>tests\/contract.rs:12<\/code>/);
  assert.match(summary, /They are not dead-code candidates/);
  assert.ok(summary.indexOf("<code>unused</code>") < summary.indexOf("<code>test_helper</code>"));
  assert.match(summary, /before removing code/);
  assert.match(summary, /Findings are advisory/);
});

test("empty reports describe the scan scope rather than asserting there is no dead code", () => {
  const summary = renderSummary(report());
  assert.match(summary, /Suspected dead public items \(no references\): 0/);
  assert.match(summary, /No unreferenced public candidates were found in the scanned roots/);
  assert.doesNotMatch(summary, /\| Definition/);
});

test("summary truncates rows while preserving totals and the complete artifact reference", () => {
  const findings = Array.from({ length: 52 }, (_, i) => ({ ...candidate, name: `candidate_${i}` }));
  const summary = renderSummary(report(findings));
  assert.match(summary, /Suspected dead public items \(no references\): 52/);
  assert.match(summary, /2 more candidate\(s\) omitted/);
  assert.match(summary, /<code>candidate_49<\/code>/);
  assert.doesNotMatch(summary, /<code>candidate_50<\/code>/);
  assert.match(summary, /unused-public-items\*\* artifact/);
});

test("test-used APIs alone produce zero dead-code candidates and bounded reference display", () => {
  const summary = renderSummary(report([{ ...testUsed, references_truncated: true }]));
  assert.match(summary, /Suspected dead public items \(no references\): 0/);
  assert.match(summary, /No unreferenced public candidates/);
  assert.match(summary, /More reference locations omitted/);
});

test("source paths cannot inject HTML, table cells or extra rows", () => {
  const summary = renderSummary(report([{ ...candidate, file: "src/<img>&|`\n.rs" }]));
  assert.match(summary, /src\/&lt;img&gt;&amp;&#124;` .rs:3/);
  assert.doesNotMatch(summary, /<img>/);
});

test("malformed reports fail before producing a misleading summary", () => {
  for (const invalid of [null, {}, { ...report(), schema_version: 2 }, { ...report(), files: -1 },
    { ...report(), findings: [candidate] }, report([{ ...candidate, line: 0 }]),
    report([{ ...candidate, evidence: "unknown" }]), report([null]),
    report([{ ...testUsed, references: [] }]), report([{ ...testUsed, references: [{file: "tests/x.rs", line: 0}] }]),
    report([{ ...candidate, references: testUsed.references }])]) {
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
