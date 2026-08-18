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
const closedClientPlatformMatrix = [
  {
    key: "codex/x86_64-unknown-linux-gnu",
    client: "codex",
    platform: "x86_64-unknown-linux-gnu",
  },
  {
    key: "claude-code/x86_64-unknown-linux-gnu",
    client: "claude-code",
    platform: "x86_64-unknown-linux-gnu",
  },
];
const sourceUniverseContract = {
  root_count: 393,
  kind_counts: {
    paragraph: 133,
    "list-item": 110,
    "table-row": 134,
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
    "Agent Plugin": 23,
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
    plugin_client_platform_keys: closedClientPlatformMatrix.map((row) => row.key),
  },
  named_source_sets: {
    saved_reference_boundary_rows: [
      ["agent-language-services/S0016", "A checked `references` input with `source`, `line`, and `column`."],
      ["agent-language-services/S0017", "Selected-project and anonymous single-file capture using the existing navigation selection rules."],
      ["agent-language-services/S0018", "Canonical `file:` locations for the current shared language service's project-owned function reference sites, in deterministic order."],
      ["agent-language-services/S0019", "Explicit project or single-file scope metadata, including whether the result is project-wide."],
      ["agent-language-services/S0020", "Empty success for a valid position with no supported function reference search."],
      ["agent-language-services/S0021", "Existing path, coordinate, schema, and stable-capture failure behavior."],
    ],
    saved_reference_acceptance_rows: [
      ["agent-language-services/S0024", "Request references at a project-owned function call in a selected project."],
      ["agent-language-services/S0025", "Request references at the unrelated ambiguous constructor call in that case."],
      ["agent-language-services/S0026", "Request references for an accepted source outside a selected project's owned-source set."],
      ["agent-language-services/S0027", "Supply an invalid position or a schema-invalid coordinate."],
      ["agent-language-services/S0028", "Replace a captured source identity or bytes during the operation."],
      ["agent-language-services/S0029", "List MCP tools after initialization."],
    ],
    closed_navigation_matrix_rows: [
      ["agent-language-services/S0148", "project-owned public and private functions"],
      ["agent-language-services/S0149", "source types and their constructors"],
      ["agent-language-services/S0150", "schemas"],
      ["agent-language-services/S0151", "public member aliases"],
      ["agent-language-services/S0152", "function and handler parameters"],
      ["agent-language-services/S0153", "local `let` and pattern bindings"],
      ["agent-language-services/S0154", "handler operation clause bindings"],
      ["agent-language-services/S0155", "handler context parameters"],
      ["agent-language-services/S0156", "exact test-companion access to target-private declarations"],
      ["agent-language-services/S0157", "visible declarations in direct dependency exports"],
      ["agent-language-services/S0158", "visible standard-library declarations"],
    ],
    closed_topic_matrix_rows: [
      ["agent-language-services/S0227", "lexical structure and the complete executable grammar"],
      ["agent-language-services/S0228", "modules, imports, packages, exports, and visibility"],
      ["agent-language-services/S0229", "declarations and aliases"],
      ["agent-language-services/S0230", "expressions, operators, and patterns"],
      ["agent-language-services/S0231", "types, inference, and constructors"],
      ["agent-language-services/S0232", "effects and handlers"],
      ["agent-language-services/S0233", "contracts"],
      ["agent-language-services/S0234", "schemas"],
      ["agent-language-services/S0235", "holes"],
      ["agent-language-services/S0236", "tests, documentation comments, and doctests"],
    ],
    unresolved_acceptance_rows: [
      ["agent-language-services/S0326", "Navigate below an unselected descendant manifest."],
      ["agent-language-services/S0327", "Add, remove, or rename a manifest."],
      ["agent-language-services/S0328", "Start through a symbolic base alias."],
      ["agent-language-services/S0329", "Supply a path containing a directory or file symbolic link."],
      ["agent-language-services/S0331", "Change a selected root identity, anonymous base identity, manifest, source, dependency input, or file set during capture."],
      ["agent-language-services/S0332", "List projects or send malformed inventory-tool input."],
      ["agent-language-services/S0338", "Resolve references for every symbol in the closed v1 navigation matrix, including shadowing and same-spelled fields."],
      ["agent-language-services/S0339", "Search references to a dependency symbol from one selected project."],
      ["agent-language-services/S0340", "Continue a paged reference result."],
      ["agent-language-services/S0341", "Use a tampered, cross-server, restarted, evicted, or pre-refresh cursor."],
      ["agent-language-services/S0342", "Resolve an exported dependency declaration."],
      ["agent-language-services/S0343", "Resolve a private dependency declaration from a consumer."],
      ["agent-language-services/S0344", "Resolve a standard-library declaration."],
      ["agent-language-services/S0345", "Address empty, LF, CRLF, non-BMP, end-of-line, end-of-file, token-end, decimal and exponent integer spellings, non-integer numeric spellings, and oversized positive positions."],
      ["agent-language-services/S0346", "Run LSP and MCP on the same saved project without overlays."],
      ["agent-language-services/S0347", "Apply an LSP open-document overlay."],
      ["agent-language-services/S0349", "Read a returned virtual source URI through MCP."],
      ["agent-language-services/S0350", "Change an included source or manifest byte in a captured distribution."],
      ["agent-language-services/S0351", "Discover private, generated, test, descendant, symlink, non-regular, and `target` sources."],
      ["agent-language-services/S0352", "Load nonportable names or colliding paths."],
      ["agent-language-services/S0353", "Read a noncanonical, unknown, or mismatched snapshot URI through MCP."],
      ["agent-language-services/S0354", "Read a private distribution source or inspect package metadata."],
      ["agent-language-services/S0355", "Keep returned dependency URIs while projects refresh or disappear."],
      ["agent-language-services/S0356", "Generate package docs."],
      ["agent-language-services/S0357", "Change catalog semantics without changing package bytes."],
      ["agent-language-services/S0358", "Package documentation generation or doctest validation fails."],
      ["agent-language-services/S0360", "Generate the syntax reference."],
      ["agent-language-services/S0361", "Select a reference example."],
      ["agent-language-services/S0362", "Change a selected example or public table fragment."],
      ["agent-language-services/S0363", "Change only development documentation."],
      ["agent-language-services/S0364", "Reorder equivalent catalog input or vary catalog-owned Unicode and line endings."],
      ["agent-language-services/S0365", "Search a known language concept."],
      ["agent-language-services/S0366", "Read a documentation resource through both routes."],
      ["agent-language-services/S0367", "Inspect the published catalog."],
      ["agent-language-services/S0368", "Generate MCP and offline Markdown views."],
      ["agent-language-services/S0370", "Validate the Codex plugin."],
      ["agent-language-services/S0371", "Validate the Claude Code plugin."],
      ["agent-language-services/S0372", "Start with a missing, shadowed, or incompatible executable."],
      ["agent-language-services/S0373", "Use the shared skill."],
      ["agent-language-services/S0374", "Run the proposal completion gate."],
    ],
  },
};
const reviewedLifecycle = new Map([
  ["agent-language-services/S0004.c01", "planned"],
]);

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
    ...validateAgentLanguageServicesPlatformMatrix(proposal),
    ...validateUniverse({ parsed, universe }),
    ...validateInventory({ inventory, parsed, universe, manifest }),
    ...validateLedgerSchema(schema),
  ];
  const ledger = buildAcceptanceLedger({ repoRoot, inventory, manifest });
  errors.push(...validateMigrationLedger({ repoRoot, ledger, inventory, manifest, schema }).errors);
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
      lifecycle: reviewedLifecycle.get(`${root.id}.c${String(index + 1).padStart(2, "0")}`) ?? classifyLifecycle(child.text, root.heading),
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
  errors.push(...validateSourceUniverseContract({ parsed, universe }));
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

function validateSourceUniverseContract({ parsed, universe }) {
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
  const rootsById = new Map(roots.map((root) => [root.id, root]));
  const parsedById = new Map(parsed.map((root) => [root.id, root]));
  for (const [name, expectedPairs] of Object.entries(sourceUniverseContract.named_source_sets)) {
    const expected = new Map(expectedPairs);
    for (const [id, identity] of expected) {
      const root = rootsById.get(id);
      const parsedRoot = parsedById.get(id);
      if (!root || !parsedRoot) {
        errors.push(`${universePath}: source-universe contract missing ${name} source ${id} (${identity}).`);
        continue;
      }
      const actualIdentity = sourceSetIdentity(parsedRoot);
      if (actualIdentity !== identity) {
        errors.push(`${universePath}: source-universe contract ${name} source ${id} identity must be ${identity}.`);
      }
    }
    const matchingRoots = roots.filter((root) => expected.has(root.id));
    if (matchingRoots.length !== expected.size) {
      errors.push(`${universePath}: source-universe contract ${name} expects ${expected.size} sources, found ${matchingRoots.length}.`);
    }
    const duplicateIds = duplicates(matchingRoots.map((root) => root.id));
    for (const id of duplicateIds) {
      errors.push(`${universePath}: source-universe contract ${name} has duplicate source ${id}.`);
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

export function validateAgentLanguageServicesPlatformMatrix(markdown) {
  const errors = [];
  const rows = extractClientPlatformRows(markdown);
  if (rows.length !== closedClientPlatformMatrix.length) {
    errors.push(`agent-language-services platform matrix: expected ${closedClientPlatformMatrix.length} rows, found ${rows.length}; enumerate the exact closed client-platform cells before freezing the lifecycle source universe.`);
  }
  const seen = new Set();
  for (const [index, expected] of closedClientPlatformMatrix.entries()) {
    const row = rows[index];
    if (!row) continue;
    if (seen.has(row.key)) errors.push(`agent-language-services platform matrix: duplicate client-platform key ${row.key}.`);
    seen.add(row.key);
    if (row.key !== expected.key) errors.push(`agent-language-services platform matrix: row ${index + 1} must be ${expected.key}.`);
    if (row.client !== expected.client) errors.push(`agent-language-services platform matrix: ${row.key} must declare client ${expected.client}.`);
    if (row.platform !== expected.platform) errors.push(`agent-language-services platform matrix: ${row.key} must declare platform ${expected.platform}.`);
    for (const field of ["key", "client", "platform", "host_build", "manifest_schema", "validator", "validator_digest", "required_contracts"]) {
      if (typeof row[field] !== "string" || row[field].trim() === "") {
        errors.push(`agent-language-services platform matrix: ${row.key || `row ${index + 1}`} has empty ${field}.`);
      }
      if (typeof row[field] === "string" && /(?:\*|\.\.|^all\b|supported platforms?|future|tbd|placeholder|range)/i.test(row[field])) {
        errors.push(`agent-language-services platform matrix: ${row.key || `row ${index + 1}`} ${field} must be an exact literal.`);
      }
    }
    if (!/^[0-9a-f]{64}$/.test(row.validator_digest)) {
      errors.push(`agent-language-services platform matrix: ${row.key} validator_digest must be exactly 64 lowercase hexadecimal digits.`);
    }
  }
  const forbiddenReferences = [
    "per supported platform",
    "every supported client and platform",
    "supported client-platform cell",
  ];
  for (const phrase of forbiddenReferences) {
    if (markdown.toLowerCase().includes(phrase)) {
      errors.push(`agent-language-services platform matrix: replace "${phrase}" with a reference to the closed client-platform matrix rows.`);
    }
  }
  return errors;
}

export function validateMigrationLedger({ repoRoot = process.cwd(), ledger, inventory, manifest, schema = migrationLedgerSchema() }) {
  const errors = [];
  if (!ledger || ledger.schema_version !== 1) errors.push("migration ledger: schema_version must be 1.");
  errors.push(...validateMigrationLedgerJsonSchema({ ledger, schema }).errors);
  if (!Array.isArray(ledger?.entries)) return { valid: false, errors: [...errors, "migration ledger: entries must be an array."] };
  const leaves = new Map((manifest.leaves ?? []).map((leaf) => [leaf.source_id, leaf]));
  const parentIds = new Set((inventory.roots ?? []).filter((root) => root.child_count > 0).map((root) => root.id));
  const seen = new Set();
  const evidenceSeen = new Set();
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
    errors.push(...validateDestination({ repoRoot, entry, evidenceSeen }));
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
  const currentEvidence = collectCheckedEvidencePaths(repoRoot);
  let currentEvidenceIndex = 0;
  const rootsById = new Map((inventory.roots ?? []).map((root) => [root.id, root]));
  const destinationFor = (leaf) => {
    const lifecycle = leaf.lifecycle;
    if (lifecycle === "current") {
      const evidence = currentEvidence[currentEvidenceIndex];
      currentEvidenceIndex += 1;
      return {
        kind: "current",
        path: "docs/specification/mcp.md",
        anchor: "mcp-workspace-projects-diagnostics-and-definitions",
        evidence: [{
          path: evidence,
          case_id: evidenceCaseId(evidence),
        }],
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
      anchor: slug(rootsById.get(leaf.parent_id)?.heading ?? "acceptance-model"),
    };
  };
  return {
    schema_version: 1,
    entries: (manifest.leaves ?? []).map((leaf) => ({
      source_id: leaf.source_id,
      lifecycle: leaf.lifecycle,
      destination: destinationFor(leaf),
    })),
  };
}

function validateDestination({ repoRoot, entry, evidenceSeen = new Set() }) {
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
        if (!evidence || typeof evidence !== "object" || Array.isArray(evidence)) {
          errors.push(`migration ledger: ${entry.source_id} evidence must name a checked path and case_id.`);
          continue;
        }
        const evidenceKey = `${evidence.path}#${evidence.case_id}`;
        if (entryEvidenceSeen.has(evidenceKey)) errors.push(`migration ledger: duplicate checked evidence ${evidenceKey}.`);
        entryEvidenceSeen.add(evidenceKey);
        if (evidenceSeen.has(evidenceKey)) errors.push(`migration ledger: checked evidence ${evidenceKey} is reused by more than one current leaf.`);
        evidenceSeen.add(evidenceKey);
        if (typeof evidence.case_id !== "string" || evidence.case_id.trim() === "") {
          errors.push(`migration ledger: ${entry.source_id} evidence case_id must be nonempty.`);
        }
        if (typeof evidence.path !== "string" || !isCheckedEvidencePath(evidence.path) || !fs.existsSync(path.join(repoRoot, evidence.path))) {
          errors.push(`migration ledger: ${entry.source_id} evidence ${evidence.path} must resolve to an allowlisted checked route.`);
        }
      }
    }
  } else if (entry.lifecycle === "planned" && destination.anchor === "acceptance-model") {
    errors.push(`migration ledger: ${entry.source_id} planned destination must route to a concrete proposal heading, not the acceptance-model summary.`);
  }
  return errors;
}

function validateMarkdownDestination({ repoRoot, sourceId, lifecycle, pathValue, anchor }) {
  const errors = [];
  if (typeof pathValue !== "string" || pathValue.trim() === "") {
    return [`migration ledger: ${sourceId} destination path must be nonempty.`];
  }
  if (typeof anchor !== "string" || anchor.trim() === "") {
    return [`migration ledger: ${sourceId} destination anchor must be nonempty.`];
  }
  const fullPath = path.join(repoRoot, pathValue);
  if (!fs.existsSync(fullPath)) {
    return [`migration ledger: ${sourceId} destination path ${pathValue} does not exist.`];
  }
  if (!fs.statSync(fullPath).isFile()) {
    return [`migration ledger: ${sourceId} destination path ${pathValue} must be a Markdown file.`];
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
  if (schema?.$id !== "agent-language-services-migration-ledger.schema.json") {
    return { valid: false, errors: ["migration ledger schema: unsupported schema."] };
  }
  const errors = validateJsonSchemaValue({ schema, value: ledger, label: "migration ledger schema" });
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

function validateJsonSchemaValue({ schema, value, label }) {
  const errors = [];
  visit(schema, value, label);
  return errors;

  function visit(currentSchema, currentValue, currentLabel) {
    if (!currentSchema || typeof currentSchema !== "object") return;
    if (currentSchema.allOf) {
      for (const [index, childSchema] of currentSchema.allOf.entries()) {
        visit(childSchema, currentValue, `${currentLabel}.allOf[${index}]`);
      }
    }
    if (currentSchema.if) {
      const conditionErrors = [];
      visitWithErrors(currentSchema.if, currentValue, currentLabel, conditionErrors);
      if (conditionErrors.length === 0 && currentSchema.then) {
        visit(currentSchema.then, currentValue, currentLabel);
      }
    }
    if (currentSchema.not) {
      const conditionErrors = [];
      visitWithErrors(currentSchema.not, currentValue, currentLabel, conditionErrors);
      if (conditionErrors.length === 0) errors.push(`${currentLabel} must not match forbidden schema.`);
    }
    if (currentSchema.type && !matchesJsonType(currentValue, currentSchema.type)) {
      errors.push(`${currentLabel} must be ${currentSchema.type}.`);
      return;
    }
    if (Object.hasOwn(currentSchema, "const") && currentValue !== currentSchema.const) {
      errors.push(`${currentLabel} must equal ${JSON.stringify(currentSchema.const)}.`);
    }
    if (currentSchema.enum && !currentSchema.enum.includes(currentValue)) {
      errors.push(`${currentLabel} must be one of ${currentSchema.enum.join(", ")}.`);
    }
    if (typeof currentValue === "string") {
      if (currentSchema.minLength !== undefined && currentValue.length < currentSchema.minLength) {
        errors.push(`${currentLabel} must have length at least ${currentSchema.minLength}.`);
      }
      if (currentSchema.pattern && !new RegExp(currentSchema.pattern, "u").test(currentValue)) {
        errors.push(`${currentLabel} must match ${currentSchema.pattern}.`);
      }
    }
    if (Array.isArray(currentValue)) {
      if (currentSchema.minItems !== undefined && currentValue.length < currentSchema.minItems) {
        errors.push(`${currentLabel} must contain at least ${currentSchema.minItems} item.`);
      }
      if (currentSchema.uniqueItems && new Set(currentValue.map((item) => JSON.stringify(item))).size !== currentValue.length) {
        errors.push(`${currentLabel} must contain unique items.`);
      }
      if (currentSchema.items) {
        for (const [index, item] of currentValue.entries()) {
          visit(currentSchema.items, item, `${currentLabel}[${index}]`);
        }
      }
    }
    if (currentValue && typeof currentValue === "object" && !Array.isArray(currentValue)) {
      for (const key of currentSchema.required ?? []) {
        if (!Object.hasOwn(currentValue, key)) errors.push(`${currentLabel} missing ${key}.`);
      }
      if (currentSchema.additionalProperties === false && currentSchema.properties) {
        for (const key of Object.keys(currentValue)) {
          if (!Object.hasOwn(currentSchema.properties, key)) errors.push(`${currentLabel} has unexpected key ${key}.`);
        }
      }
      for (const [key, childSchema] of Object.entries(currentSchema.properties ?? {})) {
        if (Object.hasOwn(currentValue, key)) visit(childSchema, currentValue[key], `${currentLabel}.${key}`);
      }
    }
  }

  function visitWithErrors(currentSchema, currentValue, currentLabel, targetErrors) {
    const savedLength = errors.length;
    visit(currentSchema, currentValue, currentLabel);
    targetErrors.push(...errors.splice(savedLength));
  }
}

function matchesJsonType(value, type) {
  if (type === "array") return Array.isArray(value);
  if (type === "object") return value !== null && typeof value === "object" && !Array.isArray(value);
  if (type === "string") return typeof value === "string";
  if (type === "number") return typeof value === "number";
  if (type === "integer") return Number.isInteger(value);
  if (type === "boolean") return typeof value === "boolean";
  if (type === "null") return value === null;
  return true;
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
          allOf: [
            lifecycleDestinationSchema("current"),
            lifecycleDestinationSchema("completed"),
            lifecycleDestinationSchema("planned"),
            lifecycleDestinationSchema("removed"),
          ],
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
                path: { type: "string", minLength: 1, pattern: "\\.md$" },
                anchor: { type: "string", minLength: 1, enum: allowedLedgerDestinationAnchors() },
                evidence: {
                  type: "array",
                  minItems: 1,
                  uniqueItems: true,
                  items: {
                    type: "object",
                    additionalProperties: false,
                    required: ["path", "case_id"],
                    properties: {
                      path: { type: "string", minLength: 1 },
                      case_id: { type: "string", minLength: 1 },
                    },
                  },
                },
                rationale: { type: "string", minLength: 1 },
                supersedes: {
                  type: "object",
                  additionalProperties: false,
                  required: ["path", "anchor"],
                  properties: {
                    path: { type: "string", minLength: 1, pattern: "\\.md$" },
                    anchor: { type: "string", minLength: 1, enum: allowedLedgerDestinationAnchors() },
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

function lifecycleDestinationSchema(lifecycle) {
  return {
    if: {
      properties: {
        lifecycle: { const: lifecycle },
      },
      required: ["lifecycle"],
    },
    then: {
      properties: {
        destination: {
          properties: {
            kind: { const: lifecycle },
          },
          required: ["kind"],
        },
      },
      required: ["destination"],
    },
  };
}

function allowedLedgerDestinationAnchors() {
  return [
    "agent-language-services-frozen-inventory",
    "agent-module-package-and-documentation-model",
    "mcp-workspace-projects-diagnostics-and-definitions",
    ...Object.keys(sourceUniverseContract.heading_counts).map(slug),
  ].sort();
}

function extractClientPlatformRows(markdown) {
  const rows = [];
  const lines = markdown.split("\n");
  const headerIndex = lines.findIndex((line) => line.includes("| Client-platform key | Client | Platform | Host build |"));
  if (headerIndex < 0) return rows;
  for (let index = headerIndex + 2; index < lines.length && isTableRow(lines[index]); index += 1) {
    const cells = splitMarkdownTableCells(lines[index]);
    if (cells.length !== 8) continue;
    rows.push({
      key: unwrapCode(cells[0]),
      client: unwrapCode(cells[1]),
      platform: unwrapCode(cells[2]),
      host_build: unwrapCode(cells[3]),
      manifest_schema: unwrapCode(cells[4]),
      validator: unwrapCode(cells[5]),
      validator_digest: unwrapCode(cells[6]),
      required_contracts: cells[7],
    });
  }
  return rows;
}

function splitMarkdownTableCells(line) {
  let text = line.trim();
  if (text.startsWith("|")) text = text.slice(1);
  if (text.endsWith("|")) text = text.slice(0, -1);
  return text.split("|").map((cell) => cell.trim());
}

function unwrapCode(text) {
  return text.replace(/^`|`$/g, "");
}

function sourceSetIdentity(root) {
  if (root.kind === "table-row") return splitMarkdownTableCells(root.text)[0];
  if (root.kind === "list-item") {
    return root.text.replace(/^\s*[-*+]\s+/, "").replace(/[.;]\s*(?:and)?\s*$/u, "").trim();
  }
  return root.text.trim();
}

function duplicates(values) {
  const seen = new Set();
  const result = new Set();
  for (const value of values) {
    if (seen.has(value)) result.add(value);
    seen.add(value);
  }
  return [...result];
}

function collectCheckedEvidencePaths(repoRoot) {
  const roots = ["examples/specification"];
  const result = [];
  for (const root of roots) {
    const fullRoot = path.join(repoRoot, root);
    if (!fs.existsSync(fullRoot)) continue;
    walk(fullRoot);
  }
  return result.sort();

  function walk(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const fullPath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
      } else if (/\.(?:toml|veln|json|jsonl|txt|raw|pl)$/.test(entry.name)) {
        result.push(path.relative(repoRoot, fullPath).replaceAll(path.sep, "/"));
      }
    }
  }
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

function evidenceCaseId(file) {
  const parts = file.split("/");
  const specificationIndex = parts.indexOf("specification");
  if (specificationIndex >= 0 && parts[specificationIndex + 1] && parts[specificationIndex + 2]) {
    return `${parts[specificationIndex + 1]}/${parts[specificationIndex + 2]}`;
  }
  return file.replace(/\.[^.]+$/u, "");
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
