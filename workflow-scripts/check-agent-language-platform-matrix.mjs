import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const agentLanguageProposalPath = "docs/proposals/agent-language-services.md";

export const expectedClientPlatformRows = [
  ["codex", "x86_64-unknown-linux-gnu"],
  ["claude-code", "x86_64-unknown-linux-gnu"],
];

export const expectedCompatibilityFields = [
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

export const expectedReferenceLinks = [
  {
    id: "plugin-server-lifecycle",
    section: "Agent Plugin",
    text: "Each client starts one server per active workspace root",
  },
  {
    id: "plugin-installer-boundary",
    section: "Agent Plugin",
    text: "add an installer after",
  },
  {
    id: "plugin-compatibility-authority",
    section: "Agent Plugin",
    text: "`compatibility.toml` records compatibility contracts",
  },
  {
    id: "plugin-native-validation",
    section: "Agent Plugin",
    text: "Every row in the",
  },
  {
    id: "conformance-requirement-coverage",
    section: "Conformance Contract",
    text: "every normative paragraph",
  },
  {
    id: "conformance-capability-membership",
    section: "Conformance Contract",
    text: "The v1 manifest closes",
  },
  {
    id: "q21-plugin-matrix",
    section: "Conformance Contract",
    text: "| Q21 plugin matrix |",
  },
  {
    id: "q22-gate-totality",
    section: "Conformance Contract",
    text: "| Q22 gate totality |",
  },
  {
    id: "umbrella-completion",
    section: "Conformance Contract",
    text: "proposal completes only when every declared cell passes",
  },
  {
    id: "plugin-acceptance-completion",
    section: "Plugin",
    text: "| Run the proposal completion gate. |",
  },
];

const closureAllowedOperations = [
  ["M", ".github/workflows/workflow--test-scripts.yaml"],
  ["M", "docs/proposals/README.md"],
  ["M", "docs/proposals/agent-language-services-lifecycle-migration.md"],
  ["D", "docs/proposals/agent-language-services-platform-matrix-closure.md"],
  ["M", "docs/proposals/agent-language-services.md"],
  ["M", "docs/reference/implemented-proposals/README.md"],
  ["A", "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md"],
  ["A", "workflow-scripts/check-agent-language-platform-matrix.mjs"],
  ["A", "workflow-scripts/check-agent-language-platform-matrix.test.mjs"],
];

const expectedRowsDescription = expectedClientPlatformRows
  .map(([client, platform]) => `${client}/${platform}`)
  .join(", ");

if (isMainModule()) {
  const repoRoot = process.cwd();
  const result = validateAgentLanguagePlatformMatrix({
    repoRoot,
    text: fs.readFileSync(path.resolve(repoRoot, agentLanguageProposalPath), "utf8"),
    baseSha: process.env.AGENT_LANGUAGE_PLATFORM_MATRIX_BASE_SHA,
    headSha: process.env.AGENT_LANGUAGE_PLATFORM_MATRIX_HEAD_SHA,
  });

  if (!result.valid) {
    const message = [
      "Restore the closed agent-language-services platform matrix before merging; the lifecycle inventory can prove finite coverage only when every client-platform row and reference is literal and checked.",
      ...result.errors.map((error) => `- ${error}`),
    ].join("\n");
    if (process.env.GITHUB_ACTIONS === "true") {
      console.error(renderGitHubErrorAnnotation(message));
    }
    console.error(message);
    process.exit(1);
  }

  console.log("Agent language-services platform matrix is closed and valid.");
}

export function validateAgentLanguagePlatformMatrix({ repoRoot = process.cwd(), text, baseSha, headSha } = {}) {
  const source = text ?? fs.readFileSync(path.resolve(repoRoot, agentLanguageProposalPath), "utf8");
  const markdown = analyzeMarkdown(source);
  const errors = [
    ...validateMembershipTable(markdown),
    ...validateCompatibilityFields(markdown),
    ...validateReferenceRegistry(markdown),
  ];
  if (baseSha !== undefined || headSha !== undefined) {
    errors.push(...validateClosureDiffScope({ repoRoot, baseSha, headSha }));
  }
  return { errors, valid: errors.length === 0 };
}

export function validateClosureDiffScope({ repoRoot = process.cwd(), baseSha, headSha }) {
  if (!baseSha || !headSha) {
    return [
      "diff-scope: set AGENT_LANGUAGE_PLATFORM_MATRIX_BASE_SHA and AGENT_LANGUAGE_PLATFORM_MATRIX_HEAD_SHA together so the transition-only allowlist can decide whether it is active; finite lifecycle coverage depends on checking the closure transition range",
    ];
  }
  if (revisionHasClosedMatrix({ repoRoot, revision: baseSha }) || !revisionHasClosedMatrix({ repoRoot, revision: headSha })) {
    return [];
  }

  const errors = [];
  const operations = diffOperations({ repoRoot, baseSha, headSha });
  const expected = new Map(closureAllowedOperations.map(([status, file]) => [file, status]));
  const actual = new Map();

  for (const operation of operations) {
    if (operation.status.length !== 1) {
      errors.push(
        `${operation.path}: restore this path operation or move it out of the matrix-closure transition; rename/copy status "${operation.status}" would make the finite closure review depend on Git rename detection`,
      );
      continue;
    }
    if (operation.oldMode !== operation.newMode && operation.oldMode !== "000000" && operation.newMode !== "000000") {
      errors.push(
        `${operation.path}: restore the Git mode ${operation.oldMode} -> ${operation.newMode}; executable-bit or type changes are outside the documentation-only closure and would hide unrelated lifecycle risk`,
      );
    }
    if (operation.status === "A" && operation.newMode !== "100644") {
      errors.push(
        `${operation.path}: restore the added path as a regular non-executable file; added symlinks, submodules, and executable files are outside the documentation-only closure`,
      );
    }
    if (operation.status === "M" && (operation.oldMode !== "100644" || operation.newMode !== "100644")) {
      errors.push(
        `${operation.path}: restore the modified path as a regular non-executable file; mode or Git type changes are outside the documentation-only closure`,
      );
    }
    if (operation.status === "D" && operation.oldMode !== "100644") {
      errors.push(
        `${operation.path}: restore the deleted path to a regular non-executable source before archiving; the closure transition only deletes the proposal record`,
      );
    }
    if (!expected.has(operation.path)) {
      errors.push(
        `${operation.path}: remove this extra path from the matrix-closure transition; finite lifecycle coverage depends on reviewing only the closed table, its validator, workflow registration, and proposal archive`,
      );
      continue;
    }
    const expectedStatus = expected.get(operation.path);
    if (operation.status !== expectedStatus) {
      errors.push(
        `${operation.path}: restore operation ${expectedStatus}; operation ${operation.status} would change the reviewed closure lifecycle boundary`,
      );
    }
    actual.set(operation.path, operation.status);
  }

  for (const [file, expectedStatus] of expected) {
    if (!actual.has(file)) {
      errors.push(
        `${file}: add the required ${expectedStatus} operation to the matrix-closure transition; the lifecycle inventory cannot prove finite coverage unless the closure table, validator, workflow registration, and archive move land together`,
      );
    }
  }

  return errors;
}

export function revisionHasClosedMatrix({ repoRoot = process.cwd(), revision }) {
  const result = spawnSync("git", ["show", `${revision}:${agentLanguageProposalPath}`], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    return false;
  }
  return validateMembershipTable(analyzeMarkdown(result.stdout)).length === 0;
}

function validateMembershipTable(markdown) {
  const errors = [];
  const section = uniqueHeading(markdown, "Closed Client-Platform Matrix", 3);
  if (section.errors.length > 0) {
    return section.errors;
  }
  const table = firstVisibleTableAfter(markdown, section.heading.index);
  if (table === undefined) {
    return [
      `Closed Client-Platform Matrix: add the literal Client/Platform table with ${expectedRowsDescription}; the lifecycle inventory cannot prove finite coverage without the table`,
    ];
  }
  if (table.header.join("/") !== "Client/Platform") {
    errors.push(
      `Closed Client-Platform Matrix line ${table.startLine}: restore columns "Client" and "Platform"; the lifecycle inventory depends on explicit client-platform cells`,
    );
  }
  const seen = new Set();
  const actualRows = table.rows.map((row) => row.cells);
  for (const [index, cells] of actualRows.entries()) {
    const line = table.rows[index].line;
    if (cells.length !== 2) {
      errors.push(
        `Closed Client-Platform Matrix line ${line}: restore exactly two cells, Client and Platform; extra compatibility values are not established by this closure`,
      );
      continue;
    }
    const [client, platform] = cells;
    const key = `${client}/${platform}`;
    if (client === "" || platform === "") {
      errors.push(
        `Closed Client-Platform Matrix line ${line}: enumerate a nonempty client and platform; empty cells cannot prove finite lifecycle coverage`,
      );
    }
    if (/[{[*?]|\.\.\.|<[^>]+>|\ball\b|\bany\b|\bfuture\b|\bsupported\b/i.test(key)) {
      errors.push(
        `Closed Client-Platform Matrix line ${line}: replace "${key}" with one literal client-platform key; ranges, wildcards, placeholders, and catch-all rows leave coverage unbounded`,
      );
    }
    if (seen.has(key)) {
      errors.push(
        `Closed Client-Platform Matrix line ${line}: remove duplicate key "${key}"; each lifecycle cell must be unique`,
      );
    }
    seen.add(key);
  }
  if (actualRows.length !== expectedClientPlatformRows.length) {
    errors.push(
      `Closed Client-Platform Matrix: restore exactly ${expectedClientPlatformRows.length} data rows (${expectedRowsDescription}); a different row count changes the finite lifecycle universe`,
    );
  }
  expectedClientPlatformRows.forEach(([expectedClient, expectedPlatform], index) => {
    const cells = actualRows[index];
    const expectedKey = `${expectedClient}/${expectedPlatform}`;
    if (cells === undefined || cells[0] !== expectedClient || cells[1] !== expectedPlatform) {
      errors.push(
        `Closed Client-Platform Matrix row ${index + 1}: restore ${expectedKey}; reordered, missing, or unexpected keys change the reviewed closure set`,
      );
    }
  });
  return errors;
}

function validateCompatibilityFields(markdown) {
  const section = uniqueHeading(markdown, "Compatibility Field Identities", 4);
  if (section.errors.length > 0) {
    return section.errors;
  }
  const table = firstVisibleTableAfter(markdown, section.heading.index);
  if (table === undefined) {
    return [
      "Compatibility Field Identities: add the ordered Field table; future compatibility records cannot share one shape unless the field identities are literal",
    ];
  }
  const errors = [];
  if (table.header.join("/") !== "Field") {
    errors.push(
      `Compatibility Field Identities line ${table.startLine}: restore the single "Field" column; this closure records identities only, not compatibility values`,
    );
  }
  const actual = table.rows.map((row) => row.cells);
  const seen = new Set();
  actual.forEach((cells, index) => {
    const line = table.rows[index].line;
    if (cells.length !== 1) {
      errors.push(
        `Compatibility Field Identities line ${line}: remove compatibility values and keep one Field cell; this closure cannot choose host builds, schemas, versions, contracts, or digests`,
      );
      return;
    }
    const field = cells[0];
    if (seen.has(field)) {
      errors.push(
        `Compatibility Field Identities line ${line}: remove duplicate field "${field}"; each future record field must have one identity`,
      );
    }
    seen.add(field);
    if (/sha256|digest|=|:|[0-9]+\.[0-9]+/.test(field)) {
      errors.push(
        `Compatibility Field Identities line ${line}: remove value-looking text "${field}"; compatibility values require later artifact-backed validation`,
      );
    }
  });
  if (actual.length !== expectedCompatibilityFields.length) {
    errors.push(
      `Compatibility Field Identities: restore exactly ${expectedCompatibilityFields.length} field rows; a different field count changes the future compatibility record shape`,
    );
  }
  expectedCompatibilityFields.forEach((expected, index) => {
    const actualField = actual[index]?.[0];
    if (actualField !== expected) {
      errors.push(
        `Compatibility Field Identities row ${index + 1}: restore "${expected}"; missing, reordered, or unexpected fields break the closed compatibility shape`,
      );
    }
  });
  return errors;
}

function validateReferenceRegistry(markdown) {
  const errors = [];
  for (const reference of expectedReferenceLinks) {
    const section = visibleSection(markdown, reference.section);
    if (section === undefined) {
      errors.push(
        `${reference.id}: restore section "${reference.section}" so the matrix reference can be checked; finite lifecycle coverage depends on named visible locations`,
      );
      continue;
    }
    const block = section.blocks.find((candidate) => candidate.text.includes(reference.text));
    const requiredLink = `[Closed Client-Platform Matrix](#closed-client-platform-matrix "matrix-ref:${reference.id}")`;
    if (block === undefined) {
      errors.push(
        `${reference.id}: restore the visible block containing "${reference.text}" in "${reference.section}" and link it to the closed matrix; finite lifecycle coverage depends on the registered location`,
      );
      continue;
    }
    if (!containsNonImageLink(block.text, reference.id)) {
      errors.push(
        `${reference.id}: add ${requiredLink} in the registered visible block; links in comments, code, quotes, image text, destinations, or other blocks do not close the platform set`,
      );
    }
  }
  return errors;
}

function diffOperations({ repoRoot, baseSha, headSha }) {
  const result = spawnSync(
    "git",
    ["diff", "--no-renames", "--raw", "-z", baseSha, headSha],
    { cwd: repoRoot, encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(`Unable to inspect matrix-closure diff: ${result.stderr.trim()}`);
  }
  const fields = result.stdout.split("\0").filter(Boolean);
  const operations = [];
  for (let index = 0; index < fields.length; index += 2) {
    const entry = fields[index];
    const file = fields[index + 1];
    const match = entry.match(/^:(\d{6}) (\d{6}) [0-9a-f]+ [0-9a-f]+ ([A-Z]+)$/);
    if (match === null) {
      throw new Error(`Unable to parse git raw diff entry: ${entry}`);
    }
    if (file === undefined) {
      throw new Error(`Unable to parse git raw diff path for entry: ${entry}`);
    }
    operations.push({
      oldMode: match[1],
      newMode: match[2],
      status: match[3],
      path: file,
    });
  }
  return operations;
}

function analyzeMarkdown(text) {
  const lines = text.split("\n");
  const visibleLines = [];
  const headings = [];
  let fenced = false;
  let htmlComment = false;
  lines.forEach((line, index) => {
    const lineNumber = index + 1;
    if (/^\s*(```|~~~)/.test(line)) {
      fenced = !fenced;
      return;
    }
    if (fenced) {
      return;
    }
    if (htmlComment) {
      if (line.includes("-->")) {
        htmlComment = false;
      }
      return;
    }
    if (/^\s*<!--/.test(line)) {
      if (!line.includes("-->")) {
        htmlComment = true;
      }
      return;
    }
    if (/^\s*>/.test(line)) {
      return;
    }
    const heading = line.match(/^(#{1,6})\s+(.+?)\s*#*\s*$/);
    if (heading !== null) {
      headings.push({
        index,
        line: lineNumber,
        level: heading[1].length,
        text: stripInlineMarkdown(heading[2]),
      });
    }
    visibleLines.push({ index, line: lineNumber, text: line });
  });
  return { lines, visibleLines, headings };
}

function uniqueHeading(markdown, text, level) {
  const matches = markdown.headings.filter((heading) => heading.text === text && heading.level === level);
  if (matches.length === 0) {
    return {
      errors: [
        `${text}: restore the unique level-${level} heading; the lifecycle inventory cannot find the checked table without this visible heading`,
      ],
    };
  }
  if (matches.length > 1) {
    return {
      errors: [
        `${text}: remove duplicate level-${level} heading; duplicate table locations leave the finite lifecycle coverage ambiguous`,
      ],
    };
  }
  return { errors: [], heading: matches[0] };
}

function firstVisibleTableAfter(markdown, headingIndex) {
  const nextHeading = markdown.headings.find((heading) => heading.index > headingIndex && heading.level <= markdown.headings.find((candidate) => candidate.index === headingIndex).level);
  const visible = markdown.visibleLines.filter((line) => line.index > headingIndex && (nextHeading === undefined || line.index < nextHeading.index));
  for (let index = 0; index < visible.length - 1; index += 1) {
    const header = parseTableRow(visible[index].text);
    const delimiter = visible[index + 1];
    if (header !== undefined && /^\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)*\|?\s*$/.test(delimiter.text)) {
      const rows = [];
      for (let rowIndex = index + 2; rowIndex < visible.length; rowIndex += 1) {
        const cells = parseTableRow(visible[rowIndex].text);
        if (cells === undefined) {
          break;
        }
        rows.push({ cells, line: visible[rowIndex].line });
      }
      return {
        header,
        rows,
        startLine: visible[index].line,
      };
    }
  }
  return undefined;
}

function visibleSection(markdown, sectionText) {
  const headings = markdown.headings.filter((heading) => heading.text === sectionText);
  if (headings.length === 0) {
    return undefined;
  }
  const heading = headings[0];
  const next = markdown.headings.find((candidate) => candidate.index > heading.index && candidate.level <= heading.level);
  const lines = markdown.visibleLines.filter((line) => line.index > heading.index && (next === undefined || line.index < next.index));
  return { heading, blocks: visibleBlocks(lines) };
}

function visibleBlocks(lines) {
  const blocks = [];
  let current = [];
  for (const line of lines) {
    if (line.text.trim() === "") {
      if (current.length > 0) {
        blocks.push({ text: current.map((item) => item.text).join("\n"), startLine: current[0].line });
        current = [];
      }
      continue;
    }
    current.push(line);
  }
  if (current.length > 0) {
    blocks.push({ text: current.map((item) => item.text).join("\n"), startLine: current[0].line });
  }
  return blocks;
}

function containsNonImageLink(text, referenceId) {
  const pattern = new RegExp(
    `(^|[^!])\\[Closed Client-Platform Matrix\\]\\(#closed-client-platform-matrix "matrix-ref:${escapeRegExp(referenceId)}"\\)`,
  );
  return pattern.test(text);
}

function escapeRegExp(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function parseTableRow(line) {
  const trimmed = line.trim();
  if (!trimmed.startsWith("|") || !trimmed.endsWith("|")) {
    return undefined;
  }
  return trimmed.slice(1, -1).split("|").map((cell) => stripInlineMarkdown(cell.trim()));
}

function stripInlineMarkdown(text) {
  return text
    .replaceAll("`", "")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .trim();
}

export function renderGitHubErrorAnnotation(message) {
  return `::error title=Invalid agent language platform matrix::${message
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A")}`;
}

function isMainModule() {
  return process.argv[1] !== undefined && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
}
