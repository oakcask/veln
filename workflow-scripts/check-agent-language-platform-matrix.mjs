import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const umbrellaPath = "docs/proposals/agent-language-services.md";

const expectedKeys = [
  "codex/x86_64-unknown-linux-gnu",
  "claude-code/x86_64-unknown-linux-gnu",
];
const expectedColumns = [
  "Client",
  "Platform",
  "Host build",
  "Manifest schema",
  "Validator version",
  "Validator integrity",
  "Veln contract",
  "MCP contract",
  "LSP contract",
  "Language-service contract",
  "Reference-schema contract",
];
const expectedRows = [
  [
    "codex", "x86_64-unknown-linux-gnu", "codex-host-1", "codex-manifest-1",
    "plugin-validator-1", "a02d8a7d8298ecd869813e0d4f5ea6334ebfb088c2686e67f8f8c2661e136cc9",
    "veln-toolchain-1", "mcp-contract-1", "lsp-contract-1",
    "veln-language-service-1", "veln-reference-schema-1",
  ],
  [
    "claude-code", "x86_64-unknown-linux-gnu", "claude-code-host-1", "claude-code-manifest-1",
    "plugin-validator-1", "69c6b136df1c218900f1a1e6f62415cce0b977eed7fc1cd1574d0615bcdde3c3",
    "veln-toolchain-1", "mcp-contract-1", "lsp-contract-1",
    "veln-language-service-1", "veln-reference-schema-1",
  ],
];
const requiredReferences = new Map([
  ["plugin server lifecycle", "Every row in the Closed Client-Platform Matrix starts one server"],
  ["plugin installer boundary", "both clients in the Closed Client-Platform Matrix expose"],
  ["plugin native validation", "Every row in the Closed Client-Platform Matrix uses client-native installation"],
  ["conformance requirement coverage", "row in the Closed Client-Platform\nMatrix."],
  ["Q21 evidence", "For every row in the Closed Client-Platform Matrix:"],
  ["Q22 totality", "missing Closed Client-Platform Matrix row"],
  ["completion rule", "every row in the Closed Client-Platform Matrix passes"],
  ["plugin acceptance completion", "row in the Closed Client-Platform Matrix passes with no orphan"],
]);
const closurePaths = new Set([
  ".github/workflows/workflow--test-scripts.yaml",
  "docs/proposals/README.md",
  "docs/proposals/agent-language-services-lifecycle-migration.md",
  "docs/proposals/agent-language-services-platform-matrix-closure.md",
  umbrellaPath,
  "docs/reference/implemented-proposals/README.md",
  "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md",
  "workflow-scripts/check-agent-language-platform-matrix.mjs",
  "workflow-scripts/check-agent-language-platform-matrix.test.mjs",
]);
const exactLiteralPattern = /^[a-z0-9]+(?:[._-][a-z0-9]+)*$/;
const placeholderPattern = /^(?:all|any|default|future|latest|placeholder|supported|tbd|todo|unspecified)$/i;

if (isMainModule()) {
  const repoRoot = process.cwd();
  const text = fs.readFileSync(path.resolve(repoRoot, umbrellaPath), "utf8");
  const errors = [...validatePlatformMatrix(text).errors];
  const baseSha = process.env.AGENT_PLATFORM_MATRIX_BASE_SHA;
  const headSha = process.env.AGENT_PLATFORM_MATRIX_HEAD_SHA;

  if (baseSha && headSha && !/^0+$/.test(baseSha)) {
    const baseText = readRevisionFile(repoRoot, baseSha, umbrellaPath);
    const headText = readRevisionFile(repoRoot, headSha, umbrellaPath);
    const changes = changedPaths(repoRoot, baseSha, headSha);
    errors.push(...validateClosureTransition({ baseText, headText, changes }).errors);
  }

  if (errors.length > 0) {
    const message = [
      "Restore the named row, reference, or closure path before merging; the lifecycle inventory cannot prove finite client-platform coverage otherwise.",
      ...errors.map((error) => `- ${error}`),
    ].join("\n");
    if (process.env.GITHUB_ACTIONS === "true") {
      console.error(renderGitHubErrorAnnotation(message));
    }
    console.error(message);
    process.exit(1);
  }

  console.log(`Agent language platform matrix is valid with ${expectedKeys.length} ordered row(s).`);
}

export function validatePlatformMatrix(text) {
  const errors = [];
  const sections = markdownSections(text, "### Closed Client-Platform Matrix");
  if (sections.length !== 1) {
    return failure(`matrix heading: restore exactly one Closed Client-Platform Matrix section; found ${sections.length}`);
  }
  const section = sections[0];
  const countMatches = [...section.matchAll(/^Closed client-platform row count: `([^`]*)`\.$/gm)];
  if (countMatches.length !== 1) {
    errors.push(`matrix row count: restore exactly one checked literal \`${expectedKeys.length}\` next to the table`);
  } else if (countMatches[0][1] !== String(expectedKeys.length)) {
    errors.push(`matrix row count: replace \`${countMatches[0][1]}\` with \`${expectedKeys.length}\` so the checked count matches the finite table`);
  }

  const tables = markdownTables(section);
  if (tables.length !== 1) {
    errors.push(`matrix table: restore exactly one literal compatibility table; found ${tables.length}`);
    return { errors, valid: false };
  }
  const table = tables[0];
  if (!sameArray(table.columns, expectedColumns)) {
    errors.push(`matrix columns: restore exactly these ordered fields: ${expectedColumns.join(", ")}`);
  }
  if (table.delimiter !== `| ${expectedColumns.map(() => "---").join(" | ")} |`) {
    errors.push("matrix delimiter: restore one plain Markdown delimiter cell for every compatibility field");
  }
  if (table.rows.length !== expectedKeys.length) {
    errors.push(`matrix rows: restore exactly ${expectedKeys.length} rows; found ${table.rows.length}`);
  }

  const keys = [];
  for (const [index, cells] of table.rows.entries()) {
    const label = `matrix row ${index + 1}`;
    if (cells.length !== expectedColumns.length) {
      errors.push(`${label}: restore all ${expectedColumns.length} compatibility fields; found ${cells.length}`);
      continue;
    }
    const values = cells.map((cell, fieldIndex) => codeLiteral(cell, `${label} ${expectedColumns[fieldIndex]}`, errors));
    const [client, platform] = values;
    keys.push(`${client}/${platform}`);
    for (const [fieldIndex, value] of values.entries()) {
      const fieldLabel = `${label} ${expectedColumns[fieldIndex]}`;
      if (value === "") {
        errors.push(`${fieldLabel}: restore a nonempty exact literal`);
      } else if (fieldIndex === 5) {
        if (!/^[0-9a-f]{64}$/.test(value)) {
          errors.push(`${fieldLabel}: use exactly 64 lowercase hexadecimal digits`);
        }
      } else if (!exactLiteralPattern.test(value) || placeholderPattern.test(value)) {
        errors.push(`${fieldLabel}: replace \`${value}\` with one exact literal; ranges, wildcards, placeholders, and catch-all values are invalid`);
      }
      const expected = expectedRows[index]?.[fieldIndex];
      if (expected !== undefined && value !== expected) {
        errors.push(`${fieldLabel}: restore the closed value \`${expected}\` instead of \`${value}\``);
      }
    }
  }

  for (const key of new Set(keys)) {
    if (keys.filter((candidate) => candidate === key).length > 1) {
      errors.push(`matrix key ${key}: remove the duplicate and restore one row per exact cell`);
    }
  }
  if (!sameArray(keys, expectedKeys)) {
    errors.push(`matrix keys: restore this exact order: ${expectedKeys.join(", ")}`);
  }

  for (const [name, fragment] of requiredReferences) {
    if (!text.includes(fragment)) {
      errors.push(`${name} reference: route the requirement to the Closed Client-Platform Matrix instead of an unnamed platform set`);
    }
  }
  const unnamedSet = text.match(/\b(?:supported|unnamed)\s+(?:client(?:-platform)?|platform)s?\b/i);
  if (unnamedSet) {
    const line = text.slice(0, unnamedSet.index).split("\n").length;
    errors.push(`reference at line ${line}: replace \`${unnamedSet[0]}\` with the Closed Client-Platform Matrix so no second platform universe exists`);
  }
  return { errors, valid: errors.length === 0 };
}

export function validateClosureTransition({ baseText, headText, changes }) {
  if (hasClosedMembership(baseText) || !hasClosedMembership(headText)) {
    return { active: false, errors: [], valid: true };
  }

  const errors = [];
  for (const change of changes) {
    const paths = [change.oldPath, change.path].filter(Boolean);
    if (change.status.startsWith("R")) {
      const archiveMove = change.oldPath === "docs/proposals/agent-language-services-platform-matrix-closure.md"
        && change.path === "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md";
      if (!archiveMove) {
        errors.push(`${paths.join(" -> ")}: restore the protected path instead of renaming it; stable paths keep the finite closure review auditable`);
      }
    }
    if (!paths.every((candidate) => closurePaths.has(candidate))) {
      errors.push(`${paths.join(" -> ")}: remove this out-of-scope change from the matrix closure; mixing other work prevents a finite lifecycle review`);
    }
    if (!["000000", "100644"].includes(change.oldMode) || !["000000", "100644"].includes(change.newMode)) {
      errors.push(`${paths.join(" -> ")}: restore a regular non-executable file Git type; type or mode changes can bypass the documentation-only review`);
    }
  }
  return { active: true, errors, valid: errors.length === 0 };
}

export function parseRawDiff(output) {
  const tokens = output.split("\0").filter(Boolean);
  const changes = [];
  for (let index = 0; index < tokens.length;) {
    const header = tokens[index++];
    const match = header.match(/^:(\d{6}) (\d{6}) [0-9a-f]+ [0-9a-f]+ ([A-Z]\d*)$/);
    if (!match) throw new Error(`Unable to parse git diff record: ${header}`);
    const status = match[3];
    const firstPath = tokens[index++];
    if (status.startsWith("R") || status.startsWith("C")) {
      changes.push({ oldMode: match[1], newMode: match[2], status, oldPath: firstPath, path: tokens[index++] });
    } else {
      changes.push({ oldMode: match[1], newMode: match[2], status, path: firstPath });
    }
  }
  return changes;
}

function markdownSections(text, heading) {
  const lines = text.replaceAll("\r\n", "\n").split("\n");
  const sections = [];
  for (const [index, line] of lines.entries()) {
    if (line !== heading) continue;
    let end = index + 1;
    while (end < lines.length && !/^#{1,3} /.test(lines[end])) end += 1;
    sections.push(lines.slice(index + 1, end).join("\n"));
  }
  return sections;
}

function markdownTables(section) {
  const lines = section.split("\n");
  const tables = [];
  for (let index = 0; index < lines.length; index += 1) {
    if (!lines[index].startsWith("|")) continue;
    const block = [];
    while (index < lines.length && lines[index].startsWith("|")) block.push(lines[index++]);
    index -= 1;
    if (block.length >= 2) {
      tables.push({ columns: markdownCells(block[0]), delimiter: block[1], rows: block.slice(2).map(markdownCells) });
    }
  }
  return tables;
}

function markdownCells(line) {
  return line.slice(1, line.endsWith("|") ? -1 : undefined).split("|").map((cell) => cell.trim());
}

function codeLiteral(cell, label, errors) {
  const match = cell.match(/^`([^`]*)`$/);
  if (!match) {
    errors.push(`${label}: wrap the exact value in one Markdown code span`);
    return cell;
  }
  return match[1];
}

function hasClosedMembership(text) {
  const sections = markdownSections(text, "### Closed Client-Platform Matrix");
  if (sections.length !== 1) return false;
  const count = sections[0].match(/^Closed client-platform row count: `([^`]*)`\.$/m)?.[1];
  const table = markdownTables(sections[0])[0];
  if (count !== String(expectedKeys.length) || table === undefined) return false;
  const keys = table.rows.map((row) => `${row[0]?.replaceAll("`", "")}/${row[1]?.replaceAll("`", "")}`);
  return sameArray(keys, expectedKeys);
}

function sameArray(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function failure(error) {
  return { errors: [error], valid: false };
}

function readRevisionFile(repoRoot, revision, file) {
  const result = spawnSync("git", ["show", `${revision}:${file}`], { cwd: repoRoot, encoding: "utf8" });
  if (result.status === 0) return result.stdout;
  if (result.status === 128) return "";
  throw new Error(`Unable to read ${file} at ${revision}: ${result.stderr.trim()}`);
}

function changedPaths(repoRoot, baseSha, headSha) {
  const result = spawnSync("git", ["diff", "--raw", "--no-abbrev", "--find-renames", "-z", baseSha, headSha, "--"], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) throw new Error(`Unable to inspect matrix closure paths: ${result.stderr.trim()}`);
  return parseRawDiff(result.stdout);
}

function isMainModule() {
  return process.argv[1] !== undefined && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
}

export function renderGitHubErrorAnnotation(message) {
  return `::error title=Invalid agent language platform matrix::${message
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A")}`;
}
