import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  markdownFrontmatter,
} from "./check-doc-frontmatter.mjs";

export const lifecycleDir = "docs/reference/agent-language-services-lifecycle";
export const reviewDir = "docs/reference/agent-language-services-lifecycle-review";
export const sourcePath = "docs/proposals/agent-language-services.md";

const universePath = `${lifecycleDir}/source-universe.json`;
const inventoryPath = `${lifecycleDir}/frozen-inventory.json`;
const manifestPath = `${lifecycleDir}/lifecycle-manifest.json`;
const ledgerSchemaPath = `${lifecycleDir}/migration-ledger.schema.json`;
const ledgerFixturePath = `${lifecycleDir}/migration-ledger.schema-fixture.json`;
const provenancePath = `${lifecycleDir}/target-provenance.json`;
const sourceDecisionsPath = `${reviewDir}/source-decisions.json`;

const lifecycleValues = new Set(["current", "completed", "planned", "removed"]);
const protectedAfterBootstrap = new Set([
  universePath,
  inventoryPath,
  manifestPath,
  ledgerSchemaPath,
  ledgerFixturePath,
  provenancePath,
  sourceDecisionsPath,
  "workflow-scripts/check-agent-language-services-lifecycle.mjs",
  "workflow-scripts/check-agent-language-services-lifecycle.test.mjs",
]);

if (isMainModule()) {
  const command = process.argv[2] ?? "validate";
  const repoRoot = process.cwd();
  let result;
  if (command === "generate") {
    writeGeneratedArtifacts(repoRoot);
    result = success("Regenerated the agent-language-services lifecycle artifacts.");
  } else if (command === "validate") {
    result = validateRepository({ repoRoot });
  } else if (command === "diff-scope") {
    result = validateDiffScope({ repoRoot, paths: process.argv.slice(3) });
  } else {
    result = failure("Use generate, validate, or diff-scope.", [`unknown command: ${command}`]);
  }
  if (!result.valid) {
    console.error(result.summary);
    for (const error of result.errors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }
  console.log(result.summary);
}

export function writeGeneratedArtifacts(repoRoot) {
  fs.mkdirSync(path.resolve(repoRoot, lifecycleDir), { recursive: true });
  fs.mkdirSync(path.resolve(repoRoot, reviewDir), { recursive: true });
  const generated = generateArtifacts({ repoRoot });
  writeJson(repoRoot, universePath, generated.universe);
  writeJson(repoRoot, inventoryPath, generated.inventory);
  writeJson(repoRoot, manifestPath, generated.manifest);
  writeJson(repoRoot, ledgerSchemaPath, generated.ledgerSchema);
  writeJson(repoRoot, ledgerFixturePath, generated.ledgerFixture);
  writeJson(repoRoot, provenancePath, generated.provenance);
  writeJson(repoRoot, sourceDecisionsPath, generated.sourceDecisions);
}

export function generateArtifacts({ repoRoot }) {
  const source = fs.readFileSync(path.resolve(repoRoot, sourcePath), "utf8");
  const roots = parseMarkdownSource(source);
  const universeRoots = roots.map((root) => ({
    id: root.id,
    kind: root.kind,
    heading: root.heading,
    span: root.span,
    digest: sha256(root.text),
    conformance: true,
    text: root.text,
  }));
  const inventoryRoots = roots.map((root) => inventoryRoot(root));
  const leaves = inventoryRoots.flatMap((root) => root.children?.length > 0
    ? root.children.map((child) => ({ ...child, root_id: root.id, conformance: true }))
    : [{ id: root.id, root_id: root.id, lifecycle: root.lifecycle, spans: [root.span], conformance: true }]);
  const identities = finiteIdentities(roots);
  return {
    universe: {
      schema_version: 1,
      source_path: sourcePath,
      source_digest: sha256(source),
      roots: universeRoots,
      identities,
    },
    inventory: {
      schema_version: 1,
      source_path: sourcePath,
      roots: inventoryRoots,
    },
    manifest: {
      schema_version: 1,
      leaves: leaves.map((leaf) => ({
        id: leaf.id,
        root_id: leaf.root_id,
        lifecycle: leaf.lifecycle,
        conformance: leaf.conformance,
      })),
    },
    ledgerSchema: migrationLedgerSchema(),
    ledgerFixture: {
      schema_version: 1,
      entries: leaves.map((leaf) => ledgerEntryForLeaf(leaf)),
    },
    provenance: targetProvenance(repoRoot),
    sourceDecisions: {
      schema_version: 1,
      source_path: sourcePath,
      source_digest: sha256(source),
      roots: universeRoots.map((root) => ({
        id: root.id,
        digest: root.digest,
        conformance: root.conformance,
      })),
      leaves: leaves.map((leaf) => ({
        id: leaf.id,
        root_id: leaf.root_id,
        lifecycle: leaf.lifecycle,
        spans: leaf.spans,
      })),
      identities,
    },
  };
}

export function validateRepository({ repoRoot }) {
  const artifacts = readArtifacts(repoRoot);
  return validateArtifacts({ repoRoot, ...artifacts });
}

export function validateArtifacts({
  repoRoot,
  universe,
  inventory,
  manifest,
  ledgerFixture,
  sourceDecisions,
}) {
  const errors = [];
  const source = fs.readFileSync(path.resolve(repoRoot, sourcePath), "utf8");
  const parsedRoots = parseMarkdownSource(source);
  const parsedById = new Map(parsedRoots.map((root) => [root.id, root]));
  const universeById = uniqueById(universe.roots, "source-universe roots", errors);
  const inventoryById = uniqueById(inventory.roots, "frozen inventory roots", errors);
  const sourceDecisionLeaves = uniqueById(sourceDecisions.leaves, "reviewed source-decision leaves", errors);

  if (universe.source_path !== sourcePath || inventory.source_path !== sourcePath || sourceDecisions.source_path !== sourcePath) {
    errors.push(`${lifecycleDir}: keep lifecycle artifacts bound to ${sourcePath}`);
  }
  if (universe.source_digest !== sha256(source) || sourceDecisions.source_digest !== sha256(source)) {
    errors.push(`${sourcePath}: restore the frozen umbrella proposal text or regenerate the reviewed source digest before relying on the inventory.`);
  }

  compareSourceRoots({ parsedRoots, parsedById, universeById, inventoryById, errors });
  const inventoryLeaves = validateInventory({ inventory, parsedById, sourceDecisionLeaves, errors });
  validateManifest({ manifest, inventoryLeaves, errors });
  validateSourceDecisions({ sourceDecisions, universe, inventoryLeaves, errors });
  validateIdentities({ source, universe, sourceDecisions, errors });
  validateLedger({ ledger: ledgerFixture, inventory, manifest, errors });

  return errors.length === 0
    ? success("Agent language-services lifecycle artifacts match the frozen proposal source and ledger schema contract.")
    : failure("Update the agent language-services lifecycle artifacts before merging; the frozen source universe and ledger schema must stay reviewable.", errors);
}

export function validateLedger({ ledger, inventory, manifest, errors = [] }) {
  const leafIds = inventoryLeafIds(inventory);
  const parentIds = new Set(inventory.roots.filter((root) => root.children?.length > 0).map((root) => root.id));
  const manifestById = new Map(manifest.leaves.map((leaf) => [leaf.id, leaf]));
  const seen = new Set();
  if (!Array.isArray(ledger.entries)) {
    errors.push(`${ledgerFixturePath}: add an entries array.`);
    return errors;
  }
  for (const [index, entry] of ledger.entries.entries()) {
    const label = `${ledgerFixturePath}: entries[${index}]`;
    const leafId = entry.leaf_id;
    if (!isStableLeafId(leafId)) {
      errors.push(`${label}: enumerate one stable leaf_id; ranges, wildcards, and catch-all entries hide migration review gaps.`);
      continue;
    }
    if (parentIds.has(leafId)) {
      errors.push(`${label}: map the contiguous children of ${leafId}, not the parent source item.`);
    }
    if (!leafIds.has(leafId)) {
      errors.push(`${label}: use a leaf from the frozen inventory.`);
    }
    if (seen.has(leafId)) {
      errors.push(`${label}: remove the duplicate mapping for ${leafId}.`);
    }
    seen.add(leafId);
    if (!lifecycleValues.has(entry.lifecycle)) {
      errors.push(`${label}: set lifecycle to current, completed, planned, or removed.`);
    } else if (manifestById.get(leafId)?.lifecycle !== entry.lifecycle) {
      errors.push(`${label}: keep the ledger lifecycle equal to the frozen lifecycle manifest.`);
    }
    if (entry.lifecycle === "removed" && manifestById.get(leafId)?.conformance !== false) {
      errors.push(`${label}: do not remove a leaf from the frozen conformance universe.`);
    }
    validateDestination(label, entry, errors);
  }
  for (const leafId of leafIds) {
    if (!seen.has(leafId)) {
      errors.push(`${ledgerFixturePath}: add exactly one ledger mapping for ${leafId}.`);
    }
  }
  return errors;
}

export function validateDiffScope({ repoRoot, paths }) {
  const changedPaths = paths.length > 0 ? paths : changedFiles(repoRoot);
  const errors = [];
  const frozenTouched = changedPaths.some((changedPath) => changedPath.startsWith(`${lifecycleDir}/`));
  if (!frozenTouched) {
    return success("No frozen lifecycle artifact changes require bootstrap diff-scope validation.");
  }
  for (const changedPath of changedPaths) {
    if (isBootstrapAllowedPath(changedPath)) {
      continue;
    }
    errors.push(`${changedPath}: move this change out of the frozen inventory bootstrap PR; mixing unrelated paths can invalidate the reviewed source universe or toolchain baselines.`);
  }
  return errors.length === 0
    ? success("Frozen inventory bootstrap diff scope contains only lifecycle artifact, validator, and workflow-registration paths.")
    : failure("Restore out-of-scope files or split the change before merging the frozen inventory bootstrap.", errors);
}

export function parseMarkdownSource(text) {
  const bodyStart = frontmatterBodyStart(text);
  const body = text.slice(bodyStart);
  const lines = splitLinesWithOffsets(body, bodyStart);
  const roots = [];
  let heading = "(document)";
  let inFence = false;
  let paragraph = [];
  let listItem = [];
  let rootIndex = 1;

  const flushParagraph = () => {
    if (paragraph.length === 0) return;
    roots.push(rootFromLines(rootIndex++, "paragraph", heading, paragraph));
    paragraph = [];
  };
  const flushListItem = () => {
    if (listItem.length === 0) return;
    roots.push(rootFromLines(rootIndex++, "list-item", heading, listItem));
    listItem = [];
  };

  for (const line of lines) {
    const trimmed = line.text.trim();
    if (/^```/.test(trimmed)) {
      flushParagraph();
      flushListItem();
      inFence = !inFence;
      continue;
    }
    if (inFence) {
      if (trimmed !== "") {
        roots.push(rootFromLines(rootIndex++, "fenced-line", heading, [line]));
      }
      continue;
    }
    const headingMatch = /^(#{1,6})\s+(.+?)\s*$/.exec(line.text);
    if (headingMatch) {
      flushParagraph();
      flushListItem();
      heading = headingMatch[2].replace(/\s+#+$/, "");
      continue;
    }
    if (trimmed === "") {
      flushParagraph();
      flushListItem();
      continue;
    }
    if (isTableRow(line.text)) {
      flushParagraph();
      flushListItem();
      if (!isTableDelimiter(line.text)) {
        roots.push(rootFromLines(rootIndex++, "table-row", heading, [line]));
      }
      continue;
    }
    if (/^\s*[-*]\s+/.test(line.text) || /^\s*\d+\.\s+/.test(line.text)) {
      flushParagraph();
      flushListItem();
      listItem = [line];
      continue;
    }
    if (listItem.length > 0 && /^\s+/.test(line.text)) {
      listItem.push(line);
      continue;
    }
    flushListItem();
    paragraph.push(line);
  }
  flushParagraph();
  flushListItem();
  return roots;
}

function compareSourceRoots({ parsedRoots, parsedById, universeById, inventoryById, errors }) {
  if (universeById.size !== parsedRoots.length) {
    errors.push(`${universePath}: record every parsed source root exactly once.`);
  }
  for (const parsed of parsedRoots) {
    const universeRoot = universeById.get(parsed.id);
    if (universeRoot === undefined) {
      errors.push(`${universePath}: add missing source root ${parsed.id}.`);
      continue;
    }
    if (universeRoot.digest !== sha256(parsed.text)) {
      errors.push(`${universePath}: update ${parsed.id} digest after reviewing the exact source text.`);
    }
    if (JSON.stringify(universeRoot.span) !== JSON.stringify(parsed.span)) {
      errors.push(`${universePath}: keep ${parsed.id} Unicode-scalar span aligned with the umbrella source.`);
    }
    if (inventoryById.get(parsed.id) === undefined) {
      errors.push(`${inventoryPath}: add missing source inventory item ${parsed.id}.`);
    }
  }
  for (const id of universeById.keys()) {
    if (!parsedById.has(id)) {
      errors.push(`${universePath}: remove unexpected source root ${id}.`);
    }
  }
}

function validateInventory({ inventory, parsedById, sourceDecisionLeaves, errors }) {
  const leaves = new Map();
  for (const root of inventory.roots) {
    const parsed = parsedById.get(root.id);
    if (parsed === undefined) {
      errors.push(`${inventoryPath}: remove unexpected source inventory item ${root.id}.`);
      continue;
    }
    if (root.digest !== sha256(parsed.text)) {
      errors.push(`${inventoryPath}: update ${root.id} digest after reviewing the exact source text.`);
    }
    if (root.children?.length > 0) {
      if (root.child_count !== root.children.length) {
        errors.push(`${inventoryPath}: ${root.id} child_count must equal its child array length.`);
      }
      validateChildren({ root, parsed, sourceDecisionLeaves, leaves, errors });
    } else {
      validateLeafLifecycle({ label: root.id, lifecycle: root.lifecycle, text: parsed.text, errors });
      leaves.set(root.id, { id: root.id, root_id: root.id, lifecycle: root.lifecycle, conformance: true });
    }
  }
  return leaves;
}

function validateChildren({ root, parsed, sourceDecisionLeaves, leaves, errors }) {
  const expectedIds = Array.from({ length: root.child_count }, (_, index) => `${root.id}.${index + 1}`);
  const seenScalars = new Set();
  for (const [index, child] of root.children.entries()) {
    const expectedId = expectedIds[index];
    if (child.id !== expectedId) {
      errors.push(`${inventoryPath}: use contiguous child ID ${expectedId} for ${root.id}.`);
    }
    if (!Array.isArray(child.spans) || child.spans.length === 0) {
      errors.push(`${inventoryPath}: ${child.id} needs at least one Unicode-scalar span.`);
      continue;
    }
    const text = child.spans.map((span) => sliceScalars(parsed.text, span[0] - parsed.span[0], span[1] - parsed.span[0])).join("");
    validateLeafLifecycle({ label: child.id, lifecycle: child.lifecycle, text, errors });
    for (const span of child.spans) {
      if (span[0] < parsed.span[0] || span[1] > parsed.span[1] || span[0] >= span[1]) {
        errors.push(`${inventoryPath}: ${child.id} span must stay inside ${root.id}.`);
        continue;
      }
      for (let scalar = span[0]; scalar < span[1]; scalar += 1) {
        if (seenScalars.has(scalar)) {
          errors.push(`${inventoryPath}: ${child.id} overlaps another child span at scalar ${scalar}.`);
        }
        seenScalars.add(scalar);
      }
    }
    leaves.set(child.id, { id: child.id, root_id: root.id, lifecycle: child.lifecycle, conformance: true });
  }
  for (let scalar = parsed.span[0]; scalar < parsed.span[1]; scalar += 1) {
    const value = sliceScalars(parsed.text, scalar - parsed.span[0], scalar - parsed.span[0] + 1);
    if (!seenScalars.has(scalar) && !isSeparatorScalar(value)) {
      errors.push(`${inventoryPath}: ${root.id} leaves scalar ${scalar} uncovered by a child span.`);
      break;
    }
  }
  for (const leafId of expectedIds) {
    if (!sourceDecisionLeaves.has(leafId)) {
      errors.push(`${sourceDecisionsPath}: add reviewed lifecycle decision for ${leafId}.`);
    }
  }
}

function validateLeafLifecycle({ label, lifecycle, text, errors }) {
  if (!lifecycleValues.has(lifecycle)) {
    errors.push(`${inventoryPath}: ${label} must use one lifecycle class.`);
    return;
  }
  const inferred = inferredLifecycles(text);
  if (inferred.size > 1) {
    errors.push(`${inventoryPath}: ${label} mixes lifecycle statements; split it into lifecycle-homogeneous children.`);
  }
}

function validateManifest({ manifest, inventoryLeaves, errors }) {
  const manifestLeaves = uniqueById(manifest.leaves, "lifecycle manifest leaves", errors);
  if (manifestLeaves.size !== inventoryLeaves.size) {
    errors.push(`${manifestPath}: map every inventory leaf exactly once.`);
  }
  for (const [leafId, leaf] of inventoryLeaves) {
    const manifestLeaf = manifestLeaves.get(leafId);
    if (manifestLeaf === undefined) {
      errors.push(`${manifestPath}: add missing leaf ${leafId}.`);
      continue;
    }
    if (manifestLeaf.lifecycle !== leaf.lifecycle) {
      errors.push(`${manifestPath}: keep ${leafId} lifecycle equal to the reviewed inventory leaf.`);
    }
    if (manifestLeaf.lifecycle === "removed" && manifestLeaf.conformance !== false) {
      errors.push(`${manifestPath}: ${leafId} is conformance content and cannot be removed.`);
    }
  }
}

function validateSourceDecisions({ sourceDecisions, universe, inventoryLeaves, errors }) {
  const decisionRoots = uniqueById(sourceDecisions.roots, "reviewed source-decision roots", errors);
  for (const root of universe.roots) {
    const decision = decisionRoots.get(root.id);
    if (decision === undefined || decision.digest !== root.digest || decision.conformance !== root.conformance) {
      errors.push(`${sourceDecisionsPath}: keep reviewed root ${root.id} equal to the frozen source-universe contract.`);
    }
  }
  const decisionLeaves = uniqueById(sourceDecisions.leaves, "reviewed source-decision leaves", errors);
  for (const [leafId, leaf] of inventoryLeaves) {
    const decision = decisionLeaves.get(leafId);
    if (decision === undefined) {
      errors.push(`${sourceDecisionsPath}: add reviewed lifecycle decision for ${leafId}.`);
    } else if (decision.lifecycle !== leaf.lifecycle) {
      errors.push(`${sourceDecisionsPath}: keep ${leafId} lifecycle equal to the frozen inventory.`);
    }
  }
}

function validateIdentities({ source, universe, sourceDecisions, errors }) {
  const requiredQ = new Set(Array.from({ length: 22 }, (_, index) => `Q${String(index + 1).padStart(2, "0")}`));
  const identityKeys = new Set();
  for (const identity of universe.identities ?? []) {
    identityKeys.add(`${identity.kind}:${identity.name}:${identity.root_id}:${identity.span?.join("-")}`);
    const text = sliceScalars(source, identity.span?.[0] ?? 0, identity.span?.[1] ?? 0);
    if (text !== identity.name) {
      errors.push(`${universePath}: bind identity ${identity.name} to its exact source occurrence.`);
    }
    requiredQ.delete(identity.name);
  }
  for (const identity of sourceDecisions.identities ?? []) {
    const key = `${identity.kind}:${identity.name}:${identity.root_id}:${identity.span?.join("-")}`;
    if (!identityKeys.has(key)) {
      errors.push(`${sourceDecisionsPath}: keep identity ${identity.name} source-bound and equal to the source universe.`);
    }
  }
  for (const missing of requiredQ) {
    errors.push(`${universePath}: preserve the named finite evidence identity ${missing}.`);
  }
}

function inventoryRoot(root) {
  const segments = lifecycleSegments(root);
  if (segments.length > 1) {
    return {
      id: root.id,
      kind: root.kind,
      heading: root.heading,
      span: root.span,
      digest: sha256(root.text),
      child_count: segments.length,
      children: segments.map((segment, index) => ({
        id: `${root.id}.${index + 1}`,
        lifecycle: segment.lifecycle,
        spans: [[root.span[0] + segment.start, root.span[0] + segment.end]],
        digest: sha256(segment.text),
      })),
    };
  }
  return {
    id: root.id,
    kind: root.kind,
    heading: root.heading,
    span: root.span,
    digest: sha256(root.text),
    lifecycle: segments[0]?.lifecycle ?? "planned",
  };
}

function lifecycleSegments(root) {
  const scalars = Array.from(root.text);
  const sentenceRanges = [];
  let start = 0;
  for (let index = 0; index < scalars.length; index += 1) {
    if (/[.!?。]/u.test(scalars[index])) {
      sentenceRanges.push([start, index + 1]);
      start = index + 1;
    }
  }
  if (start < scalars.length) {
    sentenceRanges.push([start, scalars.length]);
  }
  const segments = sentenceRanges
    .map(([segmentStart, segmentEnd]) => {
      const text = scalars.slice(segmentStart, segmentEnd).join("");
      return { start: segmentStart, end: segmentEnd, text, lifecycle: primaryLifecycle(text) };
    })
    .filter((segment) => segment.text.trim() !== "");
  const lifecycles = new Set(segments.map((segment) => segment.lifecycle));
  return lifecycles.size > 1 ? segments : [{
    start: 0,
    end: scalars.length,
    text: root.text,
    lifecycle: primaryLifecycle(root.text),
  }];
}

function primaryLifecycle(text) {
  const lower = text.toLowerCase();
  if (/\bplanned\b|\bremain(?:s|ing)?\b|\bfuture\b|\bdeferred\b|\bnot selectable\b|\bnext\b|\bsubsequent\b|\bout of scope\b/.test(lower)) return "planned";
  if (/\bcompleted\b|\bcomplete\b|\bclosed\b|\bimplemented history\b/.test(lower)) return "completed";
  if (/\bimplemented\b|\bcurrently\b|\bcurrent\b|\bspecified\b|\bexposes\b/.test(lower)) return "current";
  return "planned";
}

function frontmatterBodyStart(text) {
  const frontmatter = markdownFrontmatter(text);
  if (frontmatter === undefined || !frontmatter.closed) {
    return 0;
  }
  const prefix = text.split("\n").slice(0, frontmatter.endLine).join("\n");
  return Array.from(`${prefix}\n`).length;
}

function inferredLifecycles(text) {
  return new Set(lifecycleSegments({ text, span: [0, Array.from(text).length] }).map((segment) => segment.lifecycle));
}

function finiteIdentities(roots) {
  const identities = [];
  const patterns = [
    ["evidence-gate", /\bQ\d{2}\b/g],
    ["tool", /`(?:check_project|definition|references|search_docs|read_doc|workspace_projects|refresh_workspace)`/g],
    ["domain-error", /`(?:invalid_path|invalid_position|invalid_query|source_required|project_not_selected|project_ambiguous|snapshot_changed|invalid_cursor|stale_snapshot|resource_not_found|generation_failed|resource_capacity|incompatible_version)`/g],
    ["encoding", /UTF-(?:8|16|32)/g],
    ["client-platform", /`(?:codex|claude-code)\/x86_64-unknown-linux-gnu`/g],
  ];
  for (const root of roots) {
    for (const [kind, pattern] of patterns) {
      for (const match of root.text.matchAll(pattern)) {
        const raw = match[0];
        const backtickOffset = raw.startsWith("`") ? 1 : 0;
        const name = raw.replaceAll("`", "");
        const start = root.span[0] + Array.from(root.text.slice(0, match.index + backtickOffset)).length;
        identities.push({
          kind,
          name,
          root_id: root.id,
          span: [start, start + Array.from(name).length],
        });
      }
    }
  }
  return identities;
}

function ledgerEntryForLeaf(leaf) {
  const destination = leaf.lifecycle === "current"
    ? { kind: "specification", path: "docs/specification/mcp.md", anchor: "#mcp-workspace-projects-diagnostics-and-definitions", evidence: ["examples/specification/mcp/workspace-lifecycle/case.toml"] }
    : leaf.lifecycle === "completed"
      ? { kind: "implementation-record", path: "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md", anchor: "#agent-language-services-inventory-review-gate", evidence: ["docs/reference/agent-language-services-lifecycle/source-universe.json"] }
      : { kind: "proposal", path: "docs/proposals/agent-language-services.md", anchor: "#agent-language-services", evidence: ["docs/proposals/agent-language-services.md"] };
  return { leaf_id: leaf.id, lifecycle: leaf.lifecycle, destination };
}

function validateDestination(label, entry, errors) {
  const destination = entry.destination;
  if (!destination || typeof destination !== "object") {
    errors.push(`${label}: add a destination object.`);
    return;
  }
  const expectedKind = entry.lifecycle === "current"
    ? "specification"
    : entry.lifecycle === "completed"
      ? "implementation-record"
      : entry.lifecycle === "planned"
        ? "proposal"
        : "removed";
  if (destination.kind !== expectedKind) {
    errors.push(`${label}: route ${entry.lifecycle} leaves to a ${expectedKind} destination.`);
  }
  if (entry.lifecycle !== "removed") {
    if (!isRepoMarkdownPath(destination.path)) {
      errors.push(`${label}: use a repository-relative Markdown destination path.`);
    }
    if (!/^#[a-z0-9][a-z0-9-]*$/.test(destination.anchor ?? "")) {
      errors.push(`${label}: use a concrete Markdown heading anchor.`);
    }
    if (!Array.isArray(destination.evidence) || destination.evidence.length === 0) {
      errors.push(`${label}: list checked evidence for the destination.`);
    }
  } else if (typeof destination.rationale !== "string" || destination.rationale.trim() === "") {
    errors.push(`${label}: removed supporting leaves need a rationale.`);
  }
}

function migrationLedgerSchema() {
  return {
    $id: "https://veln-lang.invalid/schemas/agent-language-services-migration-ledger.schema.json",
    schema_version: 1,
    description: "Structural contract for the later migration ledger. The semantic validator resolves leaf IDs, lifecycle equality, and destination roles.",
    required: ["schema_version", "entries"],
    leaf_id_pattern: "^ALS-S[0-9]{4}(\\.[0-9]+)?$",
    forbidden_leaf_id_patterns: ["\\*", "\\.\\.", "^all$", "^remaining$"],
    lifecycle_enum: ["current", "completed", "planned", "removed"],
    destination_kinds_by_lifecycle: {
      current: "specification",
      completed: "implementation-record",
      planned: "proposal",
      removed: "removed",
    },
  };
}

function targetProvenance(repoRoot) {
  const head = gitOutput(repoRoot, ["rev-parse", "HEAD"]) ?? "0".repeat(40);
  return {
    schema_version: 1,
    proposal_path: "docs/proposals/agent-language-services-lifecycle-migration.md",
    proposal_anchor: "#frozen-source-universe",
    target_kind: "proposal-section",
    default_branch: "main",
    base_commit: head.trim(),
    prerequisites: [
      "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md",
      "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md",
      "docs/reference/implemented-proposals/checked-proposal-target-readiness.md",
    ],
    frozen_artifact_set: [
      universePath,
      inventoryPath,
      manifestPath,
      ledgerSchemaPath,
      ledgerFixturePath,
    ],
  };
}

function rootFromLines(index, kind, heading, lines) {
  const text = lines.map((line) => line.text).join("");
  const start = lines[0].scalarStart;
  return {
    id: `ALS-S${String(index).padStart(4, "0")}`,
    kind,
    heading,
    span: [start, start + Array.from(text).length],
    text,
  };
}

function splitLinesWithOffsets(text, scalarBase) {
  const lines = [];
  let byteOffset = 0;
  let scalarOffset = scalarBase;
  for (const part of text.matchAll(/.*(?:\n|$)/g)) {
    const line = part[0];
    if (line === "") continue;
    lines.push({ text: line, byteStart: byteOffset, scalarStart: scalarOffset });
    byteOffset += Buffer.byteLength(line);
    scalarOffset += Array.from(line).length;
  }
  return lines;
}

function isTableRow(text) {
  return /^\s*\|.*\|\s*$/.test(text);
}

function isTableDelimiter(text) {
  return /^\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)+\|?\s*$/.test(text.trim());
}

function isSeparatorScalar(value) {
  return /\s/.test(value) || value === "|" || value === "-";
}

function sliceScalars(text, start, end) {
  return Array.from(text).slice(start, end).join("");
}

function inventoryLeafIds(inventory) {
  return new Set(inventory.roots.flatMap((root) => root.children?.length > 0
    ? root.children.map((child) => child.id)
    : [root.id]));
}

function isStableLeafId(value) {
  return /^ALS-S\d{4}(?:\.\d+)?$/.test(value ?? "");
}

function isRepoMarkdownPath(value) {
  return typeof value === "string" && /^docs\/(?:proposals|specification|reference)\/.+\.md$/.test(value);
}

function isBootstrapAllowedPath(value) {
  return value === ".github/workflows/workflow--test-scripts.yaml"
    || value === "docs/reference/README.md"
    || value.startsWith(`${lifecycleDir}/`)
    || value.startsWith(`${reviewDir}/`)
    || value === "docs/reference/implemented-proposals/README.md"
    || value === "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md"
    || value === "docs/proposals/README.md"
    || value === "docs/proposals/agent-language-services-lifecycle-migration.md"
    || value === "docs/reference/proposal-target-readiness/manifest.json"
    || value === "workflow-scripts/check-agent-language-services-lifecycle.mjs"
    || value === "workflow-scripts/check-agent-language-services-lifecycle.test.mjs";
}

function changedFiles(repoRoot) {
  const base = process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA;
  const head = process.env.AGENT_LANGUAGE_SERVICES_HEAD_SHA;
  if (base && head && !/^0+$/.test(base)) {
    const result = spawnSync("git", ["diff", "--name-only", "--diff-filter=ACMRTD", base, head], { cwd: repoRoot, encoding: "utf8" });
    if (result.status === 0) {
      return result.stdout.split(/\r?\n/).filter(Boolean);
    }
  }
  return [];
}

function readArtifacts(repoRoot) {
  return {
    universe: readJson(repoRoot, universePath),
    inventory: readJson(repoRoot, inventoryPath),
    manifest: readJson(repoRoot, manifestPath),
    ledgerSchema: readJson(repoRoot, ledgerSchemaPath),
    ledgerFixture: readJson(repoRoot, ledgerFixturePath),
    provenance: readJson(repoRoot, provenancePath),
    sourceDecisions: readJson(repoRoot, sourceDecisionsPath),
  };
}

function uniqueById(items, label, errors) {
  const seen = new Map();
  if (!Array.isArray(items)) {
    errors.push(`${label}: use an array.`);
    return seen;
  }
  for (const item of items) {
    if (!item?.id) {
      errors.push(`${label}: every item needs an id.`);
      continue;
    }
    if (seen.has(item.id)) {
      errors.push(`${label}: remove duplicate item ${item.id}.`);
    }
    seen.set(item.id, item);
  }
  return seen;
}

function readJson(repoRoot, relativePath) {
  return JSON.parse(fs.readFileSync(path.resolve(repoRoot, relativePath), "utf8"));
}

function writeJson(repoRoot, relativePath, value) {
  fs.writeFileSync(path.resolve(repoRoot, relativePath), `${JSON.stringify(value, null, 2)}\n`);
}

function sha256(text) {
  return crypto.createHash("sha256").update(text, "utf8").digest("hex");
}

function gitOutput(repoRoot, args) {
  const result = spawnSync("git", args, { cwd: repoRoot, encoding: "utf8" });
  return result.status === 0 ? result.stdout : undefined;
}

function success(summary) {
  return { valid: true, summary, errors: [] };
}

function failure(summary, errors) {
  return { valid: false, summary, errors };
}

function isMainModule() {
  return process.argv[1] === new URL(import.meta.url).pathname;
}
