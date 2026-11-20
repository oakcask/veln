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
  const lines = [
    "## Suspected Dead Public Items",
    "",
    "Review these candidates before refactoring to identify unused API surface. Findings are advisory; check cfg, macros, external callers, and same-name collisions before removing code.",
    "",
    `- Rust files analyzed: ${report.files}`,
    `- Public definitions analyzed: ${report.public_definitions}`,
    `- Suspected dead public items (no references): ${unreferenced.length}`,
    `- Public APIs referenced only from tests (informational): ${testOnly.length}`,
    "",
    "### Unreferenced candidates",
    "",
  ];
  if (unreferenced.length === 0) {
    lines.push("No unreferenced public candidates were found in the scanned roots.", "");
  } else {
    appendTable(lines, unreferenced, false);
  }
  lines.push("### Public APIs referenced only from tests", "",
    "These APIs have test consumers, including harness execution and validation helpers. They are not dead-code candidates. Review API placement separately if useful. Reference examples are matched by name and may belong to another same-name definition.", "");
  if (testOnly.length === 0) {
    lines.push("No test-only public API references were found in the scanned roots.", "");
  } else {
    appendTable(lines, testOnly, true);
  }
  lines.push("Download the **unused-public-items** artifact for the complete JSON report and scanner limitations. Name matching does not prove reachability; imports and same-name items can hide unused code.", "");
  return `${lines.join("\n")}\n`;
}

function appendTable(lines, items, withReferences) {
  lines.push(`| Definition | Kind | Name |${withReferences ? " Test reference examples |" : ""}`,
    `| --- | --- | --- |${withReferences ? " --- |" : ""}`);
  for (const item of items.slice(0, maxFindings)) {
    const references = item.references.map((site) => code(`${site.file}:${site.line}`));
    if (item.references_truncated) references.push("More reference locations omitted");
    lines.push(`| ${code(`${item.file}:${item.line}`)} | ${code(item.kind)} | ${code(item.name)} |${withReferences ? ` ${references.join("<br>")} |` : ""}`);
  }
  if (items.length > maxFindings) {
    lines.push("", `${items.length - maxFindings} more ${withReferences ? "test-used API(s)" : "candidate(s)"} omitted from this summary.`);
  }
  lines.push("");
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
        || !Object.hasOwn(evidenceLabels, item.evidence)
        || !Array.isArray(item.references) || item.references.length > 3
        || typeof item.references_truncated !== "boolean"
        || !item.references.every((site) => typeof site?.file === "string" && site.file.length > 0
          && Number.isSafeInteger(site.line) && site.line > 0)
        || (item.evidence === "no-references" && (item.references.length > 0 || item.references_truncated))
        || (item.evidence === "test-only-references" && item.references.length === 0)) {
      throw new Error("expected a definition location, kind, name, and supported reference evidence for each finding");
    }
  }
}

function code(value) {
  const escaped = value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;")
    .replaceAll("|", "&#124;").replaceAll("\r", " ").replaceAll("\n", " ");
  return `<code>${escaped}</code>`;
}
