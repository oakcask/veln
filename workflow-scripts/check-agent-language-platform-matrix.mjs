import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const agentDoc = "docs/proposals/agent-language-services.md";
const lifecycleDoc = "docs/proposals/agent-language-services-lifecycle-migration.md";
const recordDoc = "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md";
const workflowFile = ".github/workflows/workflow--test-scripts.yaml";

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

const expectedRows = [
  [
    "codex",
    "x86_64-unknown-linux-gnu",
    "linux-x86_64-host-contract-v1",
    "codex-plugin-manifest-v1",
    "agent-platform-matrix-validator-v1",
    "2f5c36e1d4a9b8c7e0f123456789abcd2f5c36e1d4a9b8c7e0f123456789abcd",
    "veln-toolchain-contract-v1",
    "mcp-contract-v1",
    "codex-lsp-disabled-contract-v1",
    "language-service-contract-v1",
    "reference-schema-contract-v1",
  ],
  [
    "claude-code",
    "x86_64-unknown-linux-gnu",
    "linux-x86_64-host-contract-v1",
    "claude-code-plugin-manifest-v1",
    "agent-platform-matrix-validator-v1",
    "7a4e29c0b1d3f85690abcdef123456787a4e29c0b1d3f85690abcdef12345678",
    "veln-toolchain-contract-v1",
    "mcp-contract-v1",
    "lsp-contract-v1",
    "language-service-contract-v1",
    "reference-schema-contract-v1",
  ],
];

const expectedRegistry = [
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
  const result = validateRepository({
    repoRoot: process.cwd(),
    baseSha: process.env.AGENT_PLATFORM_MATRIX_BASE_SHA,
    headSha: process.env.AGENT_PLATFORM_MATRIX_HEAD_SHA,
  });
  if (!result.valid) {
    console.error(
      "Restore the closed agent-language platform matrix before merging; lifecycle migration coverage depends on one finite matrix, exact registered references, and the documented closure range.",
    );
    for (const error of result.errors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }
  console.log("Closed agent-language platform matrix is valid.");
}

export function validateRepository({ repoRoot = process.cwd(), baseSha, headSha } = {}) {
  const documents = new Map([
    ["agent-language-services.md", readIfExists(path.join(repoRoot, agentDoc))],
    ["agent-language-services-lifecycle-migration.md", readIfExists(path.join(repoRoot, lifecycleDoc))],
  ]);
  const errors = [];
  if (readIfExists(path.join(repoRoot, "docs/proposals/agent-language-services-platform-matrix-closure.md")) !== undefined) {
    errors.push("docs/proposals/agent-language-services-platform-matrix-closure.md: move the completed proposal record out of docs/proposals so planned-work routing remains finite.");
  }
  if (readIfExists(path.join(repoRoot, recordDoc)) === undefined) {
    errors.push(`${recordDoc}: restore the implementation record so lifecycle prerequisites can cite completed matrix evidence.`);
  }
  errors.push(...validateDocuments(documents));
  errors.push(...validateWorkflow(readIfExists(path.join(repoRoot, workflowFile)) ?? ""));
  errors.push(...validateRange({ repoRoot, baseSha, headSha }));
  return finish(errors);
}

export function validateDocuments(documents) {
  const errors = [];
  const agentText = documents.get("agent-language-services.md");
  const lifecycleText = documents.get("agent-language-services-lifecycle-migration.md");
  if (agentText === undefined) {
    errors.push(`${agentDoc}: restore this proposal because the closed matrix is anchored there.`);
    return errors;
  }
  if (lifecycleText === undefined) {
    errors.push(`${lifecycleDoc}: restore this proposal because three matrix references are registered there.`);
    return errors;
  }

  const parsedAgent = parseMarkdown(agentText, "agent-language-services.md");
  const parsedLifecycle = parseMarkdown(lifecycleText, "agent-language-services-lifecycle-migration.md");
  errors.push(...parsedAgent.errors, ...parsedLifecycle.errors);
  errors.push(...validateMatrix(parsedAgent));
  errors.push(...validateReferences(new Map([
    ["agent-language-services.md", parsedAgent],
    ["agent-language-services-lifecycle-migration.md", parsedLifecycle],
  ])));
  errors.push(...validateNoUnboundPlatformUniverse(new Map([
    ["agent-language-services.md", parsedAgent],
    ["agent-language-services-lifecycle-migration.md", parsedLifecycle],
  ])));
  return errors;
}

export function inspectPhase(text) {
  const parsed = parseMarkdown(text ?? "", "agent-language-services.md");
  const matrix = matrixBlocks(parsed, []);
  if (matrix.state !== "present") {
    return matrix.state;
  }
  const phaseBlocks = matrix.blocks.filter((block) =>
    block.kind === "paragraph" &&
    block.text === "Matrix closure phase: `agent-language-services-platform-matrix-closed`."
  );
  return phaseBlocks.length === 1 && matrix.blocks[1] === phaseBlocks[0] ? "present" : "invalid";
}

export function validateRange({ repoRoot = process.cwd(), baseSha, headSha } = {}) {
  const errors = [];
  if (baseSha === undefined && headSha === undefined) {
    return errors;
  }
  if (!baseSha || !headSha) {
    return ["set AGENT_PLATFORM_MATRIX_BASE_SHA and AGENT_PLATFORM_MATRIX_HEAD_SHA so CI validates the authoritative revision range for finite lifecycle coverage."];
  }
  for (const [name, value] of [["AGENT_PLATFORM_MATRIX_BASE_SHA", baseSha], ["AGENT_PLATFORM_MATRIX_HEAD_SHA", headSha]]) {
    if (/^0+$/.test(value)) {
      errors.push(`${name}: replace the all-zero revision with a readable commit so the closure guard can prove the exact tree delta.`);
    } else if (!gitOk(repoRoot, ["rev-parse", "--verify", `${value}^{commit}`])) {
      errors.push(`${name}: revision ${value} is not readable; fetch full history so the matrix closure range can be checked.`);
    }
  }
  if (errors.length > 0) {
    return errors;
  }

  const baseText = gitShow(repoRoot, baseSha, agentDoc);
  const headText = gitShow(repoRoot, headSha, agentDoc);
  const basePhase = inspectPhase(baseText);
  const headPhase = inspectPhase(headText);
  if (basePhase === "invalid") {
    errors.push("base phase: repair the displaced, duplicate, hidden, or malformed closure phase before comparing lifecycle coverage.");
  }
  if (headPhase === "invalid") {
    errors.push("head phase: restore the canonical matrix closure phase paragraph so finite lifecycle coverage has one state identity.");
  }
  if (basePhase === "present" && headPhase === "absent") {
    errors.push("head phase: restore the closed matrix phase; removing it reopens the lifecycle prerequisite without a finite matrix identity.");
  }

  const runGuard = !((basePhase === "absent" && headPhase === "absent") || (basePhase === "present" && headPhase === "present"));
  if (runGuard) {
    errors.push(...validateTreeDelta(repoRoot, baseSha, headSha));
  }
  return errors;
}

export function validateWorkflow(text) {
  const errors = [];
  if (!/^on:\n(?:  push:[\s\S]*?  pull_request:|  pull_request:[\s\S]*?  push:)/m.test(text)) {
    errors.push(`${workflowFile}: include both push and pull_request triggers so CI checks the matrix on PRs and default-branch pushes.`);
  }
  for (const required of ["docs/**/*.md", "workflow-scripts/**/*.mjs", ".github/workflows/workflow--test-scripts.yaml"]) {
    if (!text.includes(`      - ${required}`)) {
      errors.push(`${workflowFile}: include path filter ${required} so matrix contract changes run documentation validation.`);
    }
  }
  if (!/push:\n(?:    [^\n]*\n)*    branches:\n      - main/m.test(text)) {
    errors.push(`${workflowFile}: run on pushes to main so the closure range is checked after merge.`);
  }
  if (/pull_request:\n    types:/m.test(text)) {
    errors.push(`${workflowFile}: remove pull_request event type restrictions so every PR update checks finite matrix coverage.`);
  }
  if (!text.includes("  test-workflow-scripts:")) {
    errors.push(`${workflowFile}: restore jobs.test-workflow-scripts so the matrix validator has one authoritative CI home.`);
    return errors;
  }
  const job = /^  test-workflow-scripts:\n([\s\S]*?)(?:\n  [a-zA-Z0-9_-]+:|\n*$)/m.exec(text)?.[1] ?? text;
  if (/^\s+(if|needs):/m.test(job)) {
    errors.push(`${workflowFile}: remove job if/needs gates from test-workflow-scripts so finite matrix validation cannot be skipped.`);
  }
  const stepName = "      - name: Validate the closed agent language platform matrix";
  const stepCount = text.split(stepName).length - 1;
  if (stepCount !== 1) {
    errors.push(`${workflowFile}: keep exactly one step named "Validate the closed agent language platform matrix" so registration is unambiguous.`);
    return errors;
  }
  const stepStart = text.indexOf(stepName);
  const nextStep = text.indexOf("\n      - ", stepStart + stepName.length);
  const step = nextStep === -1 ? text.slice(stepStart) : text.slice(stepStart, nextStep);
  if (!step.includes("        run: node workflow-scripts/check-agent-language-platform-matrix.mjs")) {
    errors.push(`${workflowFile}: run node workflow-scripts/check-agent-language-platform-matrix.mjs in the matrix step so CI validates the production contract.`);
  }
  for (const [key, value] of [
    ["AGENT_PLATFORM_MATRIX_BASE_SHA", "${{ github.event.pull_request.base.sha || github.event.before }}"],
    ["AGENT_PLATFORM_MATRIX_HEAD_SHA", "${{ github.sha }}"],
  ]) {
    if (!step.includes(`          ${key}: ${value}`)) {
      errors.push(`${workflowFile}: set ${key} to ${value} so the range guard sees the authoritative base/head revisions.`);
    }
  }
  if (/^\s+(if|continue-on-error|shell|working-directory):/m.test(step)) {
    errors.push(`${workflowFile}: remove if, continue-on-error, shell, or working-directory from the matrix step so validation cannot be softened or redirected.`);
  }
  return errors;
}

function validateMatrix(parsed) {
  const errors = [];
  const matrix = matrixBlocks(parsed, errors);
  if (matrix.state !== "present") {
    return errors;
  }
  const blocks = matrix.blocks;
  const expectedKinds = ["paragraph", "paragraph", "table", "heading", "table"];
  if (blocks.length !== expectedKinds.length) {
    errors.push(`agent-language-services.md#closed-client-platform-matrix: restore exactly five matrix blocks so lifecycle inventory coverage remains finite.`);
  }
  for (const [index, kind] of expectedKinds.entries()) {
    if (blocks[index]?.kind !== kind) {
      errors.push(`matrix block B${String(index + 1).padStart(2, "0")}: restore ${kind}; finite lifecycle coverage depends on the canonical matrix layout.`);
    }
  }
  if (blocks[0]?.text !== "Closed client-platform row count: `2`.") {
    errors.push("matrix row count: restore `2` so the checked row count matches the two closed cells.");
  }
  if (inspectPhase(parsed.text) !== "present") {
    errors.push("matrix phase: restore the canonical phase paragraph as block B02 so the closure guard can retire after this transition.");
  }
  validateCompatibilityTable(blocks[2], errors);
  if (blocks[3]?.text !== "#### Matrix Reference Registry") {
    errors.push("matrix registry heading: restore `#### Matrix Reference Registry` before the registry table.");
  }
  validateRegistryTable(blocks[4], errors);
  return errors;
}

function matrixBlocks(parsed, errors) {
  const headings = parsed.blocks.filter((block) => block.kind === "heading" && block.text === "### Closed Client-Platform Matrix");
  if (headings.length === 0) {
    errors?.push("agent-language-services.md: restore `### Closed Client-Platform Matrix` under `## Agent Plugin` so client-platform membership is finite.");
    return { state: "absent", blocks: [] };
  }
  if (headings.length !== 1) {
    errors?.push("agent-language-services.md: keep exactly one `### Closed Client-Platform Matrix` heading so no second platform universe exists.");
    return { state: "invalid", blocks: [] };
  }
  const heading = headings[0];
  if (heading.h2 !== "## Agent Plugin") {
    errors?.push(`${heading.file}:${heading.line}: move the closed matrix under ## Agent Plugin so plugin references bind to one matrix.`);
    return { state: "invalid", blocks: [] };
  }
  const interval = [];
  for (const block of parsed.blocks) {
    if (block.index <= heading.index) {
      continue;
    }
    if (block.kind === "heading" && block.level === 3) {
      errors?.push(`${block.file}:${block.line}: remove the level-three subsection after the closed matrix; the matrix must be the final Agent Plugin subsection.`);
      return { state: "invalid", blocks: interval };
    }
    if (block.kind === "heading" && block.level < 3) {
      break;
    }
    interval.push(block);
  }
  for (const block of interval) {
    if (!["paragraph", "table", "heading"].includes(block.kind) || (block.kind === "heading" && block.level !== 4)) {
      errors?.push(`${block.file}:${block.line}: remove unexpected ${block.kind} from the matrix interval; finite lifecycle coverage depends on the five canonical blocks.`);
      return { state: "invalid", blocks: interval };
    }
  }
  return { state: "present", blocks: interval };
}

function validateCompatibilityTable(block, errors) {
  const rows = tableRows(block, expectedFields, "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |", "compatibility table", errors);
  if (rows.length !== expectedKeys.length) {
    errors.push(`compatibility table: restore exactly ${expectedKeys.length} rows so lifecycle inventory coverage can prove every client-platform cell.`);
  }
  const seen = new Set();
  rows.forEach((row, index) => {
    const values = row.map(codeValue);
    if (values.some((value) => value === undefined)) {
      errors.push(`compatibility row ${index + 1}: use exact inline-code cells for every compatibility field so finite coverage is machine-checkable.`);
      return;
    }
    const key = `${values[0]}/${values[1]}`;
    if (seen.has(key)) {
      errors.push(`compatibility row ${index + 1}: remove duplicate ${key}; lifecycle coverage needs each closed cell once.`);
    }
    seen.add(key);
    if (key !== expectedKeys[index]) {
      errors.push(`compatibility row ${index + 1}: restore ${expectedKeys[index]}; ranges, wildcards, placeholders, catch-alls, and unexpected literals cannot prove finite coverage.`);
    }
    values.forEach((value, fieldIndex) => {
      const field = expectedFields[fieldIndex] ?? `column ${fieldIndex + 1}`;
      if (value === "") {
        errors.push(`compatibility row ${index + 1} field ${field}: enumerate a nonempty exact literal; empty values break finite lifecycle coverage.`);
      } else if (field === "validator-integrity" && !/^[0-9a-f]{64}$/.test(value)) {
        errors.push(`compatibility row ${index + 1} field validator-integrity: restore exactly 64 lowercase hexadecimal digits so validator evidence has a stable digest identity.`);
      } else if (field !== "validator-integrity" && !isExactLiteral(value)) {
        errors.push(`compatibility row ${index + 1} field ${field}: replace ${value} with the expected exact literal; ranges, wildcards, placeholders, and catch-alls cannot prove finite coverage.`);
      }
      if (expectedRows[index]?.[fieldIndex] !== undefined && value !== expectedRows[index][fieldIndex]) {
        errors.push(`compatibility row ${index + 1} field ${field}: restore ${expectedRows[index][fieldIndex]} so the closed matrix remains one authoritative literal table.`);
      }
    });
  });
}

function validateRegistryTable(block, errors) {
  const rows = tableRows(block, ["Reference ID", "Document", "Heading", "Block", "Label", "Destination"], "| --- | --- | --- | --- | --- | --- |", "registry table", errors);
  if (rows.length !== expectedRegistry.length) {
    errors.push(`registry table: restore ${expectedRegistry.length} rows so every matrix reference is registered exactly once.`);
  }
  rows.forEach((row, index) => {
    const values = row.map(cellValue);
    const expected = expectedRegistry[index];
    if (values.some((value) => value === undefined)) {
      errors.push(`registry row ${index + 1}: use exact inline-code cells so reference identity is machine-checkable.`);
      return;
    }
    for (let cell = 0; cell < expected.length; cell += 1) {
      if (values[cell] !== expected[cell]) {
        errors.push(`registry row ${index + 1}: restore ${expected[cell]} in column ${cell + 1}; lifecycle coverage depends on exact registered references.`);
      }
    }
  });
}

function validateReferences(documents) {
  const errors = [];
  const occurrences = [];
  for (const [doc, parsed] of documents) {
    for (const block of [...parsed.blocks.filter((block) => block.kind !== "table"), ...referenceTableRows(parsed)]) {
      for (const link of matrixLinks(block.text)) {
        occurrences.push({ doc, block, ...link });
      }
    }
  }
  for (const tuple of expectedRegistry) {
    const [id, doc, heading, blockSpec, label, destination] = tuple;
    const expectedSource = `[${label}](${destination} "matrix-ref:${id}")`;
    const matches = occurrences.filter((occurrence) => occurrence.id === id);
    if (matches.length !== 1) {
      errors.push(`reference ${id}: restore exactly one ${expectedSource}; finite lifecycle coverage needs one target for each registered tuple.`);
      continue;
    }
    const occurrence = matches[0];
    if (occurrence.doc !== doc || occurrence.block.label !== heading || blockLabel(occurrence.block) !== blockSpec) {
      errors.push(`reference ${id}: move ${expectedSource} back to ${doc} ${heading} ${blockSpec}; displaced links do not bind the registered matrix cell.`);
    }
    if (occurrence.source !== expectedSource) {
      errors.push(`reference ${id}: restore exact link source ${expectedSource}; alternate labels, destinations, titles, escapes, or entities cannot prove registry coverage.`);
    }
  }
  const expectedIds = new Set(expectedRegistry.map((tuple) => tuple[0]));
  const expectedDestinations = new Set(expectedRegistry.map((tuple) => tuple[5]));
  for (const occurrence of occurrences) {
    if (!expectedIds.has(occurrence.id) || !expectedDestinations.has(occurrence.destination)) {
      errors.push(`reference ${occurrence.id}: remove unregistered matrix link at ${occurrence.doc}:${occurrence.block.line}; add a registry tuple first so finite coverage remains closed.`);
    }
  }
  return errors;
}

function validateNoUnboundPlatformUniverse(documents) {
  const errors = [];
  const forbidden = /\b(?:all supported platforms|supported-platform|supported platforms|unnamed platform set|implicit platform set|future platform row)\b/i;
  for (const [doc, parsed] of documents) {
    for (const block of parsed.blocks) {
      if (forbidden.test(block.text)) {
        errors.push(`${doc}:${block.line}: remove the unbound supported-platform phrase and route the requirement to the Closed Client-Platform Matrix; lifecycle coverage depends on one finite platform universe.`);
      }
    }
  }
  return errors;
}

function validateTreeDelta(repoRoot, baseSha, headSha) {
  const actual = gitRawDiff(repoRoot, baseSha, headSha);
  if (actual.error !== undefined) {
    return [actual.error];
  }
  const errors = [];
  const expectedByPath = new Map(expectedOperations.map((op) => [op[3], op]));
  const actualByPath = new Map(actual.operations.map((op) => [op.path, op]));
  for (const expected of expectedOperations) {
    const [status, oldMode, newMode, file] = expected;
    const op = actualByPath.get(file);
    if (op === undefined) {
      errors.push(`${file}: restore required ${status} operation; the closure transition may only contain the nine documented path operations.`);
      continue;
    }
    if (op.status !== status || op.oldMode !== oldMode || op.newMode !== newMode) {
      errors.push(`${file}: restore ${status} ${oldMode}->${newMode}; Git type, executable-bit, rename, copy, or wrong-status changes mix out-of-scope work into finite lifecycle closure.`);
    }
  }
  for (const op of actual.operations) {
    if (!expectedByPath.has(op.path)) {
      errors.push(`${op.path}: remove this ${op.status} operation from the closure transition or move it to a later PR; extra paths invalidate the documentation-only matrix lifecycle proof.`);
    }
  }
  return errors;
}

export function parseMarkdown(text, file) {
  const normalized = text.replaceAll("\r\n", "\n");
  const lines = stripFrontmatter(normalized).split("\n");
  const blocks = [];
  const errors = [];
  let h2 = "";
  let h3 = "";
  let paragraphOrdinal = 0;
  let listOrdinal = 0;
  let index = 0;

  for (let i = 0; i < lines.length;) {
    const line = lines[i];
    const lineNo = i + 1;
    if (line.trim() === "") {
      i += 1;
      continue;
    }
    if (/^```|^~~~/.test(line)) {
      const fence = line.match(/^(`{3,}|~{3,})/)[1];
      const close = new RegExp(`^${escapeRegExp(fence[0])}{${fence.length},}\\s*$`);
      let j = i + 1;
      while (j < lines.length && !close.test(lines[j])) {
        j += 1;
      }
      if (j >= lines.length) {
        errors.push(`${file}:${lineNo}: close the fenced code block so hidden matrix text cannot be mistaken for evidence.`);
        break;
      }
      blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "fence", text: lines.slice(i, j + 1).join("\n"), h2, h3 }));
      i = j + 1;
      continue;
    }
    if (line.startsWith("<!--")) {
      let j = i;
      while (j < lines.length && !lines[j].includes("-->")) {
        j += 1;
      }
      if (j >= lines.length) {
        errors.push(`${file}:${lineNo}: close the HTML comment so hidden matrix text cannot be mistaken for evidence.`);
        break;
      }
      blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "html", text: lines.slice(i, j + 1).join("\n"), h2, h3 }));
      i = j + 1;
      continue;
    }
    if (/^<[A-Za-z][^>]*>/.test(line)) {
      let j = i + 1;
      while (j < lines.length && lines[j].trim() !== "") {
        j += 1;
      }
      blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "html", text: lines.slice(i, j).join("\n"), h2, h3 }));
      i = j;
      continue;
    }
    if (/^ {4}/.test(line)) {
      let j = i + 1;
      while (j < lines.length && /^ {4}/.test(lines[j])) {
        j += 1;
      }
      blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "indented-code", text: lines.slice(i, j).join("\n"), h2, h3 }));
      i = j;
      continue;
    }
    if (/^\s*>/.test(line)) {
      blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "blockquote", text: line, h2, h3 }));
      i += 1;
      continue;
    }
    const heading = /^(#{1,6}) (.+)$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      const textValue = `${heading[1]} ${heading[2]}`;
      if (level === 2) {
        h2 = textValue;
        h3 = "";
        paragraphOrdinal = 0;
        listOrdinal = 0;
      } else if (level === 3) {
        h3 = textValue;
      }
      blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "heading", level, text: textValue, h2, h3 }));
      i += 1;
      continue;
    }
    if (line.startsWith("|")) {
      let j = i + 1;
      while (j < lines.length && lines[j].startsWith("|")) {
        j += 1;
      }
      blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "table", text: lines.slice(i, j).join("\n"), h2, h3 }));
      i = j;
      continue;
    }
    if (line.startsWith("- ")) {
      let j = i + 1;
      while (j < lines.length && lines[j].startsWith("  ")) {
        j += 1;
      }
      listOrdinal += 1;
      blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "list", ordinal: listOrdinal, text: lines.slice(i, j).join("\n"), h2, h3 }));
      i = j;
      continue;
    }
    if (/^\s/.test(line)) {
      blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "indented-continuation", text: line, h2, h3 }));
      i += 1;
      continue;
    }
    let j = i + 1;
    while (j < lines.length && lines[j].trim() !== "" && !/^(#{1,6}) |^\||^- |^```|^~~~|^<!--|^<[A-Za-z]|^ {4}|^\s*>/.test(lines[j])) {
      j += 1;
    }
    paragraphOrdinal += 1;
    blocks.push(makeBlock({ file, line: lineNo, index: index++, kind: "paragraph", ordinal: paragraphOrdinal, text: lines.slice(i, j).join(" "), h2, h3 }));
    i = j;
  }
  return { file, text: normalized, blocks, errors };
}

function makeBlock(block) {
  return { ...block, label: block.h3 || block.h2 };
}

function blockLabel(block) {
  if (block.kind === "paragraph") {
    return `paragraph ${block.ordinal}`;
  }
  if (block.kind === "list") {
    return `list item ${block.ordinal}`;
  }
  if (block.kind === "table-row") {
    return `table row \`${block.rowName.replace(/\.$/, "")}\``;
  }
  return block.kind;
}

function tableRows(block, expectedHeader, expectedDelimiter, label, errors) {
  if (block?.kind !== "table") {
    errors.push(`${label}: restore the Markdown table so the matrix contract can be checked.`);
    return [];
  }
  const lines = block.text.split("\n");
  if (lines[1] !== expectedDelimiter) {
    errors.push(`${label}: restore delimiter ${expectedDelimiter}; malformed table delimiters cannot prove finite coverage.`);
  }
  const header = splitTableLine(lines[0]);
  if (header.join("\0") !== expectedHeader.join("\0")) {
    errors.push(`${label}: restore headers ${expectedHeader.join(", ")} so row meaning is unambiguous.`);
  }
  return lines.slice(2).map((line, index) => {
    const row = splitTableLine(line);
    if (row.length !== expectedHeader.length) {
      errors.push(`${label} row ${index + 1}: restore ${expectedHeader.length} cells so every declared column has exactly one value.`);
    }
    return row;
  });
}

function splitTableLine(line) {
  return line.slice(1, -1).split("|").map((cell) => cell.trim());
}

function codeValue(cell) {
  const match = /^`([^`]*)`$/.exec(cell ?? "");
  return match?.[1];
}

function cellValue(cell) {
  return codeValue(cell) ?? cell;
}

function isExactLiteral(value) {
  return !/(?:\*|\.{2}|<|>|^all$|^any$|^latest$|^current$|^future$|^tbd$|^todo$|placeholder|supported)/i.test(value);
}

function matrixLinks(text) {
  const links = [];
  const pattern = /(!?)\[([^\]\\]*(?:\\.[^\]\\]*)*)\]\(([^)\s]+)(?: "([^"]*)")?\)/g;
  for (const match of text.matchAll(pattern)) {
    if (match[1] === "!") {
      continue;
    }
    const [source, , label, destination, title = ""] = match;
    if (title.startsWith("matrix-ref:") || destination === "#closed-client-platform-matrix" || destination === "agent-language-services.md#closed-client-platform-matrix") {
      links.push({ source, label, destination, title, id: title.replace(/^matrix-ref:/, "") });
    }
  }
  return links;
}

export function referenceTableRows(parsed) {
  return parsed.blocks.flatMap((block) => {
    if (block.kind !== "table") {
      return [];
    }
    return block.text.split("\n").slice(2).map((line) => {
      const row = splitTableLine(line);
      return makeBlock({
        file: block.file,
        line: block.line,
        index: block.index,
        kind: "table-row",
        rowName: codeValue(row[0]) ?? row[0],
        text: line,
        h2: block.h2,
        h3: block.h3,
      });
    });
  });
}

function readIfExists(file) {
  try {
    return fs.readFileSync(file, "utf8");
  } catch (error) {
    if (error.code === "ENOENT") {
      return undefined;
    }
    throw error;
  }
}

function stripFrontmatter(text) {
  if (!text.startsWith("---\n")) {
    return text;
  }
  const end = text.indexOf("\n---\n", 4);
  return end === -1 ? text : text.slice(end + 5);
}

function finish(errors) {
  return { errors, valid: errors.length === 0 };
}

function gitOk(cwd, args) {
  return spawnSync("git", args, { cwd, encoding: "utf8" }).status === 0;
}

function gitShow(cwd, rev, file) {
  const result = spawnSync("git", ["show", `${rev}:${file}`], { cwd, encoding: "utf8" });
  return result.status === 0 ? result.stdout : "";
}

function gitRawDiff(cwd, baseSha, headSha) {
  const result = spawnSync("git", ["diff", "--raw", "--no-renames", "-z", baseSha, headSha], { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    return { error: `git diff: fetch or restore the checked revisions so the matrix closure range can be validated: ${result.stderr.trim()}` };
  }
  const parts = result.stdout.split("\0").filter(Boolean);
  const operations = [];
  for (let i = 0; i < parts.length;) {
    const part = parts[i];
    let header;
    let file;
    if (part.includes("\t")) {
      [header, file] = part.split("\t");
      i += 1;
    } else {
      header = part;
      file = parts[i + 1];
      i += 2;
    }
    const match = /^:(\d{6}) (\d{6}) [0-9a-f]+ [0-9a-f]+ ([A-Z])/.exec(header ?? "");
    if (!match || file === undefined) {
      return { error: "git diff: parse the raw no-renames tree delta; malformed output cannot prove the closure allowlist." };
    }
    operations.push({ oldMode: match[1], newMode: match[2], status: match[3], path: file });
  }
  return { operations };
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}
