import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const defaultDuplicateLimit = 10;

if (isMainModule()) {
  try {
    const reportPath = process.argv[2];
    if (!reportPath) {
      throw new Error("expected the jscpd JSON report path as the first argument");
    }
    const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
    const summary = renderDuplicationSummary(report);
    if (process.env.GITHUB_STEP_SUMMARY) {
      fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary);
    } else {
      console.log(summary);
    }
  } catch (error) {
    const message = `Regenerate the jscpd JSON report before rerunning this check; CI cannot summarize code duplication from an invalid report. ${error.message}`;
    if (process.env.GITHUB_ACTIONS === "true") {
      console.error(`::error title=Invalid jscpd report::${escapeAnnotationMessage(message)}`);
    } else {
      console.error(message);
    }
    process.exitCode = 1;
  }
}

export function renderDuplicationSummary(report, options = {}) {
  validateReport(report);
  const duplicateLimit = options.duplicateLimit ?? defaultDuplicateLimit;
  const duplicates = [...report.duplicates].sort(compareDuplicates);
  const total = report.statistics.total;
  const lines = [
    "## Code Duplication Refactor Signals",
    "",
    "Inspect the largest exact clone pairs when changing either occurrence; consolidating shared behavior can prevent fixes from diverging. This report is advisory because some repetition is intentional.",
    "",
    `- Rust files analyzed: ${total.sources}`,
    `- Rust lines analyzed: ${total.lines}`,
    `- Exact clone pairs: ${total.clones}`,
    `- Duplicated lines: ${total.duplicatedLines} (${formatPercentage(total.percentage)}%)`,
    "",
    "### Largest exact clone pairs",
    "",
  ];

  if (duplicates.length === 0) {
    lines.push("No exact clone pairs were detected.", "");
    return `${lines.join("\n")}\n`;
  }

  lines.push(
    "| Lines | Tokens | First occurrence | Second occurrence |",
    "| ---: | ---: | --- | --- |",
  );
  for (const duplicate of duplicates.slice(0, duplicateLimit)) {
    lines.push(
      `| ${duplicate.lines} | ${duplicate.tokens} | ${formatLocation(duplicate.firstFile)} | ${formatLocation(duplicate.secondFile)} |`,
    );
  }
  if (duplicates.length > duplicateLimit) {
    lines.push("", `${duplicates.length - duplicateLimit} more clone pair(s) omitted from this summary.`);
  }
  lines.push("");
  return `${lines.join("\n")}\n`;
}

function validateReport(report) {
  if (!Array.isArray(report?.duplicates)) {
    throw new Error("expected a duplicates array");
  }
  validateStatistics(report.statistics?.total);
  for (const duplicate of report.duplicates) {
    if (!isNonNegativeInteger(duplicate?.lines) || !isNonNegativeInteger(duplicate?.tokens)) {
      throw new Error("expected each duplicate to have non-negative integer lines and tokens");
    }
    validateFileLocation(duplicate.firstFile);
    validateFileLocation(duplicate.secondFile);
  }
}

function validateStatistics(statistics) {
  for (const field of ["sources", "lines", "clones", "duplicatedLines"]) {
    if (!isNonNegativeInteger(statistics?.[field])) {
      throw new Error(`expected statistics.total.${field} to be a non-negative integer`);
    }
  }
  if (typeof statistics.percentage !== "number" || !Number.isFinite(statistics.percentage) || statistics.percentage < 0) {
    throw new Error("expected statistics.total.percentage to be a non-negative finite number");
  }
}

function validateFileLocation(location) {
  if (typeof location?.name !== "string" || location.name.length === 0 || !isNonNegativeInteger(location.start)) {
    throw new Error("expected each duplicate occurrence to have a file name and start line");
  }
}

function compareDuplicates(left, right) {
  return right.lines - left.lines
    || right.tokens - left.tokens
    || left.firstFile.name.localeCompare(right.firstFile.name)
    || left.firstFile.start - right.firstFile.start
    || left.secondFile.name.localeCompare(right.secondFile.name)
    || left.secondFile.start - right.secondFile.start;
}

function formatLocation(location) {
  return `\`${escapeMarkdown(location.name)}:${location.start}\``;
}

function formatPercentage(value) {
  return value.toFixed(2);
}

function isNonNegativeInteger(value) {
  return Number.isInteger(value) && value >= 0;
}

function escapeMarkdown(value) {
  return value.replaceAll("|", "\\|").replaceAll("`", "\\`");
}

function escapeAnnotationMessage(value) {
  return value.replaceAll("%", "%25").replaceAll("\r", "%0D").replaceAll("\n", "%0A");
}

function isMainModule() {
  return process.argv[1] !== undefined && fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);
}
