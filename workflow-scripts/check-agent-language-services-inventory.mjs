import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const proposalPath = "docs/proposals/agent-language-services.md";
const universePath = "docs/reference/agent-language-services-source-universe.json";
const inventoryPath = "docs/reference/agent-language-services-frozen-inventory.json";
const manifestPath = "docs/reference/agent-language-services-lifecycle-manifest.json";
const schemaPath = "docs/reference/agent-language-services-migration-ledger.schema.json";
const artifactPaths = [universePath, inventoryPath, manifestPath, schemaPath];
const validatorPaths = [
  "workflow-scripts/check-agent-language-services-inventory.mjs",
  "workflow-scripts/check-agent-language-services-inventory.test.mjs",
];
const frozenPrefix = "agent-language-services/";
const lifecycleValues = new Set(["current", "completed", "planned", "removed"]);
const sourceUniverseContract = {
  root_count: 390,
  kind_counts: {
    paragraph: 133,
    "list-item": 110,
    "table-row": 131,
    "fenced-line": 16,
  },
  heading_counts: {
    Summary: 1,
    "Implementation Status": 8,
    "Language-Semantics Prerequisite": 2,
    "Slice-Closure Prerequisite": 1,
    "Next Slice: Saved Workspace Function References": 19,
    Motivation: 3,
    Goals: 16,
    "Non-Goals": 15,
    Terminology: 15,
    "Ownership Boundary": 19,
    "Command And Transport": 2,
    "Project Selection": 18,
    Coordinates: 3,
    Tools: 16,
    Resources: 8,
    "Definition And Reference Coverage": 16,
    "Semantic Locations": 7,
    "Package Source URI": 7,
    "Package Documentation URI": 6,
    "Resolution And Failure": 3,
    "Saved Snapshot Capture": 2,
    "Package Snapshots": 5,
    "Generated Package Documentation": 24,
    "Authority And Inputs": 8,
    "Topic Catalog": 16,
    "Executable Grammar": 2,
    "Checked Examples": 2,
    "Compiler-Owned Tables": 1,
    "Reference Snapshot": 3,
    "Documentation Search And Reads": 3,
    "LSP Integration": 7,
    "Agent Plugin": 20,
    "Safety And Privacy": 11,
    "Conformance Contract": 28,
    "Acceptance Model": 1,
    "Server And Project Selection": 13,
    "Diagnostics And Navigation": 13,
    "Virtual Locations And Package Documentation": 11,
    "Published Language Reference": 10,
    Plugin: 6,
    "Implementation Slices": 9,
    "Deferred Work": 10,
  },
  identity_sets: {
    evidence_gate: Array.from({ length: 22 }, (_, index) => `Q${String(index + 1).padStart(2, "0")}`),
    mcp_tools: ["check_project", "definition", "read_doc", "references", "refresh_workspace", "search_docs", "workspace_projects"],
    mcp_resource_schemes: ["veln-doc:", "veln-pkg:"],
    domain_errors: [
      "generation_failed",
      "incompatible_version",
      "invalid_cursor",
      "invalid_path",
      "invalid_position",
      "invalid_query",
      "project_ambiguous",
      "project_not_selected",
      "resource_capacity",
      "resource_not_found",
      "snapshot_changed",
      "source_required",
      "stale_snapshot",
    ],
    package_document_declarations: ["doc", "doc-digest", "PackageIdentity"],
    lsp_encodings: ["general.positionEncodings", "veln/virtualDocument"],
    plugin_cells: ["Codex", "Claude Code", "compatibility.toml", "mcpServers", ".lsp.json", ".mcp.json"],
  },
};

if (isMainModule()) {
  const repoRoot = process.cwd();
  if (process.argv.includes("--write-artifacts")) {
    writeArtifacts(repoRoot);
  }
  const result = validateAgentLanguageServicesInventory({
    repoRoot,
    checkDiffScope: !process.argv.includes("--skip-diff-scope"),
  });
  if (!result.valid) {
    const message = [
      "Update the agent language-services frozen inventory artifacts before merging; the inventory fixes the reviewed source universe used by the lifecycle migration.",
      ...result.errors.map((error) => `- ${error}`),
    ].join("\n");
    if (process.env.GITHUB_ACTIONS === "true") {
      console.error(renderGitHubErrorAnnotation(message));
    }
    console.error(message);
    process.exit(1);
  }
  console.log("Agent language-services frozen inventory is valid.");
}

export function validateAgentLanguageServicesInventory({ repoRoot, checkDiffScope = false }) {
  const proposal = fs.readFileSync(path.join(repoRoot, proposalPath), "utf8");
  const universe = readJson(repoRoot, universePath);
  const inventory = readJson(repoRoot, inventoryPath);
  const manifest = readJson(repoRoot, manifestPath);
  const schema = readJson(repoRoot, schemaPath);
  const parsed = parseSourceUniverse(proposal);
  const errors = [
    ...validateUniverse({ parsed, universe }),
    ...validateInventory({ inventory, parsed, universe, manifest }),
    ...validateLedgerSchema(schema),
  ];
  const ledger = buildAcceptanceLedger({ repoRoot, inventory, manifest });
  errors.push(...validateMigrationLedger({ repoRoot, ledger, inventory, manifest }).errors);
  if (checkDiffScope) {
    errors.push(...validateDiffScopeFromGit(repoRoot));
  }
  return { valid: errors.length === 0, errors };
}

export function writeArtifacts(repoRoot) {
  const proposal = fs.readFileSync(path.join(repoRoot, proposalPath), "utf8");
  const roots = parseSourceUniverse(proposal);
  const universe = {
    schema_version: 1,
    source_path: proposalPath,
    frozen_prefix: frozenPrefix,
    roots: roots.map((root) => ({
      id: root.id,
      heading: root.heading,
      kind: root.kind,
      line: root.line,
      digest: root.digest,
      conformance: root.conformance,
      identities: root.identities,
    })),
  };
  const inventoryRoots = roots.map((root) => {
    const children = splitLeafChildren(root).map((child, index, children) => ({
      id: `${root.id}.c${String(index + 1).padStart(2, "0")}`,
      lifecycle: classifyLifecycle(child.text, root.heading),
      spans: child.spans,
      digest: digest(child.text),
    }));
    return {
      id: root.id,
      heading: root.heading,
      kind: root.kind,
      line: root.line,
      digest: root.digest,
      text: root.text,
      child_count: children.length,
      children,
      separator_spans: separatorSpans(root.text, children.flatMap((child) => child.spans)),
    };
  });
  const inventory = {
    schema_version: 1,
    source_path: proposalPath,
    scalar_indexing: "unicode-scalar",
    digest: "sha256",
    roots: inventoryRoots,
  };
  const leaves = inventoryRoots.flatMap((root) => root.children.map((child) => ({
    source_id: child.id,
    parent_id: root.id,
    lifecycle: child.lifecycle,
    conformance: universe.roots.find((candidate) => candidate.id === root.id).conformance,
    spans: child.spans,
  })));
  const manifest = {
    schema_version: 1,
    source_path: proposalPath,
    lifecycle_values: [...lifecycleValues],
    leaves,
  };
  fs.writeFileSync(path.join(repoRoot, universePath), `${JSON.stringify(universe, null, 2)}\n`);
  fs.writeFileSync(path.join(repoRoot, inventoryPath), `${JSON.stringify(inventory, null, 2)}\n`);
  fs.writeFileSync(path.join(repoRoot, manifestPath), `${JSON.stringify(manifest, null, 2)}\n`);
  fs.writeFileSync(path.join(repoRoot, schemaPath), `${JSON.stringify(migrationLedgerSchema(), null, 2)}\n`);
}

export function parseSourceUniverse(markdown) {
  const lines = markdown.split("\n");
  let index = 0;
  if (lines[0] === "---") {
    index = 1;
    while (index < lines.length && lines[index] !== "---") index += 1;
    index += 1;
  }
  const roots = [];
  let heading = "";
  let inFence = false;
  let fenceLine = 0;
  while (index < lines.length) {
    const line = lines[index];
    const headingMatch = line.match(/^(#{1,6})\s+(.+?)\s*$/);
    if (!inFence && headingMatch) {
      heading = headingMatch[2];
      index += 1;
      continue;
    }
    if (line.startsWith("```")) {
      inFence = !inFence;
      fenceLine = index + 1;
      index += 1;
      continue;
    }
    if (inFence) {
      if (line.trim() !== "") roots.push(sourceRecord({ kind: "fenced-line", heading, line: index + 1, text: line }));
      index += 1;
      continue;
    }
    if (line.trim() === "") {
      index += 1;
      continue;
    }
    if (isTableRow(line)) {
      while (index < lines.length && isTableRow(lines[index])) {
        if (!isTableSeparator(lines[index])) {
          roots.push(sourceRecord({ kind: "table-row", heading, line: index + 1, text: lines[index] }));
        }
        index += 1;
      }
      continue;
    }
    if (/^\s*(?:[-*+]|\d+\.)\s+/.test(line)) {
      roots.push(sourceRecord({ kind: "list-item", heading, line: index + 1, text: line }));
      index += 1;
      continue;
    }
    const paragraphLines = [];
    const startLine = index + 1;
    while (
      index < lines.length
      && lines[index].trim() !== ""
      && !/^#{1,6}\s+/.test(lines[index])
      && !lines[index].startsWith("```")
      && !isTableRow(lines[index])
      && !/^\s*(?:[-*+]|\d+\.)\s+/.test(lines[index])
    ) {
      paragraphLines.push(lines[index]);
      index += 1;
    }
    roots.push(sourceRecord({ kind: "paragraph", heading, line: startLine, text: paragraphLines.join("\n") }));
  }
  return roots.map((root, rootIndex) => ({
    ...root,
    id: `${frozenPrefix}S${String(rootIndex + 1).padStart(4, "0")}`,
  }));

  function sourceRecord({ kind, heading, line, text }) {
    return {
      kind,
      heading,
      line,
      text,
      digest: digest(text),
      conformance: isConformanceSource({ heading, text }),
      identities: identities(text),
    };
  }
}

export function validateUniverse({ parsed, universe }) {
  const errors = [];
  if (universe.schema_version !== 1) errors.push(`${universePath}: schema_version must be 1.`);
  if (universe.source_path !== proposalPath) errors.push(`${universePath}: source_path must be ${proposalPath}.`);
  const seen = new Set();
  if (!Array.isArray(universe.roots)) return [`${universePath}: roots must be an array.`];
  errors.push(...validateSourceUniverseContract(universe));
  if (universe.roots.length !== parsed.length) {
    errors.push(`${universePath}: expected ${parsed.length} root source records, found ${universe.roots.length}.`);
  }
  for (let i = 0; i < Math.min(parsed.length, universe.roots.length); i += 1) {
    const expected = parsed[i];
    const actual = universe.roots[i];
    if (seen.has(actual.id)) errors.push(`${universePath}: duplicate source id ${actual.id}.`);
    seen.add(actual.id);
    for (const key of ["id", "heading", "kind", "line", "digest", "conformance"]) {
      if (actual[key] !== expected[key]) errors.push(`${universePath}: ${actual.id ?? `root ${i + 1}`} has wrong ${key}.`);
    }
    if (JSON.stringify(actual.identities ?? []) !== JSON.stringify(expected.identities)) {
      errors.push(`${universePath}: ${actual.id} has stale named identities.`);
    }
  }
  return errors;
}

function validateSourceUniverseContract(universe) {
  const errors = [];
  const roots = universe.roots ?? [];
  if (roots.length !== sourceUniverseContract.root_count) {
    errors.push(`${universePath}: source-universe contract expects ${sourceUniverseContract.root_count} root records.`);
  }
  for (const [kind, count] of Object.entries(sourceUniverseContract.kind_counts)) {
    const actual = roots.filter((root) => root.kind === kind).length;
    if (actual !== count) errors.push(`${universePath}: source-universe contract expects ${count} ${kind} records, found ${actual}.`);
  }
  for (const [heading, count] of Object.entries(sourceUniverseContract.heading_counts)) {
    const actual = roots.filter((root) => root.heading === heading).length;
    if (actual !== count) errors.push(`${universePath}: source-universe contract expects ${count} records under ${heading}, found ${actual}.`);
  }
  const identities = new Set(roots.flatMap((root) => root.identities ?? []));
  for (const [name, expected] of Object.entries(sourceUniverseContract.identity_sets)) {
    for (const identity of expected) {
      if (!identities.has(identity)) errors.push(`${universePath}: source-universe contract missing ${name} identity ${identity}.`);
    }
  }
  return errors;
}

export function validateInventory({ inventory, parsed, universe, manifest }) {
  const errors = [];
  if (inventory.schema_version !== 1) errors.push(`${inventoryPath}: schema_version must be 1.`);
  if (inventory.source_path !== proposalPath) errors.push(`${inventoryPath}: source_path must be ${proposalPath}.`);
  if (inventory.scalar_indexing !== "unicode-scalar") errors.push(`${inventoryPath}: scalar_indexing must be unicode-scalar.`);
  if (!Array.isArray(inventory.roots)) return [`${inventoryPath}: roots must be an array.`];
  if (inventory.roots.length !== universe.roots.length) {
    errors.push(`${inventoryPath}: expected one inventory root for each source-universe root.`);
  }
  const leafMap = new Map();
  const manifestMap = new Map((manifest.leaves ?? []).map((leaf) => [leaf.source_id, leaf]));
  const sourceIds = new Set();
  for (let i = 0; i < inventory.roots.length; i += 1) {
    const root = inventory.roots[i];
    const expected = parsed[i];
    const contract = universe.roots[i];
    if (sourceIds.has(root.id)) errors.push(`${inventoryPath}: duplicate inventory root ${root.id}.`);
    sourceIds.add(root.id);
    if (!expected || root.id !== expected.id || root.digest !== expected.digest || root.text !== expected.text) {
      errors.push(`${inventoryPath}: ${root.id ?? `root ${i + 1}`} does not match frozen source text and digest.`);
    }
    if (root.id !== contract?.id) errors.push(`${inventoryPath}: ${root.id} is not aligned with the source-universe order.`);
    if (!Number.isInteger(root.child_count) || root.child_count < 1) errors.push(`${inventoryPath}: ${root.id} must declare a positive child_count.`);
    if (!Array.isArray(root.children) || root.children.length !== root.child_count) {
      errors.push(`${inventoryPath}: ${root.id} child_count does not match children.`);
      continue;
    }
    const childIds = root.children.map((child) => child.id);
    for (let childIndex = 0; childIndex < root.children.length; childIndex += 1) {
      const child = root.children[childIndex];
      const expectedId = `${root.id}.c${String(childIndex + 1).padStart(2, "0")}`;
      if (child.id !== expectedId) errors.push(`${inventoryPath}: ${root.id} child ${childIndex + 1} must be ${expectedId}.`);
      if (!lifecycleValues.has(child.lifecycle)) errors.push(`${inventoryPath}: ${child.id} has invalid lifecycle ${child.lifecycle}.`);
      if (leafMap.has(child.id)) errors.push(`${inventoryPath}: duplicate inventory leaf ${child.id}.`);
      leafMap.set(child.id, { child, root, contract });
      const manifestLeaf = manifestMap.get(child.id);
      if (!manifestLeaf) {
        errors.push(`${manifestPath}: missing lifecycle leaf ${child.id}.`);
      } else {
        if (manifestLeaf.lifecycle !== child.lifecycle) errors.push(`${inventoryPath}: ${child.id} lifecycle differs from reviewed manifest.`);
        if (manifestLeaf.parent_id !== root.id) errors.push(`${manifestPath}: ${child.id} has wrong parent_id.`);
        if (manifestLeaf.conformance !== contract?.conformance) errors.push(`${manifestPath}: ${child.id} has wrong conformance flag.`);
      }
      if (digest(spannedText(root.text, child.spans)) !== child.digest) errors.push(`${inventoryPath}: ${child.id} has stale child digest.`);
    }
    const coverage = validateSpanPartition({ rootId: root.id, text: root.text, children: root.children, separatorSpans: root.separator_spans ?? [] });
    errors.push(...coverage.map((error) => `${inventoryPath}: ${error}`));
    if (root.child_count > 1 && childIds.some((id) => !id.endsWith(`c${String(childIds.indexOf(id) + 1).padStart(2, "0")}`))) {
      errors.push(`${inventoryPath}: ${root.id} child ids must be contiguous.`);
    }
  }
  for (const manifestLeaf of manifest.leaves ?? []) {
    if (!leafMap.has(manifestLeaf.source_id)) errors.push(`${manifestPath}: unexpected lifecycle leaf ${manifestLeaf.source_id}.`);
  }
  return errors;
}

export function validateMigrationLedger({ repoRoot = process.cwd(), ledger, inventory, manifest }) {
  const errors = [];
  if (!ledger || ledger.schema_version !== 1) errors.push("migration ledger: schema_version must be 1.");
  errors.push(...validateMigrationLedgerJsonSchema({ ledger, schema: migrationLedgerSchema() }).errors);
  if (!Array.isArray(ledger?.entries)) return { valid: false, errors: [...errors, "migration ledger: entries must be an array."] };
  const leaves = new Map((manifest.leaves ?? []).map((leaf) => [leaf.source_id, leaf]));
  const parentIds = new Set((inventory.roots ?? []).filter((root) => root.child_count > 0).map((root) => root.id));
  const seen = new Set();
  for (const entry of ledger.entries) {
    const id = entry?.source_id;
    if (typeof id !== "string") {
      errors.push("migration ledger: each entry must have a string source_id.");
      continue;
    }
    if (/[*]|\.\.|^all\b|remaining|range/i.test(id)) errors.push(`migration ledger: ${id} is a range, wildcard, or catch-all source_id.`);
    if (parentIds.has(id)) errors.push(`migration ledger: ${id} maps a parent that declares children.`);
    if (seen.has(id)) errors.push(`migration ledger: duplicate leaf mapping ${id}.`);
    seen.add(id);
    const leaf = leaves.get(id);
    if (!leaf) {
      errors.push(`migration ledger: unknown leaf ${id}.`);
      continue;
    }
    if (entry.lifecycle !== leaf.lifecycle) errors.push(`migration ledger: ${id} lifecycle must be ${leaf.lifecycle}.`);
    if (leaf.conformance && entry.lifecycle === "removed") errors.push(`migration ledger: ${id} is conformance content and cannot be removed.`);
    errors.push(...validateDestination({ repoRoot, entry }));
  }
  for (const id of leaves.keys()) {
    if (!seen.has(id)) errors.push(`migration ledger: missing leaf mapping ${id}.`);
  }
  return { valid: errors.length === 0, errors };
}

export function validateDiffScope({ changes, baseHasFrozen, headHasFrozen }) {
  const errors = [];
  const bootstrap = !baseHasFrozen && headHasFrozen;
  if (!bootstrap) return errors;
  const allowed = new Set([
    ...artifactPaths,
    ...validatorPaths,
    "docs/reference/agent-language-services-frozen-inventory.md",
    ".github/workflows/workflow--test-scripts.yaml",
  ]);
  for (const change of changes) {
    const file = change.path;
    if (allowed.has(file)) continue;
    if (file === proposalPath) {
      errors.push(`${file}: restore the umbrella proposal; the bootstrap inventory must freeze the pre-migration source universe without reorganizing it.`);
    } else if (isProtectedToolchainPath(file)) {
      errors.push(`${file}: move toolchain, MCP harness, executable fixture, or semantic-baseline changes to a later behavior PR.`);
    } else {
      errors.push(`${file}: keep the first frozen-inventory PR limited to the inventory artifacts, validator, tests, and workflow registration.`);
    }
  }
  return errors;
}

export function buildAcceptanceLedger({ repoRoot, inventory, manifest }) {
  const destinationFor = (lifecycle) => {
    if (lifecycle === "current") {
      return {
        kind: "current",
        path: "docs/specification/mcp.md",
        anchor: "mcp-workspace-projects-diagnostics-and-definitions",
        evidence: ["examples/specification/doc/generated-markdown/case.toml"],
      };
    }
    if (lifecycle === "completed") {
      return {
        kind: "completed",
        path: "docs/reference/implemented-proposals/agent-module-package-docs.md",
        anchor: "agent-module-package-and-documentation-model",
      };
    }
    return {
      kind: "planned",
      path: proposalPath,
      anchor: "acceptance-model",
    };
  };
  return {
    schema_version: 1,
    entries: (manifest.leaves ?? []).map((leaf) => ({
      source_id: leaf.source_id,
      lifecycle: leaf.lifecycle,
      destination: destinationFor(leaf.lifecycle),
    })),
  };
}

function validateDestination({ repoRoot, entry }) {
  const errors = [];
  const destination = entry.destination;
  if (!destination || destination.kind !== entry.lifecycle) {
    return [`migration ledger: ${entry.source_id} destination.kind must equal its lifecycle.`];
  }
  if (entry.lifecycle === "removed") {
    if (typeof destination.rationale !== "string" || destination.rationale.trim() === "") {
      errors.push(`migration ledger: ${entry.source_id} removed destination needs a rationale.`);
    }
    if (!destination.supersedes || typeof destination.supersedes.path !== "string" || typeof destination.supersedes.anchor !== "string") {
      errors.push(`migration ledger: ${entry.source_id} removed destination needs a superseding destination.`);
    } else {
      errors.push(...validateMarkdownDestination({
        repoRoot,
        sourceId: entry.source_id,
        lifecycle: "removed",
        pathValue: destination.supersedes.path,
        anchor: destination.supersedes.anchor,
      }));
    }
    return errors;
  }
  if (typeof destination.path !== "string" || typeof destination.anchor !== "string") {
    return [`migration ledger: ${entry.source_id} destination needs path and anchor.`];
  }
  errors.push(...validateMarkdownDestination({
    repoRoot,
    sourceId: entry.source_id,
    lifecycle: entry.lifecycle,
    pathValue: destination.path,
    anchor: destination.anchor,
  }));
  if (entry.lifecycle === "current") {
    if (!Array.isArray(destination.evidence) || destination.evidence.length === 0) {
      errors.push(`migration ledger: ${entry.source_id} current destination needs checked evidence.`);
    } else {
      const entryEvidenceSeen = new Set();
      for (const evidence of destination.evidence) {
        if (entryEvidenceSeen.has(evidence)) errors.push(`migration ledger: duplicate checked evidence ${evidence}.`);
        entryEvidenceSeen.add(evidence);
        if (!isCheckedEvidencePath(evidence) || !fs.existsSync(path.join(repoRoot, evidence))) {
          errors.push(`migration ledger: ${entry.source_id} evidence ${evidence} must resolve to an allowlisted checked route.`);
        }
      }
    }
  }
  return errors;
}

function validateMarkdownDestination({ repoRoot, sourceId, lifecycle, pathValue, anchor }) {
  const errors = [];
  const fullPath = path.join(repoRoot, pathValue);
  if (!fs.existsSync(fullPath)) {
    return [`migration ledger: ${sourceId} destination path ${pathValue} does not exist.`];
  }
  const markdown = fs.readFileSync(fullPath, "utf8");
  if (!markdownHasAnchor(markdown, anchor)) {
    errors.push(`migration ledger: ${sourceId} destination anchor ${anchor} does not exist in ${pathValue}.`);
  }
  const frontmatter = parseMarkdownFrontmatter(markdown);
  const role = frontmatter.role;
  const authority = frontmatter.authority;
  if (lifecycle === "current" && (role !== "specification" || authority !== "normative")) {
    errors.push(`migration ledger: ${sourceId} current destination must be a normative specification.`);
  }
  if (lifecycle === "completed" && role !== "implementation-record") {
    errors.push(`migration ledger: ${sourceId} completed destination must be an implementation record.`);
  }
  if (lifecycle === "planned" && role !== "proposal") {
    errors.push(`migration ledger: ${sourceId} planned destination must be an active proposal.`);
  }
  if (lifecycle === "removed" && role === "proposal") {
    errors.push(`migration ledger: ${sourceId} removed superseding destination must not be an active proposal.`);
  }
  return errors;
}

function validateLedgerSchema(schema) {
  const errors = [];
  if (schema?.$id !== "agent-language-services-migration-ledger.schema.json") errors.push(`${schemaPath}: $id is wrong.`);
  if (schema?.type !== "object") errors.push(`${schemaPath}: root type must be object.`);
  const entries = schema?.properties?.entries;
  if (entries?.type !== "array") errors.push(`${schemaPath}: entries must be an array schema.`);
  if (entries?.items?.additionalProperties !== false) errors.push(`${schemaPath}: ledger entries must be closed objects.`);
  if (!entries?.items?.properties?.source_id?.pattern?.includes("agent-language-services")) {
    errors.push(`${schemaPath}: source_id pattern must be bounded to the frozen source namespace.`);
  }
  const destination = entries?.items?.properties?.destination;
  if (destination?.properties?.supersedes?.additionalProperties !== false) {
    errors.push(`${schemaPath}: removed superseding destinations must be closed objects.`);
  }
  if (!destination?.allOf?.length) {
    errors.push(`${schemaPath}: destination schema must encode lifecycle-specific requirements.`);
  }
  return errors;
}

export function validateMigrationLedgerJsonSchema({ ledger, schema = migrationLedgerSchema() }) {
  const errors = [];
  if (schema?.$id !== "agent-language-services-migration-ledger.schema.json") {
    errors.push("migration ledger schema: unsupported schema.");
  }
  if (!ledger || typeof ledger !== "object" || Array.isArray(ledger)) {
    return { valid: false, errors: ["migration ledger schema: ledger must be an object."] };
  }
  rejectExtraKeys({ errors, label: "migration ledger", value: ledger, allowed: ["schema_version", "entries"] });
  requireKeys({ errors, label: "migration ledger", value: ledger, keys: ["schema_version", "entries"] });
  if (ledger.schema_version !== 1) errors.push("migration ledger schema: schema_version must be 1.");
  if (!Array.isArray(ledger.entries) || ledger.entries.length === 0) {
    return { valid: false, errors: [...errors, "migration ledger schema: entries must be a nonempty array."] };
  }
  for (const [index, entry] of ledger.entries.entries()) {
    const label = `migration ledger schema: entries[${index}]`;
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
      errors.push(`${label} must be an object.`);
      continue;
    }
    rejectExtraKeys({ errors, label, value: entry, allowed: ["source_id", "lifecycle", "destination"] });
    requireKeys({ errors, label, value: entry, keys: ["source_id", "lifecycle", "destination"] });
    if (typeof entry.source_id !== "string" || !/^agent-language-services\/S[0-9]{4}\.c[0-9]{2}$/.test(entry.source_id)) {
      errors.push(`${label}.source_id must be one frozen leaf id.`);
    }
    if (typeof entry.source_id === "string" && /[*]|\.\.|^all\b|remaining|range/i.test(entry.source_id)) {
      errors.push(`${label}.source_id must not be a range, wildcard, or catch-all.`);
    }
    if (!lifecycleValues.has(entry.lifecycle)) errors.push(`${label}.lifecycle is invalid.`);
    validateDestinationShape({ errors, label: `${label}.destination`, lifecycle: entry.lifecycle, destination: entry.destination });
  }
  return { valid: errors.length === 0, errors };
}

function validateDestinationShape({ errors, label, lifecycle, destination }) {
  if (!destination || typeof destination !== "object" || Array.isArray(destination)) {
    errors.push(`${label} must be an object.`);
    return;
  }
  rejectExtraKeys({ errors, label, value: destination, allowed: ["kind", "path", "anchor", "evidence", "rationale", "supersedes"] });
  requireKeys({ errors, label, value: destination, keys: ["kind"] });
  if (!lifecycleValues.has(destination.kind)) errors.push(`${label}.kind is invalid.`);
  if (destination.kind !== lifecycle) errors.push(`${label}.kind must equal lifecycle.`);
  if (lifecycle === "removed") {
    requireKeys({ errors, label, value: destination, keys: ["rationale", "supersedes"] });
    if (typeof destination.rationale !== "string" || destination.rationale.trim() === "") errors.push(`${label}.rationale must be nonempty.`);
    if (!destination.supersedes || typeof destination.supersedes !== "object" || Array.isArray(destination.supersedes)) {
      errors.push(`${label}.supersedes must be an object.`);
    } else {
      rejectExtraKeys({ errors, label: `${label}.supersedes`, value: destination.supersedes, allowed: ["path", "anchor"] });
      requireKeys({ errors, label: `${label}.supersedes`, value: destination.supersedes, keys: ["path", "anchor"] });
      if (typeof destination.supersedes.path !== "string" || destination.supersedes.path.trim() === "") errors.push(`${label}.supersedes.path must be nonempty.`);
      if (typeof destination.supersedes.anchor !== "string" || destination.supersedes.anchor.trim() === "") errors.push(`${label}.supersedes.anchor must be nonempty.`);
    }
    return;
  }
  requireKeys({ errors, label, value: destination, keys: ["path", "anchor"] });
  if (typeof destination.path !== "string" || destination.path.trim() === "") errors.push(`${label}.path must be nonempty.`);
  if (typeof destination.anchor !== "string" || destination.anchor.trim() === "") errors.push(`${label}.anchor must be nonempty.`);
  if (lifecycle === "current") {
    if (!Array.isArray(destination.evidence) || destination.evidence.length === 0) {
      errors.push(`${label}.evidence must be a nonempty array.`);
    } else if (new Set(destination.evidence).size !== destination.evidence.length) {
      errors.push(`${label}.evidence must be unique.`);
    }
  }
}

function rejectExtraKeys({ errors, label, value, allowed }) {
  const allowedSet = new Set(allowed);
  for (const key of Object.keys(value ?? {})) {
    if (!allowedSet.has(key)) errors.push(`${label} has unexpected key ${key}.`);
  }
}

function requireKeys({ errors, label, value, keys }) {
  for (const key of keys) {
    if (!Object.hasOwn(value ?? {}, key)) errors.push(`${label} missing ${key}.`);
  }
}

function migrationLedgerSchema() {
  return {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: "agent-language-services-migration-ledger.schema.json",
    type: "object",
    additionalProperties: false,
    required: ["schema_version", "entries"],
    properties: {
      schema_version: { const: 1 },
      entries: {
        type: "array",
        minItems: 1,
        items: {
          type: "object",
          additionalProperties: false,
          required: ["source_id", "lifecycle", "destination"],
          properties: {
            source_id: {
              type: "string",
              pattern: "^agent-language-services/S[0-9]{4}\\.c[0-9]{2}$",
              not: { pattern: "(\\*|\\.\\.|remaining|^all\\b|range)" },
            },
            lifecycle: { enum: ["current", "completed", "planned", "removed"] },
            destination: {
              type: "object",
              additionalProperties: false,
              required: ["kind"],
              properties: {
                kind: { enum: ["current", "completed", "planned", "removed"] },
                path: { type: "string" },
                anchor: { type: "string" },
                evidence: {
                  type: "array",
                  minItems: 1,
                  uniqueItems: true,
                  items: { type: "string" },
                },
                rationale: { type: "string", minLength: 1 },
                supersedes: {
                  type: "object",
                  additionalProperties: false,
                  required: ["path", "anchor"],
                  properties: {
                    path: { type: "string", minLength: 1 },
                    anchor: { type: "string", minLength: 1 },
                  },
                },
              },
              allOf: [
                {
                  if: { properties: { kind: { const: "current" } } },
                  then: { required: ["path", "anchor", "evidence"] },
                },
                {
                  if: { properties: { kind: { enum: ["completed", "planned"] } } },
                  then: { required: ["path", "anchor"] },
                },
                {
                  if: { properties: { kind: { const: "removed" } } },
                  then: { required: ["rationale", "supersedes"] },
                },
              ],
            },
          },
        },
      },
    },
  };
}

function splitLeafChildren(root) {
  if (root.kind === "table-row") {
    const cells = tableCellSpans(root.text);
    if (cells.length > 0) return cells.map((span) => ({ text: scalarSlice(root.text, span.start, span.end), spans: [span] }));
  }
  const scalars = [...root.text];
  const boundaries = [];
  for (let i = 0; i < scalars.length; i += 1) {
    if ((scalars[i] === "." || scalars[i] === ";" || scalars[i] === ":") && scalars[i + 1] === " ") {
      boundaries.push(i + 1);
    }
  }
  if (boundaries.length === 0) return [{ text: root.text, spans: [{ start: 0, end: scalars.length }] }];
  const spans = [];
  let start = 0;
  for (const boundary of boundaries) {
    spans.push({ start, end: boundary });
    start = boundary;
    while (scalars[start] === " ") start += 1;
  }
  if (start < scalars.length) spans.push({ start, end: scalars.length });
  return spans.filter((span) => span.end > span.start).map((span) => ({
    text: scalarSlice(root.text, span.start, span.end),
    spans: [span],
  }));
}

function validateSpanPartition({ rootId, text, children, separatorSpans }) {
  const errors = [];
  const length = [...text].length;
  const allSpans = [
    ...children.flatMap((child) => (child.spans ?? []).map((span) => ({ ...span, kind: "child", child: child.id }))),
    ...separatorSpans.map((span) => ({ ...span, kind: "separator" })),
  ].sort((a, b) => a.start - b.start || a.end - b.end);
  let cursor = 0;
  for (const span of allSpans) {
    if (!Number.isInteger(span.start) || !Number.isInteger(span.end) || span.start < 0 || span.end > length || span.start >= span.end) {
      errors.push(`${rootId} has an out-of-range span.`);
      continue;
    }
    if (span.start !== cursor) {
      errors.push(`${rootId} has a span gap or overlap at scalar ${cursor}.`);
      cursor = Math.max(cursor, span.end);
      continue;
    }
    if (span.kind === "separator" && /[^\s|]/u.test(scalarSlice(text, span.start, span.end))) {
      errors.push(`${rootId} has a separator span containing source content.`);
    }
    cursor = span.end;
  }
  if (cursor !== length) errors.push(`${rootId} has uncovered source text after scalar ${cursor}.`);
  return errors;
}

function separatorSpans(text, childSpans) {
  const spans = [];
  let cursor = 0;
  for (const span of [...childSpans].sort((a, b) => a.start - b.start || a.end - b.end)) {
    if (cursor < span.start) spans.push({ start: cursor, end: span.start });
    cursor = span.end;
  }
  if (cursor < [...text].length) spans.push({ start: cursor, end: [...text].length });
  return spans;
}

function tableCellSpans(text) {
  const scalars = [...text];
  const spans = [];
  let cellStart = text.startsWith("|") ? 1 : 0;
  for (let i = cellStart; i <= scalars.length; i += 1) {
    if (i === scalars.length || scalars[i] === "|") {
      let start = cellStart;
      let end = i;
      while (scalars[start] === " ") start += 1;
      while (end > start && scalars[end - 1] === " ") end -= 1;
      if (end > start) spans.push({ start, end });
      cellStart = i + 1;
    }
  }
  return spans;
}

function classifyLifecycle(text, heading) {
  const lower = `${heading} ${text}`.toLowerCase();
  if (/\bimplemented\b|\bcurrently exposes\b|\balready implemented\b|\bspecified in\b|\bcurrent\b/.test(lower)) return "current";
  if (/\bcompleted\b|\bclosed\b|\bresolved-decision\b/.test(lower)) return "completed";
  return "planned";
}

function isConformanceSource({ heading, text }) {
  const lower = `${heading} ${text}`.toLowerCase();
  return /\bmust\b|\breject|\baccept|\breturn|\bresource|\btool|\bschema|\berror|\buri\b|\bq\d\d\b|\bevidence\b|\bcase\b|\bexpected result\b|\bplanned evidence\b|\bcodex\b|\bclaude\b/.test(lower);
}

function identities(text) {
  const values = new Set();
  for (const match of text.matchAll(/\bQ\d\d\b/g)) values.add(match[0]);
  for (const match of text.matchAll(/`([^`]+)`/g)) values.add(match[1]);
  if (/Codex/.test(text)) values.add("Codex");
  if (/Claude Code/.test(text)) values.add("Claude Code");
  return [...values].sort();
}

function validateDiffScopeFromGit(repoRoot) {
  const baseSha = process.env.DOC_FRONTMATTER_BASE_SHA;
  const headSha = process.env.DOC_FRONTMATTER_HEAD_SHA;
  if (!baseSha || !headSha || /^0+$/.test(baseSha)) return [];
  const diff = spawnSync("git", ["diff", "--name-status", "-z", baseSha, headSha], { cwd: repoRoot, encoding: "utf8" });
  if (diff.status !== 0) return [`diff scope: unable to inspect changed paths: ${diff.stderr.trim()}`];
  const parts = diff.stdout.split("\0").filter(Boolean);
  const changes = [];
  for (let i = 0; i < parts.length; i += 1) {
    const status = parts[i];
    if (/^R|^C/.test(status)) {
      changes.push({ status, path: parts[i + 2] });
      i += 2;
    } else {
      changes.push({ status, path: parts[i + 1] });
      i += 1;
    }
  }
  const baseHasFrozen = gitHasPath(repoRoot, baseSha, inventoryPath);
  const headHasFrozen = gitHasPath(repoRoot, headSha, inventoryPath);
  return validateDiffScope({ changes, baseHasFrozen, headHasFrozen });
}

function gitHasPath(repoRoot, revision, file) {
  const result = spawnSync("git", ["cat-file", "-e", `${revision}:${file}`], { cwd: repoRoot, encoding: "utf8" });
  return result.status === 0;
}

function isProtectedToolchainPath(file) {
  return file.startsWith("crates/")
    || file.startsWith("examples/specification/mcp/")
    || file.startsWith("examples/specification/run/")
    || file.includes("mcp")
    || file.includes("baseline");
}

function isTableRow(line) {
  return /^\s*\|.*\|\s*$/.test(line);
}

function isTableSeparator(line) {
  return /^\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)+\|?\s*$/.test(line);
}

function markdownHasAnchor(markdown, anchor) {
  return markdown.split("\n").some((line) => {
    const match = line.match(/^#{1,6}\s+(.+?)\s*$/);
    return match && slug(match[1]) === anchor;
  });
}

function slug(text) {
  return text.toLowerCase().replace(/`/g, "").replace(/[^a-z0-9\s-]/g, "").trim().replace(/\s+/g, "-");
}

function isCheckedEvidencePath(file) {
  return file.startsWith("examples/specification/")
    || file.startsWith("docs/specification/source-surface-fixtures/")
    || file === "docs/specification/source-surface-executable.pl";
}

function readJson(repoRoot, file) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, file), "utf8"));
}

function parseMarkdownFrontmatter(markdown) {
  const lines = markdown.split("\n");
  if (lines[0] !== "---") return {};
  const result = {};
  for (let index = 1; index < lines.length && lines[index] !== "---"; index += 1) {
    const match = lines[index].match(/^([A-Za-z0-9_-]+):\s*(.*?)\s*$/);
    if (match) result[match[1]] = match[2].replace(/^["']|["']$/g, "");
  }
  return result;
}

function digest(text) {
  return crypto.createHash("sha256").update(text, "utf8").digest("hex");
}

function scalarSlice(text, start, end) {
  return [...text].slice(start, end).join("");
}

function spannedText(text, spans) {
  return (spans ?? []).map((span) => scalarSlice(text, span.start, span.end)).join("");
}

function renderGitHubErrorAnnotation(message) {
  return `::error title=Invalid agent language-services inventory::${message.replaceAll("%", "%25").replaceAll("\n", "%0A").replaceAll("\r", "%0D")}`;
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}
