import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const sourcePath = "docs/proposals/agent-language-services.md";
const artifactDir = "docs/reference/agent-language-services-lifecycle";
const contractPath = `${artifactDir}/source-universe.json`;
const inventoryPath = `${artifactDir}/frozen-inventory.json`;
const manifestPath = `${artifactDir}/lifecycle-manifest.json`;
const ledgerSchemaPath = `${artifactDir}/migration-ledger.schema.json`;
const frozenArtifactPaths = new Set([
  contractPath,
  inventoryPath,
  manifestPath,
  ledgerSchemaPath,
]);
const protectedBootstrapPaths = [
  sourcePath,
  "crates/veln-mcp/",
  "crates/veln-cli/tests/toolchain_cases/mcp/",
  "crates/veln-cli/tests/toolchain_harness/",
  "crates/veln-cli/tests/toolchain_semantic_baseline/",
  "examples/specification/mcp/",
];
const validLifecycles = new Set(["current", "completed", "planned", "removed"]);

if (isMainModule()) {
  const repoRoot = process.cwd();
  const command = process.argv[2] ?? "validate";
  const result = runCommand({ command, repoRoot });
  if (!result.valid) {
    const message = [
      result.summary,
      ...result.errors.map((error) => `- ${error}`),
    ].join("\n");
    if (process.env.GITHUB_ACTIONS === "true") {
      console.error(renderGitHubErrorAnnotation(message));
    }
    console.error(message);
    process.exit(1);
  }
  console.log(result.summary);
}

export function runCommand({ command, repoRoot }) {
  if (command === "write-artifacts") {
    return writeArtifacts({ repoRoot });
  }
  if (command !== "validate") {
    return failure("Use `validate` or `write-artifacts`.", [`unknown command: ${command}`]);
  }

  const validation = validateArtifacts({ repoRoot });
  const diffValidation = validateCiDiffScopeFromEnv({ repoRoot });
  return combineResults([
    validation,
    diffValidation,
  ], "Agent language services lifecycle artifacts are valid.");
}

export function writeArtifacts({ repoRoot }) {
  const source = fs.readFileSync(path.resolve(repoRoot, sourcePath), "utf8");
  const roots = parseMarkdownSource(source);
  const contract = {
    schema_version: 1,
    source: sourcePath,
    roots: roots.map((root) => ({
      id: root.id,
      heading: root.heading,
      kind: root.kind,
      conformance: true,
      digest: root.digest,
      identities: identitiesForRoot(root),
    })),
  };
  const inventory = {
    schema_version: 1,
    source: sourcePath,
    roots: roots.map((root) => inventoryRoot(root)),
  };
  const manifest = {
    schema_version: 1,
    source: sourcePath,
    leaves: inventory.roots.flatMap((root) => inventoryLeaves(root).map((leaf) => ({
      id: leaf.id,
      lifecycle: leaf.lifecycle,
      destination: destinationForLifecycle(leaf.lifecycle),
    }))),
  };
  const ledgerSchema = migrationLedgerSchema();

  fs.mkdirSync(path.resolve(repoRoot, artifactDir), { recursive: true });
  writeJson(path.resolve(repoRoot, contractPath), contract);
  writeJson(path.resolve(repoRoot, inventoryPath), inventory);
  writeJson(path.resolve(repoRoot, manifestPath), manifest);
  writeJson(path.resolve(repoRoot, ledgerSchemaPath), ledgerSchema);

  return success(`Wrote frozen lifecycle artifacts for ${roots.length} source item(s).`);
}

export function validateArtifacts({ repoRoot, artifacts } = {}) {
  const source = artifacts?.sourceText ?? fs.readFileSync(path.resolve(repoRoot, sourcePath), "utf8");
  const parsedRoots = parseMarkdownSource(source);
  const parsedById = new Map(parsedRoots.map((root) => [root.id, root]));
  const contract = artifacts?.contract ?? readJson(path.resolve(repoRoot, contractPath));
  const inventory = artifacts?.inventory ?? readJson(path.resolve(repoRoot, inventoryPath));
  const manifest = artifacts?.manifest ?? readJson(path.resolve(repoRoot, manifestPath));
  const ledgerSchema = artifacts?.ledgerSchema ?? readJson(path.resolve(repoRoot, ledgerSchemaPath));
  const errors = [
    ...validateContract({ contract, parsedRoots, parsedById }),
    ...validateInventory({ inventory, contract, parsedById }),
    ...validateManifest({ manifest, inventory }),
    ...validateLedgerSchema(ledgerSchema),
  ];
  return errors.length === 0
    ? success(`Validated ${parsedRoots.length} frozen source item(s).`)
    : failure("Update the frozen agent-language-services lifecycle artifacts before merging; the inventory must match the unchanged umbrella proposal and reviewed lifecycle manifest.", errors);
}

export function validateLedger({ ledger, inventory, manifest }) {
  const errors = [];
  const leaves = allInventoryLeaves(inventory);
  const leafById = new Map(leaves.map((leaf) => [leaf.id, leaf]));
  const parentIds = new Set(inventory.roots.filter((root) => root.child_count > 0).map((root) => root.id));
  const manifestById = new Map(manifest.leaves.map((leaf) => [leaf.id, leaf]));
  if (!Array.isArray(ledger.mappings)) {
    return ["ledger: add a mappings array so every frozen inventory leaf has one reviewed destination"];
  }

  const seen = new Set();
  for (const [index, mapping] of ledger.mappings.entries()) {
    const label = `ledger.mappings[${index}]`;
    if (!mapping || typeof mapping !== "object") {
      errors.push(`${label}: use an object mapping with source_id, lifecycle, and destination`);
      continue;
    }
    if (containsCatchAll(mapping.source_id)) {
      errors.push(`${label}: replace range, wildcard, or catch-all source_id "${mapping.source_id}" with one exact inventory leaf ID`);
      continue;
    }
    if (parentIds.has(mapping.source_id)) {
      errors.push(`${label}: map each child leaf of ${mapping.source_id}; parent records that declare children cannot be mapped directly`);
      continue;
    }
    const leaf = leafById.get(mapping.source_id);
    if (!leaf) {
      errors.push(`${label}: replace unknown source_id "${mapping.source_id}" with an inventory leaf ID`);
      continue;
    }
    if (seen.has(mapping.source_id)) {
      errors.push(`${label}: remove the duplicate mapping for ${mapping.source_id}; each inventory leaf maps exactly once`);
      continue;
    }
    seen.add(mapping.source_id);
    if (!validLifecycles.has(mapping.lifecycle)) {
      errors.push(`${label}: replace lifecycle "${mapping.lifecycle}" with current, completed, planned, or removed`);
      continue;
    }
    const manifestLeaf = manifestById.get(mapping.source_id);
    if (manifestLeaf?.lifecycle !== mapping.lifecycle) {
      errors.push(`${label}: use lifecycle "${manifestLeaf?.lifecycle}" from the reviewed lifecycle manifest for ${mapping.source_id}`);
    }
    if (leaf.conformance && mapping.lifecycle === "removed") {
      errors.push(`${label}: map conformance leaf ${mapping.source_id} to current, completed, or planned; removed is only for supporting explanation`);
    }
    errors.push(...validateDestination({ label, lifecycle: mapping.lifecycle, destination: mapping.destination }));
  }

  for (const leaf of leaves) {
    if (!seen.has(leaf.id)) {
      errors.push(`ledger: add one mapping for inventory leaf ${leaf.id}`);
    }
  }
  return errors;
}

export function validateDiffScope({ changedPaths, baseHasFrozen, headHasFrozen, prerequisitesComplete = true }) {
  const errors = [];
  const addsFrozen = !baseHasFrozen && headHasFrozen;
  if (addsFrozen && !prerequisitesComplete) {
    return [
      "diff-scope: finish the checked prerequisite records before adding frozen lifecycle artifacts; the bootstrap inventory must start from the closed prerequisite universe",
    ];
  }
  if (!addsFrozen) {
    return errors;
  }

  for (const changedPath of changedPaths) {
    if (isProtectedBootstrapPath(changedPath)) {
      errors.push(`${changedPath}: restore this path or move the change to a later PR; changing it would mix toolchain behavior or reorganize the frozen proposal during the documentation-only inventory bootstrap`);
    }
  }
  return errors;
}

export function parseMarkdownSource(text) {
  const body = stripFrontmatter(text);
  const lines = body.text.split("\n");
  const roots = [];
  let heading = "";
  let paragraph = [];
  let inFence = false;
  let fenceStart = "";
  let sequence = 1;

  const flushParagraph = () => {
    if (paragraph.length === 0) {
      return;
    }
    const text = paragraph.map((line) => line.text).join("\n");
    roots.push(sourceRoot({
      sequence: sequence++,
      heading,
      kind: "paragraph",
      text,
    }));
    paragraph = [];
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("```")) {
      flushParagraph();
      if (!inFence) {
        inFence = true;
        fenceStart = trimmed;
      } else {
        inFence = false;
        fenceStart = "";
      }
      continue;
    }
    if (inFence) {
      if (trimmed.length > 0) {
        roots.push(sourceRoot({
          sequence: sequence++,
          heading,
          kind: "fenced_line",
          text: line,
        }));
      }
      continue;
    }
    const headingMatch = /^(#{1,6})\s+(.+?)\s*$/.exec(line);
    if (headingMatch) {
      flushParagraph();
      heading = headingMatch[2];
      continue;
    }
    if (trimmed.length === 0) {
      flushParagraph();
      continue;
    }
    if (isTableRow(line)) {
      flushParagraph();
      roots.push(sourceRoot({
        sequence: sequence++,
        heading,
        kind: isTableDelimiterRow(line) ? "table_delimiter" : "table_row",
        text: line,
      }));
      continue;
    }
    if (/^\s*[-*]\s+/.test(line) || /^\s*\d+\.\s+/.test(line)) {
      flushParagraph();
      roots.push(sourceRoot({
        sequence: sequence++,
        heading,
        kind: "list_item",
        text: line,
      }));
      continue;
    }
    paragraph.push({ text: line });
  }
  flushParagraph();
  return roots;
}

function validateContract({ contract, parsedRoots, parsedById }) {
  const errors = [];
  if (contract.source !== sourcePath) {
    errors.push(`source-universe: set source to ${sourcePath}`);
  }
  const contractRoots = Array.isArray(contract.roots) ? contract.roots : [];
  const seen = new Set();
  for (const [index, record] of contractRoots.entries()) {
    const label = `source-universe.roots[${index}]`;
    if (seen.has(record.id)) {
      errors.push(`${label}: remove duplicate source ID ${record.id}`);
      continue;
    }
    seen.add(record.id);
    const parsed = parsedById.get(record.id);
    if (!parsed) {
      errors.push(`${label}: remove unknown source ID ${record.id} or regenerate the reviewed contract from the current proposal source`);
      continue;
    }
    if (record.digest !== parsed.digest) {
      errors.push(`${record.id}: update the digest or restore the proposal text; the frozen source text changed`);
    }
    if (record.heading !== parsed.heading) {
      errors.push(`${record.id}: update heading "${record.heading}" to "${parsed.heading}"`);
    }
    if (record.kind !== parsed.kind) {
      errors.push(`${record.id}: update kind "${record.kind}" to "${parsed.kind}"`);
    }
  }
  for (const parsed of parsedRoots) {
    if (!seen.has(parsed.id)) {
      errors.push(`source-universe: add missing source item ${parsed.id}`);
    }
  }
  return errors;
}

function validateInventory({ inventory, contract, parsedById }) {
  const errors = [];
  const contractById = new Map(contract.roots.map((root) => [root.id, root]));
  const roots = Array.isArray(inventory.roots) ? inventory.roots : [];
  const seenRoots = new Set();
  const seenLeaves = new Set();

  for (const [index, root] of roots.entries()) {
    const label = `frozen-inventory.roots[${index}]`;
    if (seenRoots.has(root.id)) {
      errors.push(`${label}: remove duplicate inventory source ID ${root.id}`);
      continue;
    }
    seenRoots.add(root.id);
    const parsed = parsedById.get(root.id);
    const contractRoot = contractById.get(root.id);
    if (!parsed || !contractRoot) {
      errors.push(`${label}: replace unknown source ID ${root.id} with a source-universe ID`);
      continue;
    }
    if (root.digest !== parsed.digest) {
      errors.push(`${root.id}: update the inventory digest or restore the proposal text; the frozen source text changed`);
    }
    if (root.heading !== parsed.heading) {
      errors.push(`${root.id}: update inventory heading "${root.heading}" to "${parsed.heading}"`);
    }
    const children = Array.isArray(root.children) ? root.children : [];
    if ((root.child_count ?? 0) !== children.length) {
      errors.push(`${root.id}: set child_count to ${children.length} so parent structure is explicit`);
    }
    if (children.length === 0) {
      errors.push(...validateLeaf({ leaf: root, label: root.id, seenLeaves, expectedText: parsed.text, conformance: contractRoot.conformance }));
      continue;
    }
    errors.push(...validateChildren({ root, children, parsed, contractRoot, seenLeaves }));
  }

  for (const contractRoot of contract.roots) {
    if (!seenRoots.has(contractRoot.id)) {
      errors.push(`frozen-inventory: add missing inventory item ${contractRoot.id}`);
    }
  }
  return errors;
}

function validateChildren({ root, children, parsed, contractRoot, seenLeaves }) {
  const errors = [];
  const expectedIds = children.map((_, index) => `${root.id}.${String(index + 1).padStart(2, "0")}`);
  const covered = Array.from({ length: scalarLength(parsed.text) }, () => false);
  const childSpans = [];
  for (const [index, child] of children.entries()) {
    const expectedId = expectedIds[index];
    if (child.id !== expectedId) {
      errors.push(`${root.id}: use contiguous child ID ${expectedId} at child index ${index}`);
    }
    errors.push(...validateLeaf({ leaf: child, label: child.id, seenLeaves, expectedText: scalarSlice(parsed.text, child.span.start, child.span.end), conformance: contractRoot.conformance }));
    if (!validSpan(child.span, parsed.text)) {
      errors.push(`${child.id}: use a child span within 0..${scalarLength(parsed.text)}`);
      continue;
    }
    childSpans.push({ id: child.id, ...child.span });
    for (let cursor = child.span.start; cursor < child.span.end; cursor += 1) {
      if (covered[cursor]) {
        errors.push(`${child.id}: remove overlapping child span at scalar ${cursor}`);
      }
      covered[cursor] = true;
    }
  }

  const separatorSpans = Array.isArray(root.separator_spans) ? root.separator_spans : [];
  for (const [index, span] of separatorSpans.entries()) {
    const label = `${root.id}.separator_spans[${index}]`;
    if (!validSpan(span, parsed.text)) {
      errors.push(`${label}: keep separator span within 0..${scalarLength(parsed.text)}`);
      continue;
    }
    const text = scalarSlice(parsed.text, span.start, span.end);
    if (!/^[\s|:-]*$/.test(text)) {
      errors.push(`${label}: keep only whitespace or table punctuation in separator spans`);
    }
    for (let cursor = span.start; cursor < span.end; cursor += 1) {
      if (covered[cursor]) {
        errors.push(`${label}: remove separator overlap at scalar ${cursor}`);
      }
      covered[cursor] = true;
    }
  }

  for (const [index, value] of covered.entries()) {
    if (!value && /\S/.test(scalarSlice(parsed.text, index, index + 1))) {
      errors.push(`${root.id}: cover non-whitespace source scalar ${index} with exactly one child or separator span`);
      break;
    }
  }
  for (const [index, statement] of lifecycleStatements(parsed.text).entries()) {
    const owners = childSpans.filter((span) => rangesOverlap(span, statement));
    if (owners.length === 0) {
      errors.push(`${root.id}: assign lifecycle statement ${index + 1} to a child span`);
    }
    if (owners.length > 1) {
      errors.push(`${root.id}: keep lifecycle statement ${index + 1} within one child span`);
    }
  }
  return errors;
}

function validateLeaf({ leaf, label, seenLeaves, expectedText, conformance }) {
  const errors = [];
  if (seenLeaves.has(leaf.id)) {
    errors.push(`${label}: remove duplicate inventory leaf ID ${leaf.id}`);
  }
  seenLeaves.add(leaf.id);
  if (!validLifecycles.has(leaf.lifecycle)) {
    errors.push(`${label}: replace lifecycle "${leaf.lifecycle}" with current, completed, planned, or removed`);
  }
  if (conformance && leaf.lifecycle === "removed") {
    errors.push(`${label}: use current, completed, or planned for conformance leaves; removed is only for supporting explanation`);
  }
  const digest = sha256(expectedText);
  if (leaf.digest !== digest) {
    errors.push(`${label}: update leaf digest to match its exact source text`);
  }
  return errors;
}

function validateManifest({ manifest, inventory }) {
  const errors = [];
  const leaves = allInventoryLeaves(inventory);
  const leafById = new Map(leaves.map((leaf) => [leaf.id, leaf]));
  const seen = new Set();
  if (!Array.isArray(manifest.leaves)) {
    return ["lifecycle-manifest: add a leaves array"];
  }
  for (const [index, leaf] of manifest.leaves.entries()) {
    const label = `lifecycle-manifest.leaves[${index}]`;
    if (seen.has(leaf.id)) {
      errors.push(`${label}: remove duplicate lifecycle entry ${leaf.id}`);
      continue;
    }
    seen.add(leaf.id);
    const inventoryLeaf = leafById.get(leaf.id);
    if (!inventoryLeaf) {
      errors.push(`${label}: remove unknown lifecycle leaf ${leaf.id}`);
      continue;
    }
    if (leaf.lifecycle !== inventoryLeaf.lifecycle) {
      errors.push(`${leaf.id}: align inventory lifecycle "${inventoryLeaf.lifecycle}" with reviewed manifest lifecycle "${leaf.lifecycle}"`);
    }
    errors.push(...validateDestination({ label, lifecycle: leaf.lifecycle, destination: leaf.destination }));
  }
  for (const leaf of leaves) {
    if (!seen.has(leaf.id)) {
      errors.push(`lifecycle-manifest: add lifecycle entry for inventory leaf ${leaf.id}`);
    }
  }
  return errors;
}

function validateLedgerSchema(schema) {
  const errors = [];
  if (schema?.$id !== "https://veln-lang.invalid/schemas/agent-language-services-migration-ledger.schema.json") {
    errors.push("migration-ledger.schema.json: keep the stable schema $id for the reviewed migration ledger");
  }
  const lifecycleEnum = schema?.properties?.mappings?.items?.properties?.lifecycle?.enum ?? [];
  for (const lifecycle of validLifecycles) {
    if (!lifecycleEnum.includes(lifecycle)) {
      errors.push(`migration-ledger.schema.json: include lifecycle enum value ${lifecycle}`);
    }
  }
  if (schema?.properties?.mappings?.items?.additionalProperties !== false) {
    errors.push("migration-ledger.schema.json: reject unknown mapping fields so reviewed destinations stay bounded");
  }
  return errors;
}

function validateCiDiffScopeFromEnv({ repoRoot }) {
  const baseSha = process.env.ALS_LIFECYCLE_BASE_SHA;
  const headSha = process.env.ALS_LIFECYCLE_HEAD_SHA;
  if (!baseSha || !headSha || /^0+$/.test(baseSha)) {
    return success("No agent language services diff-scope range was provided.");
  }
  const changedPaths = changedPathNames({ repoRoot, baseSha, headSha });
  const errors = validateDiffScope({
    changedPaths,
    baseHasFrozen: gitPathExists({ repoRoot, revision: baseSha, file: inventoryPath }),
    headHasFrozen: gitPathExists({ repoRoot, revision: headSha, file: inventoryPath }),
    prerequisitesComplete: prerequisitesAreComplete({ repoRoot, revision: baseSha }),
  });
  return errors.length === 0
    ? success(`Diff scope is valid for ${changedPaths.length} changed path(s).`)
    : failure("Restore the listed paths before merging; the first frozen-inventory PR may only add documentation lifecycle artifacts and their validator.", errors);
}

function inventoryRoot(root) {
  const cells = root.kind === "table_row" ? tableCellSpans(root.text) : [];
  if (cells.length > 0) {
    return {
      id: root.id,
      heading: root.heading,
      kind: root.kind,
      digest: root.digest,
      child_count: cells.length,
      children: cells.map((cell, index) => {
        const text = scalarSlice(root.text, cell.start, cell.end);
        const lifecycle = lifecycleForText(root.heading, text);
        return {
          id: `${root.id}.${String(index + 1).padStart(2, "0")}`,
          heading: root.heading,
          kind: "table_cell",
          span: cell,
          digest: sha256(text),
          lifecycle,
          conformance: true,
        };
      }),
      separator_spans: separatorSpansFor(root.text, cells),
    };
  }
  return {
    id: root.id,
    heading: root.heading,
    kind: root.kind,
    digest: root.digest,
    child_count: 0,
    lifecycle: lifecycleForText(root.heading, root.text),
    conformance: true,
  };
}

function allInventoryLeaves(inventory) {
  return inventory.roots.flatMap(inventoryLeaves);
}

function inventoryLeaves(root) {
  return Array.isArray(root.children) && root.children.length > 0 ? root.children : [root];
}

function sourceRoot({ sequence, heading, kind, text }) {
  return {
    id: `ALS-S${String(sequence).padStart(4, "0")}`,
    heading,
    kind,
    text,
    digest: sha256(text),
  };
}

function identitiesForRoot(root) {
  const identities = [];
  for (const match of root.text.matchAll(/\bQ(?:0[1-9]|1[0-9]|2[0-2])\b/g)) {
    identities.push({ kind: "evidence_gate", name: match[0] });
  }
  for (const token of ["workspace_projects", "refresh_workspace", "check_project", "definition", "references", "search_docs", "read_doc"]) {
    if (root.text.includes(token)) {
      identities.push({ kind: "tool_or_resource_kind", name: token });
    }
  }
  for (const token of ["UTF-8", "UTF-16", "UTF-32"]) {
    if (root.text.includes(token)) {
      identities.push({ kind: "lsp_encoding", name: token });
    }
  }
  return identities;
}

function lifecycleForText(heading, text) {
  const combined = `${heading}\n${text}`.toLowerCase();
  if (/\b(remain planned|planned|deferred|future|unresolved|next slice|will|must be selected)\b/.test(combined)) {
    return "planned";
  }
  if (/\b(implemented and specified|implemented|completed|current contract|currently exposes|existing)\b/.test(combined)) {
    return "current";
  }
  if (/\b(history|rationale|evidence route)\b/.test(combined)) {
    return "completed";
  }
  return "planned";
}

function destinationForLifecycle(lifecycle) {
  if (lifecycle === "current") {
    return {
      kind: "current",
      specification: "docs/specification/mcp.md",
      evidence: "crates/veln-mcp/src/server/tests.rs",
    };
  }
  if (lifecycle === "completed") {
    return {
      kind: "completed",
      record: "docs/reference/implemented-proposals/agent-language-services.md",
    };
  }
  if (lifecycle === "planned") {
    return {
      kind: "planned",
      proposal: "docs/proposals/agent-language-services.md",
    };
  }
  return {
    kind: "removed",
    reason: "duplicated supporting explanation",
    duplicate_destination: "docs/proposals/agent-language-services.md",
  };
}

function migrationLedgerSchema() {
  return {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: "https://veln-lang.invalid/schemas/agent-language-services-migration-ledger.schema.json",
    type: "object",
    additionalProperties: false,
    required: ["schema_version", "mappings"],
    properties: {
      schema_version: { const: 1 },
      mappings: {
        type: "array",
        minItems: 1,
        items: {
          type: "object",
          additionalProperties: false,
          required: ["source_id", "lifecycle", "destination"],
          properties: {
            source_id: {
              type: "string",
              pattern: "^ALS-S[0-9]{4}(?:\\.[0-9]{2})?$",
            },
            lifecycle: {
              type: "string",
              enum: [...validLifecycles],
            },
            destination: {
              type: "object",
            },
          },
        },
      },
    },
  };
}

function validateDestination({ label, lifecycle, destination }) {
  if (!destination || typeof destination !== "object" || destination.kind !== lifecycle) {
    return [`${label}: use a ${lifecycle} destination object whose kind matches the lifecycle`];
  }
  if (lifecycle === "current" && (!isPath(destination.specification, "docs/specification/") || !destination.evidence)) {
    return [`${label}: link current mappings to a specification path and checked evidence`];
  }
  if (lifecycle === "completed" && !isPath(destination.record, "docs/reference/")) {
    return [`${label}: link completed mappings to a supporting reference or implementation record`];
  }
  if (lifecycle === "planned" && !isPath(destination.proposal, "docs/proposals/")) {
    return [`${label}: link planned mappings to the active proposal that retains the acceptance condition`];
  }
  if (lifecycle === "removed" && (!destination.reason || !destination.duplicate_destination)) {
    return [`${label}: explain removed mappings with a reason and duplicate or superseding destination`];
  }
  return [];
}

function isPath(value, prefix) {
  return typeof value === "string" && value.startsWith(prefix);
}

function tableCellSpans(line) {
  const scalars = [...line];
  const pipeIndexes = [];
  for (const [index, scalar] of scalars.entries()) {
    if (scalar === "|") {
      pipeIndexes.push(index);
    }
  }
  if (pipeIndexes.length < 2) {
    return [];
  }
  const cells = [];
  for (let index = 0; index < pipeIndexes.length - 1; index += 1) {
    let start = pipeIndexes[index] + 1;
    let end = pipeIndexes[index + 1];
    while (start < end && /\s/.test(scalars[start])) {
      start += 1;
    }
    while (end > start && /\s/.test(scalars[end - 1])) {
      end -= 1;
    }
    if (start < end) {
      cells.push({ start, end });
    }
  }
  return cells;
}

function separatorSpansFor(text, childSpans) {
  const spans = [];
  let cursor = 0;
  for (const span of childSpans) {
    if (cursor < span.start) {
      spans.push({ start: cursor, end: span.start });
    }
    cursor = span.end;
  }
  if (cursor < scalarLength(text)) {
    spans.push({ start: cursor, end: scalarLength(text) });
  }
  return spans.filter((span) => span.start < span.end);
}

function lifecycleStatements(text) {
  const scalars = [...text];
  const spans = [];
  let start = 0;
  for (const [index, scalar] of scalars.entries()) {
    if (scalar === "." || scalar === ";" || scalar === "\n" || scalar === "|") {
      spans.push({ start, end: index + 1 });
      start = index + 1;
    }
  }
  if (start < scalars.length) {
    spans.push({ start, end: scalars.length });
  }
  return spans.filter((span) => /[^\s|:-]/.test(scalarSlice(text, span.start, span.end)));
}

function stripFrontmatter(text) {
  const lines = text.split("\n");
  if (lines[0]?.trim() !== "---") {
    return { text };
  }
  const closing = lines.findIndex((line, index) => index > 0 && line.trim() === "---");
  if (closing === -1) {
    return { text };
  }
  return { text: lines.slice(closing + 1).join("\n") };
}

function isTableRow(line) {
  return line.trim().startsWith("|") && line.trim().endsWith("|");
}

function isTableDelimiterRow(line) {
  return /^\s*\|(?:\s*:?-{3,}:?\s*\|)+\s*$/.test(line);
}

function validSpan(span, text) {
  return Number.isInteger(span?.start)
    && Number.isInteger(span?.end)
    && span.start >= 0
    && span.end >= span.start
    && span.end <= scalarLength(text);
}

function rangesOverlap(left, right) {
  return left.start < right.end && right.start < left.end;
}

function scalarLength(text) {
  return [...text].length;
}

function scalarSlice(text, start, end) {
  return [...text].slice(start, end).join("");
}

function sha256(text) {
  return crypto.createHash("sha256").update(text).digest("hex");
}

function containsCatchAll(sourceId) {
  return typeof sourceId !== "string" || sourceId === "*" || sourceId.includes("..") || sourceId.includes("*") || /\bALL\b/i.test(sourceId);
}

function isProtectedBootstrapPath(changedPath) {
  if (frozenArtifactPaths.has(changedPath) || changedPath === "workflow-scripts/check-agent-language-services-lifecycle.mjs" || changedPath === "workflow-scripts/check-agent-language-services-lifecycle.test.mjs" || changedPath === ".github/workflows/workflow--test-scripts.yaml" || changedPath.startsWith(`${artifactDir}/`)) {
    return false;
  }
  return protectedBootstrapPaths.some((protectedPath) => (
    protectedPath.endsWith("/") ? changedPath.startsWith(protectedPath) : changedPath === protectedPath
  ));
}

function changedPathNames({ repoRoot, baseSha, headSha }) {
  const result = spawnSync(
    "git",
    ["diff", "--name-status", "-z", baseSha, headSha],
    { cwd: repoRoot, encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(`Unable to inspect diff scope: ${result.stderr.trim()}`);
  }
  const fields = result.stdout.split("\0").filter(Boolean);
  const paths = [];
  for (let index = 0; index < fields.length; index += 1) {
    const status = fields[index];
    if (/^R|^C/.test(status)) {
      paths.push(fields[index + 1], fields[index + 2]);
      index += 2;
    } else {
      paths.push(fields[index + 1]);
      index += 1;
    }
  }
  return paths.filter(Boolean);
}

function gitPathExists({ repoRoot, revision, file }) {
  const result = spawnSync(
    "git",
    ["cat-file", "-e", `${revision}:${file}`],
    { cwd: repoRoot, encoding: "utf8" },
  );
  return result.status === 0;
}

function prerequisitesAreComplete({ repoRoot, revision }) {
  const active = [
    "docs/proposals/agent-language-services-platform-matrix-closure.md",
    "docs/proposals/checked-proposal-target-readiness.md",
  ];
  return active.every((file) => !gitPathExists({ repoRoot, revision, file }));
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function writeJson(file, value) {
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

function renderGitHubErrorAnnotation(message) {
  return `::error title=Invalid agent language services lifecycle artifacts::${message.replaceAll("%", "%25").replaceAll("\n", "%0A").replaceAll("\r", "%0D")}`;
}

function combineResults(results, summary) {
  const errors = results.flatMap((result) => result.errors);
  return errors.length === 0 ? success(summary) : failure(results.find((result) => !result.valid)?.summary ?? summary, errors);
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
