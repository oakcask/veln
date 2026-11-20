import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const maxFindings = 50;
const evidenceLabels = {
  "no-references": "No references",
  "test-only-references": "Test references only",
};

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 3) {
      throw new Error("provide the unused-public-items JSON report path");
    }
    const report = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
    const summary = renderSummary(report);
    if (process.env.GITHUB_STEP_SUMMARY) {
      fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary);
    } else {
      process.stdout.write(summary);
    }
  } catch (error) {
    console.error(`Regenerate the unused-public-items JSON report before rerunning; an invalid report cannot provide a complete refactoring summary. ${error.message}`);
    process.exitCode = 1;
  }
}

export function renderSummary(report) {
  validateReport(report);
  const unreferenced = report.findings.filter((item) => item.evidence === "no-references");
  const testOnly = report.findings.filter((item) => item.evidence === "test-only-references");
  const findings = [...unreferenced, ...testOnly];
  const lines = [
    "## Suspected Dead Public Items",
    "",
    "Review these candidates before refactoring to identify unused API surface. Findings are advisory; check cfg, macros, external callers, and same-name collisions before removing code.",
    "",
    `- Rust files analyzed: ${report.files}`,
    `- Public definitions analyzed: ${report.public_definitions}`,
    `- Suspected dead public items: ${findings.length}`,
    `- No references: ${unreferenced.length}`,
    `- Test references only: ${testOnly.length}`,
    "",
  ];
  if (findings.length === 0) {
    lines.push("No suspected dead public items were found in the scanned roots.", "");
  } else {
    lines.push("| Definition | Kind | Name | Evidence |", "| --- | --- | --- | --- |");
    for (const item of findings.slice(0, maxFindings)) {
      lines.push(`| ${code(`${item.file}:${item.line}`)} | ${code(item.kind)} | ${code(item.name)} | ${evidenceLabels[item.evidence]} |`);
    }
    if (findings.length > maxFindings) {
      lines.push("", `${findings.length - maxFindings} more candidate(s) omitted from this summary.`);
    }
    lines.push("");
  }
  lines.push("Download the **unused-public-items** artifact for the complete JSON report and scanner limitations. Name matching does not prove reachability; imports and same-name items can hide unused code.", "");
  return `${lines.join("\n")}\n`;
}

function validateReport(report) {
  if (report?.schema_version !== 1 || report.status !== "advisory") {
    throw new Error("expected advisory unused-public-items schema version 1");
  }
  if (![report.files, report.public_definitions].every((count) => Number.isSafeInteger(count) && count >= 0)
      || !Array.isArray(report.findings) || report.findings.length > report.public_definitions) {
    throw new Error("expected nonnegative scan counts and a findings array within the definition count");
  }
  for (const item of report.findings) {
    if (!item || ![item.file, item.kind, item.name].every((value) => typeof value === "string" && value.length > 0)
        || !Number.isSafeInteger(item.line) || item.line < 1
        || !Object.hasOwn(evidenceLabels, item.evidence)) {
      throw new Error("expected a definition location, kind, name, and supported reference evidence for each finding");
    }
  }
}

function code(value) {
  const escaped = value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;")
    .replaceAll("|", "&#124;").replaceAll("\r", " ").replaceAll("\n", " ");
  return `<code>${escaped}</code>`;
}
