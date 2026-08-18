import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const sourcePath = "docs/proposals/agent-language-services.md";
const decisionPath = "docs/reference/agent-language-services-lifecycle-review/source-decisions.json";
const provenancePath = "docs/reference/agent-language-services-lifecycle/provenance.json";
const validClasses = new Set(["conformance", "supporting"]);
const validLifecycles = new Set(["current", "completed", "planned", "removed"]);

if (isMainModule()) {
  const repoRoot = process.cwd();
  const command = process.argv[2] ?? "validate";
  const result = runCommand({ repoRoot, command, outputPath: process.argv[3] });
  if (!result.valid) {
    console.error(result.summary);
    for (const error of result.errors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }
  console.log(result.summary);
}

export function runCommand({ repoRoot, command, outputPath }) {
  if (command === "validate") {
    const errors = validateRepository({ repoRoot });
    return errors.length === 0
      ? success("Agent language-services lifecycle review artifacts match the umbrella proposal.")
      : failure("Repair the agent language-services lifecycle review artifacts before selecting the frozen inventory target.", errors);
  }
  if (command === "print-source-decisions") {
    const artifact = buildSourceDecisionArtifact({ repoRoot });
    const text = `${JSON.stringify(artifact, null, 2)}\n`;
    if (outputPath === undefined) {
      process.stdout.write(text);
    } else {
      fs.mkdirSync(path.dirname(path.resolve(repoRoot, outputPath)), { recursive: true });
      fs.writeFileSync(path.resolve(repoRoot, outputPath), text);
    }
    return success("Wrote reviewed source-decision artifact.");
  }
  return failure("Use `validate` or `print-source-decisions [path]`.", [`unknown command: ${command}`]);
}

export function validateRepository({ repoRoot, artifact = readJson(path.resolve(repoRoot, decisionPath)) }) {
  const parsed = parseUmbrellaProposal({ repoRoot });
  const errors = [
    ...validateArtifactShape({ artifact }),
    ...validateProvenance({ repoRoot }),
  ];
  if (errors.length !== 0) {
    return errors;
  }
  errors.push(...validateArtifactAgainstSource({ artifact, parsed }));
  errors.push(...validateSemanticDecisions({ artifact }));
  return errors;
}

export function buildSourceDecisionArtifact({ repoRoot }) {
  const parsed = parseUmbrellaProposal({ repoRoot });
  return {
    schema_version: 1,
    source_path: sourcePath,
    source_digest: parsed.sourceDigest,
    roots: parsed.roots.map((root) => {
      const leaves = classifyRoot(root);
      return {
        id: root.id,
        kind: root.kind,
        source_heading: root.heading,
        start_scalar: root.start,
        end_scalar: root.end,
        digest: root.digest,
        text: root.text,
        source_class: root.sourceClass,
        leaf_count: leaves.length,
        leaves,
        identities: identityOccurrences(root, leaves),
      };
    }),
  };
}

export function parseUmbrellaProposal({ repoRoot, sourceText = fs.readFileSync(path.resolve(repoRoot, sourcePath), "utf8") }) {
  const body = stripFrontmatter(sourceText);
  const lineRecords = linesWithSpans(body.text, body.offset);
  const roots = [];
  let heading = "# Agent Language Services";
  let inAcceptanceModel = false;
  let index = 0;
  let rootIndex = 1;
  while (index < lineRecords.length) {
    const line = lineRecords[index];
    const trimmed = line.text.trim();
    if (trimmed === "") {
      index += 1;
      continue;
    }
    if (/^#{1,6}\s+/.test(line.text)) {
      heading = trimmed;
      if (heading === "## Acceptance Model") {
        inAcceptanceModel = true;
      } else if (/^##\s+/.test(heading)) {
        inAcceptanceModel = false;
      }
      index += 1;
      continue;
    }
    if (/^```/.test(trimmed)) {
      index += 1;
      while (index < lineRecords.length && !/^```/.test(lineRecords[index].text.trim())) {
        if (lineRecords[index].text.trim() !== "") {
          roots.push(sourceRoot({ rootIndex: rootIndex++, kind: "fenced-line", heading, inAcceptanceModel, lines: [lineRecords[index]] }));
        }
        index += 1;
      }
      index += index < lineRecords.length ? 1 : 0;
      continue;
    }
    if (isTableRow(lineRecords, index)) {
      roots.push(sourceRoot({ rootIndex: rootIndex++, kind: "table-row", heading, inAcceptanceModel, lines: [line] }));
      index += 1;
      continue;
    }
    if (isListItemStart(line.text)) {
      const start = index;
      index += 1;
      while (index < lineRecords.length && belongsToListItem(lineRecords[index], lineRecords, index)) {
        index += 1;
      }
      roots.push(sourceRoot({ rootIndex: rootIndex++, kind: "list-item", heading, inAcceptanceModel, lines: lineRecords.slice(start, index) }));
      continue;
    }
    const start = index;
    index += 1;
    while (
      index < lineRecords.length
      && lineRecords[index].text.trim() !== ""
      && !/^#{1,6}\s+/.test(lineRecords[index].text)
      && !/^```/.test(lineRecords[index].text.trim())
      && !isTableRow(lineRecords, index)
      && !isListItemStart(lineRecords[index].text)
    ) {
      index += 1;
    }
    roots.push(sourceRoot({ rootIndex: rootIndex++, kind: "paragraph", heading, inAcceptanceModel, lines: lineRecords.slice(start, index) }));
  }
  return {
    sourceDigest: digest(sourceText),
    roots,
  };
}

export function validateDiffScope({ changedPaths, hasFrozenArtifact, isBootstrap }) {
  const allowedBootstrap = [
    ".github/workflows/workflow--test-scripts.yaml",
    "docs/reference/README.md",
    "workflow-scripts/check-agent-language-services-lifecycle.mjs",
    "workflow-scripts/check-agent-language-services-lifecycle.test.mjs",
  ];
  const allowedPrefix = "docs/reference/agent-language-services-lifecycle/";
  const errors = [];
  if (isBootstrap && !hasFrozenArtifact) {
    errors.push("diff scope: bootstrap validation requires the frozen lifecycle artifact addition");
  }
  for (const changedPath of changedPaths) {
    if (isBootstrap && (allowedBootstrap.includes(changedPath) || changedPath.startsWith(allowedPrefix))) {
      continue;
    }
    if (isBootstrap) {
      errors.push(`${changedPath}: remove this path from the frozen-inventory bootstrap PR; it is outside the reviewed allowlist`);
    }
  }
  return errors;
}

function validateArtifactShape({ artifact }) {
  const errors = [];
  if (artifact?.schema_version !== 1) {
    errors.push("source-decisions.json: set schema_version to 1");
  }
  if (artifact?.source_path !== sourcePath) {
    errors.push(`source-decisions.json: set source_path to ${sourcePath}`);
  }
  if (!Array.isArray(artifact?.roots)) {
    errors.push("source-decisions.json: add a roots array");
  }
  return errors;
}

function validateProvenance({ repoRoot }) {
  const errors = [];
  const fullPath = path.resolve(repoRoot, provenancePath);
  if (!fs.existsSync(fullPath)) {
    return [`${provenancePath}: add tracked target provenance for the later frozen-inventory bootstrap`];
  }
  const provenance = readJson(fullPath);
  for (const field of ["schema_version", "target_kind", "proposal_path", "proposal_anchor", "base_commit", "default_branch", "prerequisites"]) {
    if (!(field in provenance)) {
      errors.push(`${provenancePath}: add ${field}`);
    }
  }
  if (provenance.schema_version !== 1) {
    errors.push(`${provenancePath}: set schema_version to 1`);
  }
  if (provenance.target_kind !== "proposal-section") {
    errors.push(`${provenancePath}: set target_kind to proposal-section for the frozen source inventory`);
  }
  if (provenance.proposal_path !== "docs/proposals/agent-language-services-lifecycle-migration.md") {
    errors.push(`${provenancePath}: bind provenance to the lifecycle migration proposal`);
  }
  if (provenance.proposal_anchor !== "#frozen-source-universe") {
    errors.push(`${provenancePath}: bind provenance to #frozen-source-universe`);
  }
  if (!/^[0-9a-f]{40}$/.test(provenance.base_commit ?? "")) {
    errors.push(`${provenancePath}: use a full 40-character base commit`);
  }
  if (!Array.isArray(provenance.prerequisites)) {
    errors.push(`${provenancePath}: keep prerequisites as an array`);
  }
  return errors;
}

function validateArtifactAgainstSource({ artifact, parsed }) {
  const errors = [];
  if (artifact.source_digest !== parsed.sourceDigest) {
    errors.push(`${sourcePath}: restore the reviewed umbrella proposal text or regenerate reviewed decisions from the accepted source`);
  }
  const rootsById = new Map(parsed.roots.map((root) => [root.id, root]));
  const seenRoots = new Set();
  for (const [index, root] of artifact.roots.entries()) {
    const label = `source-decisions.json roots[${index}] ${root?.id ?? ""}`.trim();
    if (seenRoots.has(root?.id)) {
      errors.push(`${label}: remove duplicate source root`);
    }
    seenRoots.add(root?.id);
    const parsedRoot = rootsById.get(root?.id);
    if (parsedRoot === undefined) {
      errors.push(`${label}: remove unknown source root`);
      continue;
    }
    if (root.digest !== parsedRoot.digest || root.text !== parsedRoot.text) {
      errors.push(`${root.id}: restore the exact reviewed source text and digest`);
    }
    if (root.source_heading !== parsedRoot.heading || root.kind !== parsedRoot.kind) {
      errors.push(`${root.id}: restore source heading and root kind from the structural parser`);
    }
    if (!validClasses.has(root.source_class)) {
      errors.push(`${root.id}: set source_class to conformance or supporting`);
    }
    errors.push(...validateLeaves({ root, parsedRoot }));
    errors.push(...validateIdentities({ root }));
  }
  for (const parsedRoot of parsed.roots) {
    if (!seenRoots.has(parsedRoot.id)) {
      errors.push(`${parsedRoot.id}: add missing source root to reviewed decisions`);
    }
  }
  return errors;
}

function validateLeaves({ root, parsedRoot }) {
  const errors = [];
  if (!Array.isArray(root.leaves)) {
    return [`${root.id}: add semantic leaves`];
  }
  if (root.leaf_count !== root.leaves.length) {
    errors.push(`${root.id}: set leaf_count to the exact number of leaves`);
  }
  const expectedIds = new Set(Array.from({ length: root.leaf_count }, (_, index) => `${root.id}-L${String(index + 1).padStart(2, "0")}`));
  const seenLeaves = new Set();
  const covered = new Set();
  for (const leaf of root.leaves) {
    if (!expectedIds.has(leaf?.id)) {
      errors.push(`${root.id}: use contiguous child leaf IDs`);
    }
    if (seenLeaves.has(leaf?.id)) {
      errors.push(`${root.id}: remove duplicate child leaf ${leaf.id}`);
    }
    seenLeaves.add(leaf?.id);
    if (!validLifecycles.has(leaf?.lifecycle)) {
      errors.push(`${leaf?.id ?? root.id}: set lifecycle to current, completed, planned, or removed`);
    }
    if (root.source_class === "conformance" && leaf?.lifecycle === "removed") {
      errors.push(`${leaf.id}: conformance leaves may not use removed lifecycle`);
    }
    if (!Array.isArray(leaf?.spans) || leaf.spans.length === 0) {
      errors.push(`${leaf?.id ?? root.id}: add at least one source span`);
      continue;
    }
    for (const span of leaf.spans) {
      if (!Number.isInteger(span?.start_scalar) || !Number.isInteger(span?.end_scalar)) {
        errors.push(`${leaf.id}: span bounds must be integer Unicode-scalar offsets`);
        continue;
      }
      if (span.start_scalar < 0 || span.end_scalar > parsedRoot.scalarLength || span.start_scalar >= span.end_scalar) {
        errors.push(`${leaf.id}: span is outside the source root`);
        continue;
      }
      for (let scalar = span.start_scalar; scalar < span.end_scalar; scalar += 1) {
        if (covered.has(scalar)) {
          errors.push(`${leaf.id}: child spans overlap at scalar ${scalar}`);
          break;
        }
        covered.add(scalar);
      }
    }
    const joined = leaf.spans.map((span) => scalarSlice(parsedRoot.text, span.start_scalar, span.end_scalar)).join("");
    if (leaf.text !== joined) {
      errors.push(`${leaf.id}: restore leaf text from its source spans`);
    }
    if (leaf.digest !== digest(joined)) {
      errors.push(`${leaf.id}: restore leaf digest from its source spans`);
    }
  }
  for (const expectedId of expectedIds) {
    if (!seenLeaves.has(expectedId)) {
      errors.push(`${root.id}: add missing child leaf ${expectedId}`);
    }
  }
  for (const scalar of meaningfulScalars(parsedRoot.text)) {
    if (!covered.has(scalar)) {
      errors.push(`${root.id}: cover source scalar ${scalar} with exactly one child leaf`);
      break;
    }
  }
  return errors;
}

function validateIdentities({ root }) {
  const errors = [];
  if (!Array.isArray(root.identities)) {
    return [`${root.id}: add identities array, even when empty`];
  }
  const leafIds = new Set((root.leaves ?? []).map((leaf) => leaf.id));
  const seen = new Set();
  for (const identity of root.identities) {
    const key = `${identity.kind}:${identity.name}:${identity.leaf_id}`;
    if (seen.has(key)) {
      errors.push(`${root.id}: remove duplicate identity ${identity.kind}:${identity.name}`);
    }
    seen.add(key);
    if (!leafIds.has(identity.leaf_id)) {
      errors.push(`${root.id}: bind identity ${identity.kind}:${identity.name} to an existing leaf`);
    }
    if (!identityKinds().has(identity.kind)) {
      errors.push(`${root.id}: use a declared finite identity kind for ${identity.name}`);
    }
  }
  return errors;
}

function validateSemanticDecisions({ artifact }) {
  const errors = [];
  const identities = new Map();
  for (const root of artifact.roots) {
    for (const identity of root.identities ?? []) {
      const key = `${identity.kind}:${identity.name}`;
      if (!identities.has(key)) {
        identities.set(key, []);
      }
      identities.get(key).push(identity);
    }
  }
  for (const question of Array.from({ length: 22 }, (_, index) => `Q${String(index + 1).padStart(2, "0")}`)) {
    if (!identities.has(`evidence-gate:${question}`)) {
      errors.push(`source-decisions.json: bind evidence-gate identity ${question} to its source leaf`);
    }
  }
  for (const kind of identityKinds()) {
    if (![...identities.keys()].some((key) => key.startsWith(`${kind}:`))) {
      errors.push(`source-decisions.json: add at least one source-bound ${kind} identity`);
    }
  }
  return errors;
}

function classifyRoot(root) {
  const lifecycle = lifecycleFor(root);
  const spans = contiguousMeaningfulSpans(root.text);
  return [{
    id: `${root.id}-L01`,
    lifecycle,
    spans: spans.map(([start, end]) => ({ start_scalar: start, end_scalar: end })),
    text: spans.map(([start, end]) => scalarSlice(root.text, start, end)).join(""),
    digest: digest(spans.map(([start, end]) => scalarSlice(root.text, start, end)).join("")),
  }];
}

function lifecycleFor(root) {
  const text = root.text.toLowerCase();
  if (root.sourceClass === "supporting") {
    return "completed";
  }
  if (/\b(current|implemented|exposes|completed|closed|passes)\b/.test(text)) {
    return "current";
  }
  return "planned";
}

function identityOccurrences(root, leaves) {
  const leafId = leaves[0]?.id;
  const identities = [];
  for (const match of root.text.matchAll(/\bQ([0-1][0-9]|2[0-2])\b/g)) {
    identities.push({ kind: "evidence-gate", name: match[0], leaf_id: leafId });
  }
  const tableCells = root.kind === "table-row" ? root.text.split("|").map((cell) => cell.trim()).filter(Boolean) : [];
  if (root.heading.includes("Saved Workspace Function Reference")) {
    addTableIdentity(identities, "saved-reference-row", tableCells[0], leafId);
  }
  if (root.heading.includes("Definition And Reference Coverage")) {
    addTableIdentity(identities, "navigation-matrix-row", tableCells[0] ?? listItemName(root.text), leafId);
  }
  if (root.heading.includes("Topic Catalog")) {
    addTableIdentity(identities, "published-topic-matrix-row", tableCells[0] ?? listItemName(root.text), leafId);
  }
  if (root.inAcceptanceModel) {
    addTableIdentity(identities, "unresolved-acceptance-row", tableCells[0], leafId);
  }
  if (root.heading === "### Tools") {
    addTableIdentity(identities, "tool-kind", tableCells[0], leafId);
  }
  if (root.heading === "### Resources") {
    addTableIdentity(identities, "resource-kind", tableCells[0] ?? listItemName(root.text), leafId);
  }
  for (const name of matchesAfterColon(root.text, /tool kinds?:/i)) {
    identities.push({ kind: "tool-kind", name, leaf_id: leafId });
  }
  for (const name of matchesAfterColon(root.text, /resource kinds?:/i)) {
    identities.push({ kind: "resource-kind", name, leaf_id: leafId });
  }
  if (/package-document/i.test(root.text)) {
    for (const name of codeNames(root.text)) {
      identities.push({ kind: "package-document-declaration-kind", name, leaf_id: leafId });
    }
  }
  if (/\bLSP\b|language server protocol/i.test(root.text)) {
    for (const name of codeNames(root.text)) {
      identities.push({ kind: "lsp-encoding", name, leaf_id: leafId });
    }
  }
  if (/plugin/i.test(root.text) && root.kind === "table-row") {
    for (const cell of tableCells) {
      identities.push({ kind: "plugin-compatibility-cell", name: cell, leaf_id: leafId });
    }
  }
  const seen = new Set();
  return identities.filter((identity) => {
    if (!identity.name || /^[-]+$/.test(identity.name)) {
      return false;
    }
    const key = `${identity.kind}:${identity.name}:${identity.leaf_id}`;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

function addTableIdentity(identities, kind, name, leafId) {
  if (name && name !== "---" && !/^[-]+$/.test(name)) {
    identities.push({ kind, name, leaf_id: leafId });
  }
}

function listItemName(text) {
  return text.replace(/^\s*(?:[-*+]|\d+[.)])\s+/, "").trim().replace(/[.;]$/, "");
}

function identityKinds() {
  return new Set([
    "evidence-gate",
    "saved-reference-row",
    "navigation-matrix-row",
    "published-topic-matrix-row",
    "unresolved-acceptance-row",
    "tool-kind",
    "resource-kind",
    "package-document-declaration-kind",
    "lsp-encoding",
    "plugin-compatibility-cell",
  ]);
}

function sourceRoot({ rootIndex, kind, heading, inAcceptanceModel, lines }) {
  const start = lines[0].start;
  const end = lines[lines.length - 1].end;
  const text = lines.map((line) => line.text).join("\n");
  return {
    id: `ALS-R${String(rootIndex).padStart(4, "0")}`,
    kind,
    heading,
    start,
    end,
    text,
    digest: digest(text),
    scalarLength: [...text].length,
    inAcceptanceModel,
    sourceClass: sourceClassFor({ kind, heading, text }),
  };
}

function sourceClassFor({ kind, heading, text }) {
  if (kind === "fenced-line" || /^#/.test(text.trim())) {
    return "supporting";
  }
  if (/^(## Summary|## Problem|## Non-Goals|## Out of Scope)/.test(heading)) {
    return "supporting";
  }
  return "conformance";
}

function belongsToListItem(line, lines, index) {
  if (line.text.trim() === "") {
    return index + 1 < lines.length && !isListItemStart(lines[index + 1].text) && lines[index + 1].text.trim() !== "";
  }
  return !isListItemStart(line.text) && !/^#{1,6}\s+/.test(line.text) && !isTableRow(lines, index);
}

function isListItemStart(text) {
  return /^\s*(?:[-*+]|\d+[.)])\s+/.test(text);
}

function isTableRow(lines, index) {
  const text = lines[index].text.trim();
  if (!text.includes("|") || /^[-|:\s]+$/.test(text)) {
    return false;
  }
  return text.startsWith("|") || text.endsWith("|") || (index + 1 < lines.length && /^[-|:\s]+$/.test(lines[index + 1].text.trim()));
}

function linesWithSpans(text, offset) {
  const records = [];
  let position = offset;
  for (const raw of text.split("\n")) {
    records.push({ text: raw, start: position, end: position + [...raw].length });
    position += [...raw].length + 1;
  }
  return records;
}

function stripFrontmatter(text) {
  if (!text.startsWith("---\n")) {
    return { text, offset: 0 };
  }
  const end = text.indexOf("\n---\n", 4);
  if (end === -1) {
    return { text, offset: 0 };
  }
  const start = end + "\n---\n".length;
  return { text: text.slice(start), offset: [...text.slice(0, start)].length };
}

function contiguousMeaningfulSpans(text) {
  const spans = [];
  let start;
  for (let index = 0; index < [...text].length; index += 1) {
    if (isMeaningfulScalar(text, index)) {
      if (start === undefined) {
        start = index;
      }
    } else if (start !== undefined) {
      spans.push([start, index]);
      start = undefined;
    }
  }
  if (start !== undefined) {
    spans.push([start, [...text].length]);
  }
  return spans;
}

function meaningfulScalars(text) {
  const scalars = [];
  for (let index = 0; index < [...text].length; index += 1) {
    if (isMeaningfulScalar(text, index)) {
      scalars.push(index);
    }
  }
  return scalars;
}

function isMeaningfulScalar(text, index) {
  const scalar = [...text][index];
  return !/\s/.test(scalar) && !["|", "`"].includes(scalar);
}

function scalarSlice(text, start, end) {
  return [...text].slice(start, end).join("");
}

function matchesAfterColon(text, labelRegex) {
  const match = text.match(labelRegex);
  if (!match) {
    return [];
  }
  const rest = text.slice((match.index ?? 0) + match[0].length);
  return codeNames(rest);
}

function codeNames(text) {
  return [...text.matchAll(/`([^`]+)`/g)].map((match) => match[1]);
}

function digest(text) {
  return `sha256:${crypto.createHash("sha256").update(text, "utf8").digest("hex")}`;
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function success(summary) {
  return { valid: true, summary, errors: [] };
}

function failure(summary, errors) {
  return { valid: false, summary, errors };
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}
