import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const expectedKeys = [
  "codex/x86_64-unknown-linux-gnu",
  "claude-code/x86_64-unknown-linux-gnu",
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

export const expectedCellRows = [
  [
    "codex",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
    "agent-language-services-v1",
    "agent-platform-matrix-validator-v1",
    "0daa73783d55340cae2cba0fc68bf01d0082fa6825dd1409d29f0266b1542545",
    "veln-toolchain-contract-v1",
    "mcp-tool-contract-v1",
    "lsp-adapter-contract-v1",
    "language-service-contract-v1",
    "reference-schema-contract-v1",
  ],
  [
    "claude-code",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
    "agent-language-services-v1",
    "agent-platform-matrix-validator-v1",
    "b512373bba9ba13ea2ce3d12ec7c3eca81c50ad85f39d60ca61d58585ea5f44d",
    "veln-toolchain-contract-v1",
    "mcp-tool-contract-v1",
    "lsp-adapter-contract-v1",
    "language-service-contract-v1",
    "reference-schema-contract-v1",
  ],
];

export const expectedReferences = [
  ["agent-plugin-server-lifecycle", "agent-language-services.md", "## Agent Plugin", "paragraph 3", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["agent-plugin-installer-boundary", "agent-language-services.md", "## Agent Plugin", "paragraph 7", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["agent-plugin-compatibility-authority", "agent-language-services.md", "## Agent Plugin", "paragraph 8", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["agent-plugin-native-validation", "agent-language-services.md", "## Agent Plugin", "paragraph 9", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["conformance-requirement-coverage", "agent-language-services.md", "## Conformance Contract", "paragraph 1", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["conformance-capability-membership", "agent-language-services.md", "## Conformance Contract", "paragraph 2", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["q21-plugin-matrix", "agent-language-services.md", "## Conformance Contract", "table row `Q21 plugin matrix`", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["q22-gate-totality", "agent-language-services.md", "## Conformance Contract", "table row `Q22 gate totality`", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["umbrella-completion", "agent-language-services.md", "## Conformance Contract", "paragraph 5", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["plugin-acceptance-completion", "agent-language-services.md", "### Plugin", "table row `Run the proposal completion gate`", "Closed Client-Platform Matrix", "#closed-client-platform-matrix"],
  ["lifecycle-conformance-cells", "agent-language-services-lifecycle-migration.md", "## Preserved Finite Inputs", "list item 6", "Closed Client-Platform Matrix", "agent-language-services.md#closed-client-platform-matrix"],
  ["lifecycle-compatibility-cells", "agent-language-services-lifecycle-migration.md", "## Preserved Finite Inputs", "list item 7", "Closed Client-Platform Matrix", "agent-language-services.md#closed-client-platform-matrix"],
  ["lifecycle-prerequisite-acceptance", "agent-language-services-lifecycle-migration.md", "## Acceptance Model", "table row `Close the prerequisite client-platform set`", "Closed Client-Platform Matrix", "agent-language-services.md#closed-client-platform-matrix"],
];

export const expectedOperations = [
  ["M", "100644", "100644", ".github/workflows/workflow--test-scripts.yaml"],
  ["M", "100644", "100644", "docs/proposals/README.md"],
  ["M", "100644", "100644", "docs/proposals/agent-language-services-lifecycle-migration.md"],
  ["D", "100644", "000000", "docs/proposals/agent-language-services-platform-matrix-closure.md"],
  ["M", "100644", "100644", "docs/proposals/agent-language-services.md"],
  ["M", "100644", "100644", "docs/reference/implemented-proposals/README.md"],
  ["A", "000000", "100644", "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md"],
  ["A", "000000", "100644", "workflow-scripts/check-agent-language-platform-matrix.mjs"],
  ["A", "000000", "100644", "workflow-scripts/check-agent-language-platform-matrix.test.mjs"],
];

const phaseText = "Matrix closure phase: `agent-language-services-platform-matrix-closed`.";
const workflowStepName = "Validate the closed agent language platform matrix";
const workflowCommand = "node workflow-scripts/check-agent-language-platform-matrix.mjs";

if (isMainModule()) {
  const repoRoot = process.cwd();
  const result = validateRepository(repoRoot);
  if (!result.valid) {
    console.error([
      "Restore the closed agent-language-services platform matrix before merging; lifecycle migration coverage depends on one finite client-platform universe.",
      ...result.errors.map((error) => `- ${error}`),
    ].join("\n"));
    process.exit(1);
  }
  console.log("Closed agent-language-services platform matrix is valid.");
}

export function validateRepository(repoRoot) {
  const errors = [];
  const docs = {
    "agent-language-services.md": readMaybe(path.join(repoRoot, "docs/proposals/agent-language-services.md")),
    "agent-language-services-lifecycle-migration.md": readMaybe(path.join(repoRoot, "docs/proposals/agent-language-services-lifecycle-migration.md")),
  };

  for (const [file, text] of Object.entries(docs)) {
    if (text === undefined) {
      errors.push(`${file}: restore this document; matrix references cannot be checked without it`);
    }
  }
  if (Object.values(docs).every((text) => text !== undefined)) {
    errors.push(...validateDocuments(docs));
  }
  errors.push(...validateWorkflow(readMaybe(path.join(repoRoot, ".github/workflows/workflow--test-scripts.yaml")) ?? ""));
  errors.push(...validateRangeFromEnvironment(repoRoot, process.env));
  return { valid: errors.length === 0, errors };
}

export function validateDocuments(docs) {
  const errors = [];
  const agent = docs["agent-language-services.md"];
  const lifecycle = docs["agent-language-services-lifecycle-migration.md"];
  const blocks = parseBlocks(agent, "agent-language-services.md");
  const matrixBlocks = matrixInterval(blocks);
  if (matrixBlocks.errors.length > 0) {
    errors.push(...matrixBlocks.errors);
  } else {
    errors.push(...validateMatrixBlocks(matrixBlocks.blocks));
  }
  errors.push(...validateNoHiddenMatrixEvidence(agent, "agent-language-services.md"));
  errors.push(...validateReferences({ "agent-language-services.md": agent, "agent-language-services-lifecycle-migration.md": lifecycle }));
  return errors;
}

export function inspectPhaseInText(text) {
  let parsed;
  try {
    parsed = parseBlocks(text, "agent-language-services.md");
  } catch {
    return "invalid";
  }
  const interval = matrixInterval(parsed);
  if (interval.errors.some((error) => error.includes("Closed Client-Platform Matrix"))) {
    return text.includes(phaseText) ? "invalid" : "absent";
  }
  if (interval.errors.length > 0) {
    return "invalid";
  }
  const count = interval.blocks.filter((block) => block.kind === "paragraph" && block.text.trim() === phaseText).length;
  return count === 0 ? "absent" : count === 1 && interval.blocks[1]?.text.trim() === phaseText ? "present" : "invalid";
}

export function shouldRunRangeGuard(basePhase, headPhase) {
  if (basePhase === "absent" && headPhase === "absent") return false;
  if (basePhase === "present" && headPhase === "present") return false;
  return true;
}

export function validateRawOperations(operations) {
  const errors = [];
  const expected = expectedOperations.map(([status, oldMode, newMode, file]) => ({ status, oldMode, newMode, file }));
  const actualByFile = new Map(operations.map((op) => [op.file, op]));
  for (const exp of expected) {
    const actual = actualByFile.get(exp.file);
    if (actual === undefined) {
      errors.push(`${exp.file}: restore required ${exp.status} operation; finite lifecycle coverage depends on the exact documentation-only closure range`);
      continue;
    }
    if (actual.status !== exp.status || actual.oldMode !== exp.oldMode || actual.newMode !== exp.newMode) {
      errors.push(`${exp.file}: use ${exp.status} ${exp.oldMode}->${exp.newMode}, found ${actual.status} ${actual.oldMode}->${actual.newMode}; restore the permitted path operation so closure evidence stays bounded`);
    }
  }
  for (const actual of operations) {
    if (!expected.some((exp) => exp.file === actual.file)) {
      errors.push(`${actual.file}: remove this ${actual.status} operation from the closure transition or move it to another PR; finite lifecycle coverage depends on exactly the nine matrix-closure paths`);
    }
  }
  return errors;
}

export function validateWorkflow(text) {
  const errors = [];
  if (!/^on:\n(?:  push:[\s\S]*?  pull_request:|  pull_request:[\s\S]*?  push:)/m.test(text)) {
    errors.push("workflow--test-scripts.yaml: keep both push and pull_request triggers so the matrix validator runs on authoritative ranges");
  }
  const pullRequestIndex = text.indexOf("  pull_request:");
  if (pullRequestIndex === -1) {
    errors.push("workflow--test-scripts.yaml: restore the pull_request trigger so closure validation runs before merge");
  } else {
    const triggerBody = text.slice(pullRequestIndex, nextTopLevelKeyIndex(text, pullRequestIndex + 1));
    if (/^    types:/m.test(triggerBody)) {
      errors.push("workflow--test-scripts.yaml: remove restrictive pull_request event types; every pull request that changes matrix paths must run validation");
    }
  }
  for (const required of ["docs/**/*.md", "workflow-scripts/**/*.mjs", ".github/workflows/workflow--test-scripts.yaml"]) {
    if (!text.includes(`      - ${required}`)) {
      errors.push(`workflow--test-scripts.yaml: include path filter ${required}; matrix changes must trigger validation`);
    }
  }
  if (!text.includes("test-workflow-scripts:")) {
    errors.push("workflow--test-scripts.yaml: keep job test-workflow-scripts for matrix validation");
  }
  const stepMatches = [...text.matchAll(new RegExp(`^      - name: ${escapeRegExp(workflowStepName)}$`, "gm"))];
  if (stepMatches.length !== 1) {
    errors.push(`workflow--test-scripts.yaml: keep exactly one step named "${workflowStepName}" so the closed matrix is checked once`);
    return errors;
  }
  const step = text.slice(stepMatches[0].index, nextStepIndex(text, stepMatches[0].index + 1));
  const requiredLines = [
    "        env:",
    "          AGENT_PLATFORM_MATRIX_BASE_SHA: ${{ github.event.pull_request.base.sha || github.event.before }}",
    "          AGENT_PLATFORM_MATRIX_HEAD_SHA: ${{ github.sha }}",
    `        run: ${workflowCommand}`,
  ];
  for (const line of requiredLines) {
    if (!step.includes(line)) {
      errors.push(`workflow--test-scripts.yaml: add "${line.trim()}" to the matrix step; CI must pass the exact base and head range`);
    }
  }
  for (const forbidden of ["        if:", "        continue-on-error:", "        shell:", "        working-directory:"]) {
    if (step.includes(forbidden)) {
      errors.push(`workflow--test-scripts.yaml: remove ${forbidden.trim()} from the matrix step; the guard must not be skipped or rerouted`);
    }
  }
  const jobText = text.slice(text.indexOf("  test-workflow-scripts:"));
  for (const forbidden of ["    if:", "    needs:"]) {
    if (jobText.includes(forbidden)) {
      errors.push(`workflow--test-scripts.yaml: remove job ${forbidden.trim()} from test-workflow-scripts; the guard must run independently`);
    }
  }
  return errors;
}

function validateRangeFromEnvironment(repoRoot, env) {
  const base = env.AGENT_PLATFORM_MATRIX_BASE_SHA;
  const head = env.AGENT_PLATFORM_MATRIX_HEAD_SHA;
  if (!base && !head) return [];
  if (!base || /^0+$/.test(base)) {
    return ["AGENT_PLATFORM_MATRIX_BASE_SHA: pass a readable nonzero base revision so the closure transition can be bounded"];
  }
  if (!head || /^0+$/.test(head)) {
    return ["AGENT_PLATFORM_MATRIX_HEAD_SHA: pass a readable nonzero head revision so the closure transition can be bounded"];
  }
  const baseText = gitShow(repoRoot, base, "docs/proposals/agent-language-services.md");
  const headText = gitShow(repoRoot, head, "docs/proposals/agent-language-services.md");
  if (baseText.status !== 0) return [`${base}: read the base agent-language-services.md from the checkout; phase comparison cannot proceed without it`];
  if (headText.status !== 0) return [`${head}: read the head agent-language-services.md from the checkout; phase comparison cannot proceed without it`];
  const basePhase = inspectPhaseInText(baseText.stdout);
  const headPhase = inspectPhaseInText(headText.stdout);
  const errors = [];
  if (basePhase === "invalid") errors.push("base phase: repair the malformed matrix phase before comparing lifecycle coverage");
  if (headPhase === "invalid") errors.push("head phase: repair the malformed matrix phase before comparing lifecycle coverage");
  if (basePhase === "present" && headPhase === "absent") errors.push("head phase: restore the matrix closure phase; removing it reopens finite lifecycle coverage");
  if (shouldRunRangeGuard(basePhase, headPhase)) {
    const raw = gitRawDiff(repoRoot, base, head);
    if (raw.status !== 0) {
      errors.push(`git diff: inspect ${base}..${head}; unable to read closure range: ${raw.stderr.trim()}`);
    } else {
      errors.push(...validateRawOperations(parseRawDiff(raw.stdout)));
    }
  }
  return errors;
}

function validateMatrixBlocks(blocks) {
  const errors = [];
  const expectedKinds = ["paragraph", "paragraph", "table", "heading4", "table"];
  if (blocks.length !== expectedKinds.length) {
    errors.push(`Closed Client-Platform Matrix: restore exactly five blocks; found ${blocks.length}`);
    return errors;
  }
  expectedKinds.forEach((kind, index) => {
    if (blocks[index].kind !== kind) errors.push(`Closed Client-Platform Matrix block ${index + 1}: restore ${kind}`);
  });
  if (blocks[0].text.trim() !== "Closed client-platform row count: `2`.") errors.push("row count: restore exact literal `2`; finite lifecycle coverage depends on the checked table size");
  if (blocks[1].text.trim() !== phaseText) errors.push("phase: restore exact matrix closure phase paragraph");
  const matrix = tableRows(blocks[2]);
  validateCompatibilityMatrix(errors, matrix);
  const keys = matrix.slice(2).map((row) => `${stripCode(row[0] ?? "")}/${stripCode(row[1] ?? "")}`);
  expectedKeys.forEach((key, index) => {
    if (keys[index] !== key) errors.push(`compatibility row ${index + 1}: restore exact key ${key}; ranges, wildcards, placeholders, and catch-all cells cannot prove finite coverage`);
  });
  if (new Set(keys).size !== keys.length) errors.push("compatibility matrix: remove duplicate client-platform key; each finite cell must be unique");
  if (blocks[3].text.trim() !== "#### Matrix Reference Registry") errors.push("registry heading: restore exact heading");
  assertExactRows(errors, "registry", tableRows(blocks[4]), [["Reference ID", "Document", "Heading", "Block", "Label", "Destination"], ["---", "---", "---", "---", "---", "---"], ...expectedReferences.map((row) => row.map((cell, index) => index === 3 ? cell : `\`${cell}\``))]);
  return errors;
}

function validateCompatibilityMatrix(errors, rows) {
  const header = rows[0] ?? [];
  const separator = rows[1] ?? [];
  const dataRows = rows.slice(2);
  const expectedHeader = expectedFields.map((field) => `\`${field}\``);
  assertExactRows(errors, "compatibility matrix", rows, [expectedHeader, expectedFields.map(() => "---"), ...expectedCellRows.map((row) => row.map((cell) => `\`${cell}\``))]);
  if (header.length !== expectedFields.length) {
    errors.push(`compatibility header: restore exactly ${expectedFields.length} fields; each cell must carry every compatibility value`);
  }
  expectedFields.forEach((field, index) => {
    if (stripCode(header[index] ?? "") !== field) {
      errors.push(`compatibility field ${index + 1}: restore ${field}; lifecycle inventory depends on one exact cell shape`);
    }
  });
  if (separator.length !== expectedFields.length || separator.some((cell) => cell !== "---")) {
    errors.push("compatibility separator: restore one markdown separator cell for every compatibility field");
  }
  if (dataRows.length !== expectedCellRows.length) {
    errors.push(`compatibility matrix: restore exactly ${expectedCellRows.length} data rows`);
  }
  dataRows.forEach((row, rowIndex) => validateCompatibilityRow(errors, row, rowIndex));
}

function validateCompatibilityRow(errors, row, rowIndex) {
  if (row.length !== expectedFields.length) {
    errors.push(`compatibility row ${rowIndex + 1}: restore ${expectedFields.length} cells; lifecycle coverage depends on complete client-platform records`);
    return;
  }
  row.forEach((cell, fieldIndex) => {
    const field = expectedFields[fieldIndex];
    const value = stripCode(cell);
    if (!/^`[^`]+`$/.test(cell)) {
      errors.push(`compatibility row ${rowIndex + 1} ${field}: use one nonempty code-spanned exact literal so the cell can be checked mechanically`);
    }
    if (!isExactLiteral(value)) {
      errors.push(`compatibility row ${rowIndex + 1} ${field}: replace "${value}" with one exact nonempty literal; ranges, wildcards, placeholders, and catch-all values cannot prove finite lifecycle coverage`);
    }
    if (field === "validator-integrity" && !/^[0-9a-f]{64}$/.test(value)) {
      errors.push(`compatibility row ${rowIndex + 1} validator-integrity: use exactly 64 lowercase hexadecimal digits so validator provenance is pinned`);
    }
  });
}

function validateReferences(docs) {
  const errors = [];
  const found = [];
  for (const [file, text] of Object.entries(docs)) {
    const blocks = parseBlocks(text, file);
    for (const block of blocks) {
      if (block.kind === "table") {
        for (const row of block.text.split("\n")) {
          for (const link of matrixLinks(row)) {
            found.push({ file, block: { ...block, text: row }, ...link });
          }
        }
      } else {
        for (const link of matrixLinks(block.text)) {
          found.push({ file, block, ...link });
        }
      }
    }
  }
  for (const ref of expectedReferences) {
    const [id, file, heading, blockName, label, destination] = ref;
    const matches = found.filter((item) => item.title === `matrix-ref:${id}`);
    if (matches.length !== 1) {
      errors.push(`matrix-ref:${id}: restore exactly one target link; lifecycle coverage depends on every registered reference being present once`);
      continue;
    }
    const match = matches[0];
    const actualBlock = blockIdentity(match.block);
    const actualHeading = heading.startsWith("### ") ? match.block.subowner : match.block.owner;
    if (match.file !== file || match.label !== label || match.destination !== destination || actualHeading !== heading || actualBlock !== blockName) {
      errors.push(`matrix-ref:${id}: restore ${file} ${heading} ${blockName} -> ${destination}; found ${match.file} ${actualHeading} ${actualBlock} -> ${match.destination}`);
    }
  }
  for (const item of found) {
    const id = item.title?.replace(/^matrix-ref:/, "");
    if (!expectedReferences.some((ref) => ref[0] === id)) {
      errors.push(`${item.file}:${item.block.line}: remove unregistered matrix reference ${item.title ?? item.destination}; only registry tuples may define the closed universe`);
    }
  }
  return errors;
}

function validateNoHiddenMatrixEvidence(text, file) {
  const errors = [];
  let blocks = [];
  try {
    blocks = parseBlocks(text, file);
  } catch {
    return errors;
  }
  for (const block of blocks) {
    if (!["fence", "html", "quote", "indented-code"].includes(block.kind)) {
      continue;
    }
    for (const phrase of ["Closed Client-Platform Matrix", phaseText]) {
      if (block.text.includes(phrase)) {
        errors.push(`${file}:${block.line}: remove hidden or displaced "${phrase}" evidence; only visible column-zero matrix blocks count`);
      }
    }
  }
  return errors;
}

function parseBlocks(text, file = "<text>") {
  const normalized = text.replaceAll("\r\n", "\n");
  const lines = normalized.split("\n");
  const blocks = [];
  let owner = "";
  let subowner = "";
  const counts = new Map();
  for (let i = 0; i < lines.length;) {
    const line = lines[i];
    if (line.trim() === "") {
      i += 1;
    } else if (/^#{2} /.test(line)) {
      owner = line.trim();
      subowner = "";
      blocks.push({ kind: "heading2", text: line, line: i + 1, owner, subowner });
      i += 1;
    } else if (/^#{3} /.test(line)) {
      subowner = line.trim();
      blocks.push({ kind: "heading3", text: line, line: i + 1, owner, subowner });
      i += 1;
    } else if (/^#{4} /.test(line)) {
      blocks.push(numbered({ kind: "heading4", text: line, line: i + 1, owner, subowner }, counts));
      i += 1;
    } else if (/^(```|~~~)/.test(line)) {
      const mark = line.slice(0, 3);
      let j = i + 1;
      while (j < lines.length && !lines[j].startsWith(mark)) j += 1;
      if (j >= lines.length) throw new Error(`${file}:${i + 1}: close fenced code before matrix validation can continue`);
      blocks.push(numbered({ kind: "fence", text: lines.slice(i, j + 1).join("\n"), line: i + 1, owner, subowner }, counts));
      i = j + 1;
    } else if (line.startsWith("<!--")) {
      let j = i;
      while (j < lines.length && !lines[j].includes("-->")) j += 1;
      if (j >= lines.length) throw new Error(`${file}:${i + 1}: close HTML comment before matrix validation can continue`);
      blocks.push(numbered({ kind: "html", text: lines.slice(i, j + 1).join("\n"), line: i + 1, owner, subowner }, counts));
      i = j + 1;
    } else if (/^<[A-Za-z]/.test(line)) {
      let j = i + 1;
      while (j < lines.length && lines[j].trim() !== "") j += 1;
      blocks.push(numbered({ kind: "html", text: lines.slice(i, j).join("\n"), line: i + 1, owner, subowner }, counts));
      i = j;
    } else if (/^( {4}|>)/.test(line)) {
      blocks.push(numbered({ kind: line.startsWith(">") ? "quote" : "indented-code", text: line, line: i + 1, owner, subowner }, counts));
      i += 1;
    } else if (line.startsWith("|")) {
      let j = i;
      while (j < lines.length && lines[j].startsWith("|")) {
        if (!lines[j].endsWith("|")) throw new Error(`${file}:${j + 1}: finish malformed table row or remove it; validation must fail instead of stalling`);
        j += 1;
      }
      blocks.push(numbered({ kind: "table", text: lines.slice(i, j).join("\n"), line: i + 1, owner, subowner }, counts));
      i = j;
    } else if (line.startsWith("- ")) {
      let j = i + 1;
      while (j < lines.length && lines[j].startsWith("  ")) j += 1;
      blocks.push(numbered({ kind: "list item", text: lines.slice(i, j).join("\n"), line: i + 1, owner, subowner }, counts));
      i = j;
    } else if (/^\S/.test(line)) {
      let j = i + 1;
      while (j < lines.length && lines[j].trim() !== "" && /^\S/.test(lines[j]) && !/^#{2,4} /.test(lines[j]) && !lines[j].startsWith("|") && !lines[j].startsWith("- ")) j += 1;
      blocks.push(numbered({ kind: "paragraph", text: lines.slice(i, j).join("\n"), line: i + 1, owner, subowner }, counts));
      i = j;
    } else {
      let j = i + 1;
      while (j < lines.length && lines[j].trim() !== "" && !/^\S/.test(lines[j])) j += 1;
      blocks.push(numbered({ kind: "indented-continuation", text: lines.slice(i, j).join("\n"), line: i + 1, owner, subowner }, counts));
      i = j;
    }
  }
  return blocks;
}

function matrixInterval(blocks) {
  const matrixHeadings = blocks.filter((block) => block.kind === "heading3" && block.text.trim() === "### Closed Client-Platform Matrix");
  if (matrixHeadings.length !== 1) return { errors: ["Closed Client-Platform Matrix: restore exactly one top-level matrix heading"], blocks: [] };
  const heading = matrixHeadings[0];
  if (heading.owner !== "## Agent Plugin") return { errors: ["Closed Client-Platform Matrix: move the matrix under ## Agent Plugin"], blocks: [] };
  const start = blocks.indexOf(heading) + 1;
  const end = blocks.findIndex((block, index) => index >= start && (block.kind === "heading2" || block.kind === "heading3"));
  const interval = blocks.slice(start, end === -1 ? blocks.length : end);
  const next = blocks[end];
  if (next?.text.trim() !== "## Safety And Privacy") return { errors: ["Closed Client-Platform Matrix: keep the matrix as the final Agent Plugin subsection before ## Safety And Privacy"], blocks: interval };
  return { errors: [], blocks: interval };
}

function numbered(block, counts) {
  const key = `${block.owner}\0${block.kind}`;
  const count = (counts.get(key) ?? 0) + 1;
  counts.set(key, count);
  return { ...block, ordinal: count };
}

function blockIdentity(block) {
  if (block.kind === "paragraph") return `paragraph ${block.ordinal}`;
  if (block.kind === "list item") return `list item ${block.ordinal}`;
  if (block.kind === "table") {
    const first = tableRows(block).find((row) => row.some((cell) => cell.trim() !== ""));
    if (first?.[0] && first[0] !== "---") return `table row \`${stripCode(first[0]).replace(/\.$/, "")}\``;
  }
  return `${block.kind} ${block.ordinal}`;
}

function tableRows(block) {
  return block.text.split("\n").map((line) => line.slice(1, -1).split("|").map((cell) => cell.trim()));
}

function assertExactRows(errors, label, actual, expected) {
  if (actual.length !== expected.length) {
    errors.push(`${label}: restore exactly ${expected.length - 2} data rows; found ${Math.max(0, actual.length - 2)}`);
    return;
  }
  expected.forEach((row, rowIndex) => {
    if (actual[rowIndex]?.length !== row.length || row.some((cell, cellIndex) => actual[rowIndex][cellIndex] !== cell)) {
      errors.push(`${label} row ${rowIndex + 1}: restore exact cells ${row.join(" | ")}`);
    }
  });
}

function matrixLinks(text) {
  const links = [];
  const pattern = /\[([^\]\\]+)\]\(([^)\s]+) "([^"]+)"\)/g;
  for (const match of text.matchAll(pattern)) {
    if (match[3].startsWith("matrix-ref:") || ["#closed-client-platform-matrix", "agent-language-services.md#closed-client-platform-matrix"].includes(match[2])) {
      links.push({ label: match[1], destination: match[2], title: match[3] });
    }
  }
  return links;
}

function stripCode(value) {
  return value.replace(/^`|`$/g, "");
}

function isExactLiteral(value) {
  if (value.trim() === "") return false;
  if (/[*?]/.test(value)) return false;
  if (/\b(all|any|placeholder|supported|tbd|todo|future)\b/i.test(value)) return false;
  if (/^x+$|^n\/a$/i.test(value)) return false;
  if (/\.\.|<|>|\[[^\]]+\]|\{[^}]+\}/.test(value)) return false;
  return true;
}

function parseRawDiff(output) {
  const parts = output.split("\0").filter(Boolean);
  const operations = [];
  for (let i = 0; i < parts.length; i += 2) {
    const meta = parts[i];
    const file = parts[i + 1];
    const match = /^:([0-7]{6}) ([0-7]{6}) [0-9a-f]+ [0-9a-f]+ ([A-Z])/.exec(meta);
    if (match && file) operations.push({ oldMode: match[1], newMode: match[2], status: match[3], file });
  }
  return operations;
}

function gitRawDiff(repoRoot, base, head) {
  return spawnSync("git", ["diff", "--raw", "--no-renames", "-z", base, head], { cwd: repoRoot, encoding: "utf8" });
}

function gitShow(repoRoot, rev, file) {
  return spawnSync("git", ["show", `${rev}:${file}`], { cwd: repoRoot, encoding: "utf8" });
}

function readMaybe(file) {
  return fs.existsSync(file) ? fs.readFileSync(file, "utf8") : undefined;
}

function nextStepIndex(text, start) {
  const next = text.slice(start).search(/^      - /m);
  return next === -1 ? text.length : start + next;
}

function nextTopLevelKeyIndex(text, start) {
  const next = text.slice(start).search(/^  [A-Za-z0-9_-]+:/m);
  return next === -1 ? text.length : start + next;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}
