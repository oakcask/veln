import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const umbrellaPath = "docs/proposals/agent-language-services.md";
export const expectedKeys = [
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

const requiredReferences = new Map([
  ["plugin requirement", "Every row in the Closed Client-Platform Matrix uses client-native installation"],
  ["Q21 evidence", "For every row in the Closed Client-Platform Matrix:"],
  ["Q22 totality", "missing Closed Client-Platform Matrix row"],
  ["completion rule", "every row in the Closed Client-Platform Matrix passes"],
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
  const matrixResult = validatePlatformMatrix(text);
  const errors = [...matrixResult.errors];

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
      "Restore the exact closed client-platform matrix before merging; the lifecycle inventory needs one finite platform universe to prove complete coverage.",
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
  const section = markdownSection(text, "### Closed Client-Platform Matrix");
  if (section === undefined) {
    return failure("matrix: restore the Closed Client-Platform Matrix heading and enumerate the two exact cells");
  }

  const rowCountMatch = section.match(/^Closed client-platform row count: `([^`]*)`\.$/m);
  if (!rowCountMatch) {
    errors.push("matrix row count: restore the checked literal `2` next to the table");
  } else if (rowCountMatch[1] !== String(expectedKeys.length)) {
    errors.push(`matrix row count: replace \`${rowCountMatch[1]}\` with \`${expectedKeys.length}\` so the checked count matches the finite table`);
  }

  const lines = section.split("\n");
  const headerIndex = lines.findIndex((line) => line.startsWith("| Client |"));
  if (headerIndex === -1) {
    errors.push("matrix columns: restore the literal compatibility table and all required fields");
    return { errors, valid: false };
  }

  const columns = markdownCells(lines[headerIndex]);
  if (!sameArray(columns, expectedColumns)) {
    errors.push(`matrix columns: restore exactly these ordered fields: ${expectedColumns.join(", ")}`);
  }

  const rows = [];
  for (const line of lines.slice(headerIndex + 2)) {
    if (!line.startsWith("|")) break;
    rows.push(markdownCells(line));
  }

  if (rows.length !== expectedKeys.length) {
    errors.push(`matrix rows: restore exactly ${expectedKeys.length} rows; found ${rows.length}`);
  }

  const keys = [];
  for (const [index, cells] of rows.entries()) {
    const label = `matrix row ${index + 1}`;
    if (cells.length !== expectedColumns.length) {
      errors.push(`${label}: restore all ${expectedColumns.length} compatibility fields; found ${cells.length}`);
      continue;
    }
    const values = cells.map(stripCodeLiteral);
    const [client, platform] = values;
    const key = `${client}/${platform}`;
    keys.push(key);

    if (client === "" || platform === "") {
      errors.push(`${label}: restore a nonempty literal client and platform identifier so the cell has an exact identity`);
    } else {
      validateLiteral(client, `${label} client`, errors);
      validateLiteral(platform, `${label} platform`, errors);
    }

    for (const [fieldIndex, value] of values.entries()) {
      if (fieldIndex < 2) continue;
      const field = expectedColumns[fieldIndex];
      if (value === "") {
        errors.push(`${label} ${field}: restore a nonempty exact literal`);
      } else if (field === "Validator integrity") {
        if (!/^[0-9a-f]{64}$/.test(value)) {
          errors.push(`${label} ${field}: use exactly 64 lowercase hexadecimal digits`);
        }
      } else {
        validateLiteral(value, `${label} ${field}`, errors);
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
      errors.push(`${name} reference: route it to the Closed Client-Platform Matrix instead of an unnamed platform set`);
    }
  }
  const unnamedSet = text.match(/\b(?:supported|unnamed)\s+(?:client(?:-platform)?|platform)s?\b/i);
  if (unnamedSet) {
    const line = text.slice(0, unnamedSet.index).split("\n").length;
    errors.push(`reference at line ${line}: replace \`${unnamedSet[0]}\` with a reference to the Closed Client-Platform Matrix so no second platform universe exists`);
  }

  return { errors, valid: errors.length === 0 };
}

export function validateClosureTransition({ baseText, headText, changes }) {
  if (hasExactMatrix(baseText) || !hasExactMatrix(headText)) {
    return { active: false, errors: [], valid: true };
  }

  const errors = [];
  for (const change of changes) {
    const paths = [change.oldPath, change.path].filter(Boolean);
    if (change.status.startsWith("R")) {
      const isArchiveMove = change.oldPath === "docs/proposals/agent-language-services-platform-matrix-closure.md"
        && change.path === "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md";
      if (!isArchiveMove) {
        errors.push(`${paths.join(" -> ")}: restore the protected path instead of renaming it; stable closure paths keep the finite review scope auditable`);
      }
    }
    if (!paths.every((candidate) => closurePaths.has(candidate))) {
      errors.push(`${paths.join(" -> ")}: remove this out-of-scope change from the matrix closure; mixing other work prevents a finite lifecycle review`);
    }
    if (!["000000", "100644"].includes(change.oldMode) || !["000000", "100644"].includes(change.newMode)) {
      errors.push(`${paths.join(" -> ")}: restore a regular file Git type; type changes can bypass the documentation-only closure review`);
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

function validateLiteral(value, label, errors) {
  if (!exactLiteralPattern.test(value) || placeholderPattern.test(value)) {
    errors.push(`${label}: replace \`${value}\` with one nonempty exact literal; ranges, wildcards, placeholders, and catch-all values are invalid`);
  }
}

function markdownSection(text, heading) {
  const start = text.indexOf(`${heading}\n`);
  if (start === -1) return undefined;
  const bodyStart = start + heading.length + 1;
  const rest = text.slice(bodyStart);
  const end = rest.search(/^#{1,3} /m);
  return end === -1 ? rest : rest.slice(0, end);
}

function markdownCells(line) {
  return line.slice(1, line.endsWith("|") ? -1 : undefined).split("|").map((cell) => cell.trim());
}

function stripCodeLiteral(value) {
  return value.startsWith("`") && value.endsWith("`") ? value.slice(1, -1) : value;
}

function sameArray(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function hasExactMatrix(text) {
  return validatePlatformMatrix(text).valid;
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
  if (result.status !== 0) {
    throw new Error(`Unable to inspect matrix closure paths: ${result.stderr.trim()}`);
  }
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
