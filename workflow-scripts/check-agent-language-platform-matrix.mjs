import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const agentLanguageProposal = "docs/proposals/agent-language-services.md";
const matrixHeading = "### Closed Client-Platform Matrix";
const fieldHeading = "#### Compatibility Field Identities";
const lifecycleReason =
  "finite lifecycle coverage depends on an explicit client-platform set";

export const expectedRows = [
  { client: "codex", platform: "x86_64-unknown-linux-gnu" },
  { client: "claude-code", platform: "x86_64-unknown-linux-gnu" },
];

export const expectedFields = [
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

export const expectedReferences = [
  { id: "plugin-server-lifecycle", section: "## Agent Plugin" },
  { id: "plugin-installer-boundary", section: "## Agent Plugin" },
  { id: "plugin-compatibility-authority", section: "## Agent Plugin" },
  { id: "plugin-native-validation", section: "## Agent Plugin" },
  { id: "conformance-requirement-coverage", section: "## Conformance Contract" },
  { id: "conformance-capability-membership", section: "## Conformance Contract" },
  { id: "q21-plugin-matrix", section: "## Conformance Contract" },
  { id: "q22-gate-totality", section: "## Conformance Contract" },
  { id: "umbrella-completion", section: "## Conformance Contract" },
  { id: "plugin-acceptance-completion", section: "### Plugin" },
];

const transitionManifest = [
  { status: "M", path: ".github/workflows/workflow--test-scripts.yaml" },
  { status: "M", path: "docs/proposals/README.md" },
  { status: "M", path: "docs/proposals/agent-language-services-lifecycle-migration.md" },
  { status: "D", path: "docs/proposals/agent-language-services-platform-matrix-closure.md" },
  { status: "M", path: "docs/proposals/agent-language-services.md" },
  { status: "M", path: "docs/reference/implemented-proposals/README.md" },
  { status: "A", path: "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md" },
  { status: "A", path: "workflow-scripts/check-agent-language-platform-matrix.mjs" },
  { status: "A", path: "workflow-scripts/check-agent-language-platform-matrix.test.mjs" },
];

const transitionManifestByPath = new Map(
  transitionManifest.map((entry) => [entry.path, entry]),
);

if (isMainModule()) {
  const repoRoot = process.cwd();
  const errors = [
    ...validateAgentLanguagePlatformMatrix(
      fs.readFileSync(path.join(repoRoot, agentLanguageProposal), "utf8"),
      { file: agentLanguageProposal },
    ).errors,
    ...validateTransitionDiffFromEnvironment(repoRoot),
  ];

  if (errors.length > 0) {
    console.error(
      `Restore the closed agent-language-services client-platform matrix before merging; ${lifecycleReason}.`,
    );
    for (const error of errors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }

  console.log("Agent language-services platform matrix is valid.");
}

export function validateAgentLanguagePlatformMatrix(text, { file = agentLanguageProposal } = {}) {
  const visibleLines = visibleMarkdownLines(text);
  const errors = [
    ...validateMatrixTable({ file, visibleLines }),
    ...validateFieldTable({ file, visibleLines }),
    ...validateClosedReferences({ file, visibleLines }),
    ...validateUnboundPlatformReferences({ file, visibleLines }),
  ];
  return { errors, valid: errors.length === 0 };
}

export function validateTransitionDiffScope({ baseHasMatrix, headHasMatrix, entries }) {
  if (baseHasMatrix || !headHasMatrix) {
    return [];
  }

  const errors = [];
  const seen = new Set();
  for (const entry of entries) {
    const expected = transitionManifestByPath.get(entry.path);
    seen.add(entry.path);
    if (expected === undefined) {
      errors.push(
        `${entry.path}: remove this path from the matrix-closure transition; ${lifecycleReason}, and the transition may not include unrelated remediation`,
      );
      continue;
    }
    if (entry.status !== expected.status) {
      errors.push(
        `${entry.path}: restore operation ${expected.status}; ${lifecycleReason}, and protected proposal lifecycle paths may not be renamed or recast`,
      );
    }
    if (entry.oldType !== undefined && entry.oldType !== "blob") {
      errors.push(
        `${entry.path}: restore the base path to a regular file before this transition; ${lifecycleReason}`,
      );
    }
    if (entry.newType !== undefined && entry.newType !== "blob") {
      errors.push(
        `${entry.path}: keep the head path as a regular file; ${lifecycleReason}`,
      );
    }
    if (entry.oldExecutable === true || entry.newExecutable === true) {
      errors.push(
        `${entry.path}: keep this transition non-executable; ${lifecycleReason}`,
      );
    }
  }

  for (const expected of transitionManifest) {
    if (!seen.has(expected.path)) {
      errors.push(
        `${expected.path}: add required ${expected.status} operation to the matrix-closure transition; ${lifecycleReason}`,
      );
    }
  }

  return errors;
}

function validateMatrixTable({ file, visibleLines }) {
  const errors = [];
  const headingLines = headingLineNumbers(visibleLines, matrixHeading);
  if (headingLines.length !== 1) {
    return [
      `${file}: restore exactly one ${matrixHeading} heading with the two literal client-platform rows; ${lifecycleReason}`,
    ];
  }

  const table = tableAfterHeading(visibleLines, headingLines[0]);
  if (table === undefined) {
    return [
      `${file}:${headingLines[0]}: enumerate the two client-platform rows directly under ${matrixHeading}; ${lifecycleReason}`,
    ];
  }

  if (table.headers.join("\0") !== "Client\0Platform") {
    errors.push(
      `${file}:${table.line}: restore the Client and Platform columns in that order; ${lifecycleReason}`,
    );
  }
  if (table.rows.length !== expectedRows.length) {
    errors.push(
      `${file}:${table.line}: restore exactly ${expectedRows.length} matrix row(s); ${lifecycleReason}`,
    );
  }

  const keys = new Set();
  table.rows.forEach((row, index) => {
    const expected = expectedRows[index];
    const client = row.cells[0] ?? "";
    const platform = row.cells[1] ?? "";
    const key = `${client}/${platform}`;
    if (row.cells.length !== 2) {
      errors.push(
        `${file}:${row.line}: remove compatibility values from matrix row ${index + 1}; ${lifecycleReason}`,
      );
    }
    if (client === "" || platform === "") {
      errors.push(
        `${file}:${row.line}: enumerate a nonempty client and platform in row ${index + 1}; ${lifecycleReason}`,
      );
    }
    if (/[*?]|\b(?:all|any|future|supported|platforms?)\b|\.{2,}/i.test(key)) {
      errors.push(
        `${file}:${row.line}: replace ranged, wildcard, placeholder, or catch-all key "${key}" with the exact row; ${lifecycleReason}`,
      );
    }
    if (keys.has(key)) {
      errors.push(
        `${file}:${row.line}: remove duplicate client-platform key "${key}"; ${lifecycleReason}`,
      );
    }
    keys.add(key);
    if (expected !== undefined && (client !== expected.client || platform !== expected.platform)) {
      errors.push(
        `${file}:${row.line}: restore row ${index + 1} to ${expected.client}/${expected.platform}; ${lifecycleReason}`,
      );
    }
  });

  return errors;
}

function validateFieldTable({ file, visibleLines }) {
  const errors = [];
  const headingLines = headingLineNumbers(visibleLines, fieldHeading);
  if (headingLines.length !== 1) {
    return [
      `${file}: restore exactly one ${fieldHeading} heading with the ordered field identities; ${lifecycleReason}`,
    ];
  }

  const table = tableAfterHeading(visibleLines, headingLines[0]);
  if (table === undefined) {
    return [
      `${file}:${headingLines[0]}: enumerate the compatibility field identities directly under ${fieldHeading}; ${lifecycleReason}`,
    ];
  }

  if (table.headers.join("\0") !== "Field") {
    errors.push(
      `${file}:${table.line}: keep the compatibility table to the Field identity column only; ${lifecycleReason}`,
    );
  }
  if (table.rows.length !== expectedFields.length) {
    errors.push(
      `${file}:${table.line}: restore exactly ${expectedFields.length} compatibility field identities; ${lifecycleReason}`,
    );
  }

  const seen = new Set();
  table.rows.forEach((row, index) => {
    const field = row.cells[0] ?? "";
    if (row.cells.length !== 1) {
      errors.push(
        `${file}:${row.line}: remove compatibility values from field identity row ${index + 1}; ${lifecycleReason}`,
      );
    }
    if (seen.has(field)) {
      errors.push(
        `${file}:${row.line}: remove duplicate compatibility field "${field}"; ${lifecycleReason}`,
      );
    }
    seen.add(field);
    if (field !== expectedFields[index]) {
      errors.push(
        `${file}:${row.line}: restore compatibility field ${index + 1} to "${expectedFields[index]}"; ${lifecycleReason}`,
      );
    }
  });

  return errors;
}

function validateClosedReferences({ file, visibleLines }) {
  const errors = [];
  for (const reference of expectedReferences) {
    const section = sectionLines(visibleLines, reference.section);
    if (section.length === 0) {
      errors.push(
        `${file}: restore section ${reference.section} so matrix reference ${reference.id} can be checked; ${lifecycleReason}`,
      );
      continue;
    }
    const linkPattern = new RegExp(
      String.raw`(?<!!)\[Closed Client-Platform Matrix\]\(#closed-client-platform-matrix "matrix-ref:${escapeRegExp(reference.id)}"\)`,
      "g",
    );
    const matches = [...section.join("\n").matchAll(linkPattern)];
    if (matches.length !== 1) {
      errors.push(
        `${file}: restore exactly one titled matrix link for ${reference.id} in ${reference.section}; ${lifecycleReason}`,
      );
    }
  }

  for (const match of visibleLines.map((line) => line.text).join("\n").matchAll(/matrix-ref:([a-z0-9-]+)/g)) {
    if (!expectedReferences.some((reference) => reference.id === match[1])) {
      errors.push(
        `${file}: remove unexpected matrix reference ${match[1]}; ${lifecycleReason}`,
      );
    }
  }

  return errors;
}

function validateUnboundPlatformReferences({ file, visibleLines }) {
  const errors = [];
  for (const line of visibleLines) {
    if (
      /\bsupported (?:client-platform|client and platform|platform)\b/i.test(line.text) ||
      /\bcompatibility cells\b/i.test(line.text) ||
      /\bevery (?:declared|matrix) cell\b/i.test(line.text)
    ) {
      errors.push(
        `${file}:${line.number}: route this platform-set reference to the closed matrix table; ${lifecycleReason}`,
      );
    }
  }
  return errors;
}

function validateTransitionDiffFromEnvironment(repoRoot) {
  const baseSha = process.env.AGENT_LANGUAGE_PLATFORM_MATRIX_BASE_SHA;
  const headSha = process.env.AGENT_LANGUAGE_PLATFORM_MATRIX_HEAD_SHA;
  if (!baseSha || !headSha || /^0+$/.test(baseSha)) {
    return [];
  }

  const baseText = gitShowText(repoRoot, baseSha, agentLanguageProposal);
  const headText = gitShowText(repoRoot, headSha, agentLanguageProposal);
  const entries = gitDiffEntries(repoRoot, baseSha, headSha);
  return validateTransitionDiffScope({
    baseHasMatrix: baseText !== undefined && validateAgentLanguagePlatformMatrix(baseText).valid,
    headHasMatrix: headText !== undefined && validateAgentLanguagePlatformMatrix(headText).valid,
    entries,
  });
}

function gitShowText(repoRoot, revision, file) {
  const result = spawnSync("git", ["show", `${revision}:${file}`], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    return undefined;
  }
  return result.stdout;
}

function gitDiffEntries(repoRoot, baseSha, headSha) {
  const nameStatus = spawnSync(
    "git",
    ["diff", "--name-status", "--no-renames", "-z", baseSha, headSha],
    { cwd: repoRoot, encoding: "utf8" },
  );
  if (nameStatus.status !== 0) {
    throw new Error(`Unable to inspect matrix closure diff paths: ${nameStatus.stderr.trim()}`);
  }

  const raw = spawnSync(
    "git",
    ["diff", "--raw", "--no-renames", "-z", baseSha, headSha],
    { cwd: repoRoot, encoding: "utf8" },
  );
  if (raw.status !== 0) {
    throw new Error(`Unable to inspect matrix closure diff modes: ${raw.stderr.trim()}`);
  }

  const entries = [];
  const parts = nameStatus.stdout.split("\0").filter(Boolean);
  for (let index = 0; index < parts.length; index += 2) {
    entries.push({ status: parts[index], path: parts[index + 1] });
  }

  const rawParts = raw.stdout.split("\0").filter(Boolean);
  for (let index = 0; index < rawParts.length; index += 2) {
    const meta = rawParts[index];
    const file = rawParts[index + 1];
    const match = meta.match(/^:(\d{6}) (\d{6}) [0-9a-f]+ [0-9a-f]+ ([A-Z])$/);
    if (match === null) {
      continue;
    }
    const entry = entries.find((candidate) => candidate.path === file);
    if (entry === undefined) {
      continue;
    }
    const oldMode = match[1];
    const newMode = match[2];
    entry.oldType = oldMode === "000000" ? undefined : gitModeType(oldMode);
    entry.newType = newMode === "000000" ? undefined : gitModeType(newMode);
    entry.oldExecutable = oldMode === "100755";
    entry.newExecutable = newMode === "100755";
  }

  return entries;
}

function gitModeType(mode) {
  if (mode.startsWith("100")) {
    return "blob";
  }
  if (mode.startsWith("120")) {
    return "symlink";
  }
  if (mode.startsWith("160")) {
    return "gitlink";
  }
  return "other";
}

function visibleMarkdownLines(text) {
  const lines = text.split("\n");
  const visible = [];
  let inFence = false;
  let inHtmlComment = false;
  lines.forEach((line, index) => {
    const trimmed = line.trim();
    if (inHtmlComment) {
      if (trimmed.includes("-->")) {
        inHtmlComment = false;
      }
      return;
    }
    if (trimmed.startsWith("<!--")) {
      if (!trimmed.includes("-->")) {
        inHtmlComment = true;
      }
      return;
    }
    if (/^(```|~~~)/.test(trimmed)) {
      inFence = !inFence;
      return;
    }
    if (inFence || trimmed.startsWith(">")) {
      return;
    }
    visible.push({ number: index + 1, text: stripInlineCode(line) });
  });
  return visible;
}

function headingLineNumbers(lines, heading) {
  return lines
    .filter((line) => line.text.trim() === heading)
    .map((line) => line.number);
}

function tableAfterHeading(lines, headingLine) {
  const headingIndex = lines.findIndex((line) => line.number === headingLine);
  let index = headingIndex + 1;
  while (index < lines.length && lines[index].text.trim() === "") {
    index += 1;
  }
  if (index + 1 >= lines.length || !isTableLine(lines[index].text) || !isSeparatorLine(lines[index + 1].text)) {
    return undefined;
  }

  const headers = tableCells(lines[index].text);
  const rows = [];
  index += 2;
  while (index < lines.length && isTableLine(lines[index].text)) {
    rows.push({ line: lines[index].number, cells: tableCells(lines[index].text) });
    index += 1;
  }
  return { line: lines[index - rows.length - 2].number, headers, rows };
}

function isTableLine(line) {
  return line.trim().startsWith("|") && line.trim().endsWith("|");
}

function isSeparatorLine(line) {
  return /^\|\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)*\|?$/.test(line.trim());
}

function tableCells(line) {
  return line.trim().slice(1, -1).split("|").map((cell) => cell.trim());
}

function stripInlineCode(line) {
  return line.replace(/`[^`\n]*`/g, "");
}

function sectionLines(lines, heading) {
  const start = lines.findIndex((line) => line.text.trim() === heading);
  if (start === -1) {
    return [];
  }
  const level = heading.match(/^#+/)?.[0].length ?? 1;
  const selected = [];
  for (let index = start + 1; index < lines.length; index += 1) {
    const text = lines[index].text;
    const match = text.match(/^(#+)\s+/);
    if (match !== null && match[1].length <= level) {
      break;
    }
    selected.push(text);
  }
  return selected;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}
