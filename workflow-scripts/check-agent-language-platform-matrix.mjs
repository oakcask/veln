import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const proposalPath = "docs/proposals/agent-language-services.md";
const lifecyclePath = "docs/proposals/agent-language-services-lifecycle-migration.md";
const workflowPath = ".github/workflows/workflow--test-scripts.yaml";
const expectedKeys = [
  "codex/x86_64-unknown-linux-gnu",
  "claude-code/x86_64-unknown-linux-gnu",
];
const expectedFields = [
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
const expectedPhase = "Matrix closure phase: `agent-language-services-platform-matrix-closed`.";
const expectedRowCount = "Closed client-platform row count: `2`.";
const expectedReferences = [
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
const expectedOperations = [
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

if (isMainModule()) {
  const repoRoot = process.cwd();
  const result = validateRepository({ repoRoot });
  if (!result.valid) {
    console.error(
      "Restore the closed agent language platform matrix contract before merging; lifecycle migration can prove finite coverage only when each row, reference, and closure path is exact.",
    );
    for (const error of result.errors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }
  console.log("Closed agent language platform matrix is valid.");
}

export function validateRepository({ repoRoot, baseSha = process.env.AGENT_PLATFORM_MATRIX_BASE_SHA, headSha = process.env.AGENT_PLATFORM_MATRIX_HEAD_SHA } = {}) {
  const errors = [];
  const documents = new Map();
  for (const file of [proposalPath, lifecyclePath]) {
    const absolute = path.join(repoRoot, file);
    if (!fs.existsSync(absolute)) {
      errors.push(`${file}: restore this document so matrix references have an authoritative target`);
      continue;
    }
    documents.set(file, fs.readFileSync(absolute, "utf8"));
  }
  if (documents.has(proposalPath)) {
    errors.push(...validateMatrixDocument(documents.get(proposalPath)));
  }
  if (documents.size === 2) {
    errors.push(...validateMatrixReferences(documents));
  }
  errors.push(...validateWorkflowRegistration({ repoRoot }));
  errors.push(...validateClosureRange({ repoRoot, baseSha, headSha }));
  return { errors, valid: errors.length === 0 };
}

export function validateWorkflowRegistration({ repoRoot }) {
  const file = path.join(repoRoot, workflowPath);
  if (!fs.existsSync(file)) {
    return [`${workflowPath}: restore the documentation-validation workflow so CI checks the closed platform matrix`];
  }
  const text = fs.readFileSync(file, "utf8");
  const expected = [
    "      - name: Validate the closed agent language platform matrix",
    "        env:",
    "          AGENT_PLATFORM_MATRIX_BASE_SHA: ${{ github.event.pull_request.base.sha || github.event.before }}",
    "          AGENT_PLATFORM_MATRIX_HEAD_SHA: ${{ github.sha }}",
    "        run: node workflow-scripts/check-agent-language-platform-matrix.mjs",
  ].join("\n");
  if (!text.includes(expected)) {
    return [`${workflowPath}: restore the exact "Validate the closed agent language platform matrix" step with base/head environment values so CI checks the authoritative range`];
  }
  if ((text.match(/Validate the closed agent language platform matrix/g) ?? []).length !== 1) {
    return [`${workflowPath}: keep exactly one closed platform matrix validation step so CI range evidence is unambiguous`];
  }
  return [];
}

export function validateMatrixDocument(text) {
  const errors = [];
  const blocks = parseBlocks(text, proposalPath, errors);
  const matrixHeadings = blocks.filter((block) => block.type === "heading" && block.level === 3 && block.text === "Closed Client-Platform Matrix");
  if (matrixHeadings.length !== 1) {
    errors.push(`${proposalPath}: restore exactly one top-level matrix heading named "### Closed Client-Platform Matrix"; finite lifecycle coverage needs one closed matrix source`);
    return errors;
  }
  const start = blocks.indexOf(matrixHeadings[0]) + 1;
  const end = blocks.findIndex((block, index) => index >= start && block.type === "heading" && block.level <= 3);
  const section = blocks.slice(start, end === -1 ? blocks.length : end);
  const layout = section.map((block) => block.type === "heading" ? `h${block.level}:${block.text}` : block.type);
  const expectedLayout = ["paragraph", "paragraph", "table", "h4:Compatibility Field Identities", "table", "h4:Matrix Reference Registry", "table"];
  if (JSON.stringify(layout) !== JSON.stringify(expectedLayout)) {
    errors.push(`${proposalPath}: restore the matrix section block order; lifecycle coverage depends on the row count, phase, membership table, field table, and registry appearing once in that order`);
    return errors;
  }
  validateParagraph(section[0], expectedRowCount, "row count", errors);
  validateParagraph(section[1], expectedPhase, "phase identity", errors);
  validateMembership(section[2], errors);
  validateFields(section[4], errors);
  validateRegistry(section[6], errors);
  return errors;
}

function validateParagraph(block, expected, label, errors) {
  if (block.text !== expected) {
    errors.push(`${proposalPath}:${block.line}: restore the exact ${label} paragraph "${expected}" so the validator can bind the finite matrix lifecycle`);
  }
}

function validateMembership(table, errors) {
  if (table.delimiter !== "| --- | --- |") {
    errors.push(`${proposalPath}:${table.line}: restore the membership table delimiter "| --- | --- |"; malformed tables cannot prove finite row coverage`);
  }
  if (JSON.stringify(table.header) !== JSON.stringify(["Client", "Platform"])) {
    errors.push(`${proposalPath}:${table.line}: restore membership headers "Client" and "Platform" so client-platform keys are explicit`);
  }
  const keys = table.rows.map((row) => `${stripCode(row[0])}/${stripCode(row[1])}`);
  compareOrdered({
    actual: keys,
    expected: expectedKeys,
    label: "client-platform key",
    errors,
  });
  for (const [index, row] of table.rows.entries()) {
    if (row.length !== 2 || row.some((cell) => !/^`[a-z0-9_.-]+`$/.test(cell))) {
      errors.push(`${proposalPath}:${table.line}: restore row ${index + 1} as exact inline-code client and platform literals; ranges, wildcards, and placeholders cannot prove finite coverage`);
    }
  }
}

function validateFields(table, errors) {
  if (table.delimiter !== "| --- |") {
    errors.push(`${proposalPath}:${table.line}: restore the compatibility field table delimiter "| --- |"; malformed tables cannot prove record shape`);
  }
  if (JSON.stringify(table.header) !== JSON.stringify(["Compatibility field"])) {
    errors.push(`${proposalPath}:${table.line}: restore the "Compatibility field" header so future record shape is explicit`);
  }
  for (const [rowIndex, row] of table.rows.entries()) {
    if (row.length !== 1 || !/^`[a-z0-9-]+`$/.test(row[0])) {
      errors.push(`${proposalPath}:${table.line}: restore compatibility field row ${rowIndex + 1} as one exact inline-code field identity with no value`);
    }
    if (row.join(" ").includes("=") || row.join(" ").includes(": ")) {
      errors.push(`${proposalPath}:${table.line}: remove compatibility value from field row ${rowIndex + 1}; this closure records identities only, not unchecked artifact values`);
    }
    if (/validator-integrity.*[0-9a-f]{1,63}|validator-integrity.*[0-9a-f]{65,}/.test(row.join(" "))) {
      errors.push(`${proposalPath}:${table.line}: remove unchecked validator-integrity digest text from row ${rowIndex + 1}; digest values must come from checked artifacts later`);
    }
  }
  compareOrdered({
    actual: table.rows.map((row) => stripCode(row[0])),
    expected: expectedFields,
    label: "compatibility field",
    errors,
  });
}

function validateRegistry(table, errors) {
  const expectedHeader = ["Reference ID", "Document", "Heading", "Block", "Label", "Destination"];
  if (table.delimiter !== "| --- | --- | --- | --- | --- | --- |") {
    errors.push(`${proposalPath}:${table.line}: restore the registry delimiter with six columns; malformed registry rows cannot bind references`);
  }
  if (JSON.stringify(table.header) !== JSON.stringify(expectedHeader)) {
    errors.push(`${proposalPath}:${table.line}: restore registry headers ${expectedHeader.join(", ")}`);
  }
  const rows = table.rows.map((row) => row.map(stripCode));
  compareOrdered({
    actual: rows.map((row) => row.join("\0")),
    expected: expectedReferences.map((row) => row.join("\0")),
    label: "matrix reference registry row",
    errors,
  });
}

export function validateMatrixReferences(documents) {
  const errors = [];
  const observed = [];
  for (const [file, text] of documents) {
    const parseErrors = [];
    const blocks = parseBlocks(text, file, parseErrors);
    errors.push(...parseErrors);
    observed.push(...matrixLinksInBlocks({ file, blocks }));
  }
  const expected = expectedReferences.map(([id, document, heading, block, label, destination]) => ({
    id,
    file: document === "agent-language-services.md" ? proposalPath : lifecyclePath,
    heading,
    block,
    label,
    destination,
  }));
  compareOrdered({
    actual: observed.map(referenceKey),
    expected: expected.map(referenceKey),
    label: "matrix reference",
    errors,
  });
  return errors;
}

function matrixLinksInBlocks({ file, blocks }) {
  const links = [];
  const counters = new Map();
  let h2 = undefined;
  let h3 = undefined;
  for (const block of blocks) {
    if (block.type === "heading") {
      if (block.level === 2) {
        h2 = `## ${block.text}`;
        h3 = undefined;
        counters.clear();
      } else if (block.level === 3) {
        h3 = `### ${block.text}`;
      }
      continue;
    }
    const heading = h3 && expectedReferences.some((row) => row[2] === h3) ? h3 : h2;
    const kind = block.type === "list_item" ? "list item" : block.type;
    if (block.type === "paragraph" || block.type === "list_item") {
      const key = `${heading}\0${kind}`;
      counters.set(key, (counters.get(key) ?? 0) + 1);
      for (const link of exactLinks(block.text)) {
        if (link.title.startsWith("matrix-ref:") || matrixDestinations().has(link.destination)) {
          links.push({ id: link.title.replace(/^matrix-ref:/, ""), file, heading, block: `${kind} ${counters.get(key)}`, label: link.label, destination: link.destination });
        }
      }
    }
    if (block.type === "table") {
      for (const row of block.rows) {
        const rowName = stripCode(row[0] ?? "");
        const blockName = `table row \`${rowName}\``;
        for (const cell of row) {
          for (const link of exactLinks(cell)) {
            if (link.title.startsWith("matrix-ref:") || matrixDestinations().has(link.destination)) {
              links.push({ id: link.title.replace(/^matrix-ref:/, ""), file, heading, block: blockName, label: link.label, destination: link.destination });
            }
          }
        }
      }
    }
  }
  return links.filter((link) => link.id !== "");
}

function exactLinks(text) {
  const links = [];
  const visible = text.replace(/`[^`]*`/g, "");
  const pattern = /(?<!!)\[([^\]\n\\&`]+)\]\(([^()\s]+) "([^"\n]+)"\)/g;
  for (const match of visible.matchAll(pattern)) {
    links.push({ label: match[1], destination: match[2], title: match[3] });
  }
  return links;
}

function matrixDestinations() {
  return new Set(expectedReferences.map((row) => row[5]));
}

function referenceKey(reference) {
  return [reference.id, reference.file, reference.heading, reference.block, reference.label, reference.destination].join("\0");
}

export function validateClosureRange({ repoRoot, baseSha, headSha }) {
  const errors = [];
  if (!baseSha || !headSha) {
    return ["set AGENT_PLATFORM_MATRIX_BASE_SHA and AGENT_PLATFORM_MATRIX_HEAD_SHA so the closure guard checks the authoritative CI range"];
  }
  if (/^0+$/.test(baseSha) || /^0+$/.test(headSha)) {
    return ["replace all-zero closure range revision with a real commit; finite lifecycle coverage depends on comparing the actual base and head trees"];
  }
  const basePhase = phasePresenceAtRevision({ repoRoot, revision: baseSha, file: proposalPath });
  const headPhase = phasePresenceAtRevision({ repoRoot, revision: headSha, file: proposalPath });
  if (basePhase.error) errors.push(basePhase.error);
  if (headPhase.error) errors.push(headPhase.error);
  if (errors.length > 0) return errors;
  if (basePhase.present || !headPhase.present) {
    return [];
  }
  const operations = rawDiffOperations({ repoRoot, baseSha, headSha });
  if (operations.error) {
    return [operations.error];
  }
  return validatePathOperations(operations.operations);
}

export function validatePathOperations(operations) {
  const errors = [];
  const actualByPath = new Map(operations.map((operation) => [operation.path, operation]));
  for (const [status, oldMode, newMode, file] of expectedOperations) {
    const actual = actualByPath.get(file);
    if (!actual) {
      errors.push(`${file}: restore required ${status} path operation; the closure transition must remain documentation-only so lifecycle coverage has a finite review range`);
      continue;
    }
    if (actual.status !== status || actual.oldMode !== oldMode || actual.newMode !== newMode) {
      errors.push(`${file}: restore ${status} ${oldMode}->${newMode}; found ${actual.status} ${actual.oldMode}->${actual.newMode}, which changes the protected closure range`);
    }
  }
  const expectedPaths = new Set(expectedOperations.map((operation) => operation[3]));
  for (const operation of operations) {
    if (!expectedPaths.has(operation.path)) {
      errors.push(`${operation.path}: remove this ${operation.status} ${operation.oldMode}->${operation.newMode} change or move it to another PR; finite lifecycle coverage depends on the exact closure manifest`);
    }
    if (!["M", "A", "D"].includes(operation.status)) {
      errors.push(`${operation.path}: replace Git type ${operation.status} with the exact documented add, modify, or delete operation; rename/copy/type changes are outside the closure manifest`);
    }
  }
  return errors;
}

function phasePresenceAtRevision({ repoRoot, revision, file }) {
  const result = spawnSync("git", ["show", `${revision}:${file}`], { cwd: repoRoot, encoding: "utf8" });
  if (result.status !== 0) {
    if (result.stderr.includes("exists on disk, but not in")) {
      return { present: false };
    }
    return { error: `${file}: cannot read ${revision}; restore a readable closure range so the phase guard can compare base and head` };
  }
  const errors = [];
  const blocks = parseBlocks(result.stdout, file, errors);
  if (errors.length > 0) {
    return { present: false };
  }
  return { present: blocks.some((block) => block.type === "paragraph" && block.text === expectedPhase) };
}

function rawDiffOperations({ repoRoot, baseSha, headSha }) {
  const result = spawnSync("git", ["diff", "--raw", "--no-renames", "-z", baseSha, headSha], { cwd: repoRoot, encoding: "buffer" });
  if (result.status !== 0) {
    return { error: `unable to read closure tree delta: ${result.stderr.toString("utf8").trim()}` };
  }
  const parts = result.stdout.toString("utf8").split("\0").filter(Boolean);
  const operations = [];
  for (let index = 0; index < parts.length; index += 2) {
    const meta = parts[index];
    const file = parts[index + 1];
    const match = /^:(\d{6}) (\d{6}) [0-9a-f]+ [0-9a-f]+ ([A-Z])/.exec(meta);
    if (!match || !file) {
      return { error: "unable to parse closure tree delta; restore a standard git raw diff range" };
    }
    operations.push({ oldMode: match[1], newMode: match[2], status: match[3], path: file });
  }
  return { operations };
}

function parseBlocks(text, file, errors = []) {
  const lines = text.replaceAll("\r\n", "\n").split("\n");
  const blocks = [];
  for (let index = 0; index < lines.length;) {
    const line = lines[index];
    if (line.trim() === "") {
      index += 1;
      continue;
    }
    if (/^(`{3,}|~{3,})/.test(line)) {
      const fence = /^(`{3,}|~{3,})/.exec(line)[1];
      const char = fence[0];
      const size = fence.length;
      index += 1;
      while (index < lines.length && !new RegExp(`^\\${char}{${size},}\\s*$`).test(lines[index])) index += 1;
      if (index >= lines.length) errors.push(`${file}:${lines.length}: close the fenced code block; hidden matrix text cannot satisfy the contract`);
      index += 1;
      continue;
    }
    if (line.startsWith("<!--")) {
      while (index < lines.length && !lines[index].includes("-->")) index += 1;
      if (index >= lines.length) errors.push(`${file}:${lines.length}: close the HTML comment; hidden matrix text cannot satisfy the contract`);
      index += 1;
      continue;
    }
    if (/^<[A-Za-z!/]/.test(line)) {
      index += 1;
      while (index < lines.length && lines[index].trim() !== "") index += 1;
      continue;
    }
    if (/^ {4}/.test(line) || /^>/.test(line)) {
      index += 1;
      continue;
    }
    const heading = /^(#{1,6}) ([^#].*?)\s*$/.exec(line);
    if (heading) {
      blocks.push({ type: "heading", level: heading[1].length, text: heading[2], line: index + 1 });
      index += 1;
      continue;
    }
    if (line.startsWith("|")) {
      const startLine = index + 1;
      const tableLines = [];
      while (index < lines.length && lines[index].startsWith("|") && lines[index].endsWith("|")) {
        tableLines.push(lines[index]);
        index += 1;
      }
      blocks.push(parseTable(tableLines, startLine, file, errors));
      continue;
    }
    if (line.startsWith("- ")) {
      const startLine = index + 1;
      const itemLines = [line.slice(2)];
      index += 1;
      while (index < lines.length && lines[index].startsWith("  ")) {
        itemLines.push(lines[index].slice(2));
        index += 1;
      }
      blocks.push({ type: "list_item", text: itemLines.join(" "), line: startLine });
      continue;
    }
    if (/^\s/.test(line)) {
      index += 1;
      continue;
    }
    const startLine = index + 1;
    const paragraph = [];
    while (index < lines.length && lines[index].trim() !== "" && !/^(#{1,6}) /.test(lines[index]) && !lines[index].startsWith("|") && !lines[index].startsWith("- ")) {
      paragraph.push(lines[index]);
      index += 1;
    }
    blocks.push({ type: "paragraph", text: paragraph.join(" "), line: startLine });
  }
  return blocks;
}

function parseTable(lines, line, file, errors) {
  for (const [offset, raw] of lines.entries()) {
    const codeSpans = [...raw.matchAll(/`([^`]*)`/g)].map((match) => match[1]);
    if (raw.includes("\\") || raw.includes("&") || raw.includes("<") || codeSpans.some((span) => span.includes("|"))) {
      errors.push(`${file}:${line + offset}: remove escapes, entities, raw HTML, or code-span pipes from the contract table; table cells must be exact literals`);
    }
  }
  const split = (raw) => raw.slice(1, -1).split("|").map((cell) => cell.trim());
  return {
    type: "table",
    line,
    delimiter: lines[1] ?? "",
    header: split(lines[0] ?? ""),
    rows: lines.slice(2).map(split),
  };
}

function compareOrdered({ actual, expected, label, errors }) {
  if (JSON.stringify(actual) === JSON.stringify(expected)) {
    return;
  }
  const seen = new Set();
  for (const [index, item] of actual.entries()) {
    if (item === "") errors.push(`row ${index + 1}: restore nonempty ${label}; empty values cannot prove finite coverage`);
    if (seen.has(item)) errors.push(`row ${index + 1}: remove duplicate ${label} "${item.replaceAll("\0", " / ")}"`);
    seen.add(item);
    if (expected[index] !== item) {
      errors.push(`row ${index + 1}: restore ${label} "${(expected[index] ?? "<none>").replaceAll("\0", " / ")}"; found "${item.replaceAll("\0", " / ")}"`);
    }
  }
  for (let index = actual.length; index < expected.length; index += 1) {
    errors.push(`row ${index + 1}: add missing ${label} "${expected[index].replaceAll("\0", " / ")}"`);
  }
  if (actual.length > expected.length) {
    errors.push(`row ${expected.length + 1}: remove unexpected ${label}; the closed matrix has exactly ${expected.length} entries`);
  }
}

function stripCode(value) {
  return /^`[^`]*`$/.test(value) ? value.slice(1, -1) : value;
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}

export function makeTemporaryGitRepository() {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "veln-platform-matrix-"));
  spawnSync("git", ["init"], { cwd: repo, encoding: "utf8" });
  spawnSync("git", ["config", "user.name", "Veln Test"], { cwd: repo, encoding: "utf8" });
  spawnSync("git", ["config", "user.email", "veln-test@example.invalid"], { cwd: repo, encoding: "utf8" });
  return repo;
}
