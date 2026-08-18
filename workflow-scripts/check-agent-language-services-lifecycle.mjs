import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  frontmatterField,
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
const lifecycleProposalPath = "docs/proposals/agent-language-services-lifecycle-migration.md";
const lifecycleProposalAnchor = "#frozen-source-universe";

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
  const generated = generateArtifacts({ repoRoot });
  writeJson(repoRoot, universePath, generated.universe);
  writeJson(repoRoot, inventoryPath, generated.inventory);
  writeJson(repoRoot, manifestPath, generated.manifest);
  writeJson(repoRoot, ledgerSchemaPath, generated.ledgerSchema);
  writeJson(repoRoot, ledgerFixturePath, generated.ledgerFixture);
  writeJson(repoRoot, provenancePath, generated.provenance);
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
    : [{ id: root.id, root_id: root.id, heading: root.heading, lifecycle: root.lifecycle, spans: [root.span], digest: root.digest, conformance: true }]);
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
      leaves: leaves.map((leaf) => reviewedLeafDecision(leaf)),
      identities,
    },
  };
}

export function validateRepository({ repoRoot }) {
  const artifacts = readArtifacts(repoRoot);
  const baseAuthorityErrors = [];
  const baseAuthority = reviewedSourceDecisionsForValidation({
    repoRoot,
    provenance: artifacts.provenance,
    errors: baseAuthorityErrors,
  });
  const result = validateArtifacts({
    repoRoot,
    ...artifacts,
    sourceDecisions: baseAuthority ?? artifacts.sourceDecisions,
  });
  if (baseAuthorityErrors.length === 0) {
    return result;
  }
  return failure(result.summary, [...baseAuthorityErrors, ...result.errors]);
}

export function validateArtifacts({
  repoRoot,
  universe,
  inventory,
  manifest,
  ledgerSchema,
  ledgerFixture,
  provenance,
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
  validateMigrationLedgerSchema({ ledgerSchema, errors });
  validateLedger({ repoRoot, ledger: ledgerFixture, ledgerSchema, inventory, manifest, errors });
  validateTargetProvenance({ repoRoot, provenance, errors });

  return errors.length === 0
    ? success("Agent language-services lifecycle artifacts match the frozen proposal source and ledger schema contract.")
    : failure("Update the agent language-services lifecycle artifacts before merging; the frozen source universe and ledger schema must stay reviewable.", errors);
}

export function validateLedger({ repoRoot = ".", ledger, ledgerSchema = migrationLedgerSchema(), inventory, manifest, errors = [] }) {
  validateLedgerStructure({ ledger, ledgerSchema, errors, label: ledgerFixturePath });
  const leafIds = inventoryLeafIds(inventory);
  const parentIds = new Set(inventory.roots.filter((root) => root.children?.length > 0).map((root) => root.id));
  const manifestById = new Map(manifest.leaves.map((leaf) => [leaf.id, leaf]));
  const leafIdPatternSource = ledgerSchema?.properties?.entries?.items?.properties?.leaf_id?.pattern;
  const leafIdPattern = isUsableRegExp(leafIdPatternSource)
    ? new RegExp(leafIdPatternSource)
    : /^$/;
  const forbiddenPatterns = forbiddenLeafIdPatterns(ledgerSchema)
    .filter(isUsableRegExp)
    .map((pattern) => new RegExp(pattern));
  const lifecycleEnum = new Set(ledgerSchema?.properties?.entries?.items?.properties?.lifecycle?.enum ?? []);
  const seen = new Set();
  if (!Array.isArray(ledger.entries)) {
    errors.push(`${ledgerFixturePath}: add an entries array.`);
    return errors;
  }
  for (const [index, entry] of ledger.entries.entries()) {
    const label = `${ledgerFixturePath}: entries[${index}]`;
    const leafId = entry.leaf_id;
    if (!isStableLeafId(leafId, leafIdPattern) || forbiddenPatterns.some((pattern) => pattern.test(leafId ?? ""))) {
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
    if (!lifecycleEnum.has(entry.lifecycle)) {
      errors.push(`${label}: set lifecycle to current, completed, planned, or removed.`);
    } else if (manifestById.get(leafId)?.lifecycle !== entry.lifecycle) {
      errors.push(`${label}: keep the ledger lifecycle equal to the frozen lifecycle manifest.`);
    }
    if (entry.lifecycle === "removed" && manifestById.get(leafId)?.conformance !== false) {
      errors.push(`${label}: do not remove a leaf from the frozen conformance universe.`);
    }
    validateDestination({ repoRoot, label, entry, ledgerSchema, errors });
  }
  for (const leafId of leafIds) {
    if (!seen.has(leafId)) {
      errors.push(`${ledgerFixturePath}: add exactly one ledger mapping for ${leafId}.`);
    }
  }
  return errors;
}

export function validateLedgerStructure({ ledger, ledgerSchema = migrationLedgerSchema(), errors = [], label = ledgerFixturePath }) {
  validateJsonSchemaSubset({ schema: ledgerSchema, value: ledger, label, errors });
  return errors;
}

function validateMigrationLedgerSchema({ ledgerSchema, errors }) {
  const expected = migrationLedgerSchema();
  if (JSON.stringify(ledgerSchema) !== JSON.stringify(expected)) {
    errors.push(`${ledgerSchemaPath}: keep the checked migration ledger schema equal to the validator contract.`);
  }
  const leafIdPattern = ledgerSchema?.properties?.entries?.items?.properties?.leaf_id?.pattern;
  if (!isUsableRegExp(leafIdPattern)) {
    errors.push(`${ledgerSchemaPath}: provide a valid leaf_id_pattern.`);
  }
  for (const pattern of forbiddenLeafIdPatterns(ledgerSchema)) {
    if (!isUsableRegExp(pattern)) {
      errors.push(`${ledgerSchemaPath}: provide valid forbidden leaf_id patterns.`);
    }
  }
  for (const lifecycle of lifecycleValues) {
    if (!ledgerSchema?.properties?.entries?.items?.properties?.lifecycle?.enum?.includes(lifecycle)) {
      errors.push(`${ledgerSchemaPath}: preserve lifecycle enum value ${lifecycle}.`);
    }
    if (ledgerSchema?.x_veln_semantic?.destination_kinds_by_lifecycle?.[lifecycle] !== expected.x_veln_semantic.destination_kinds_by_lifecycle[lifecycle]) {
      errors.push(`${ledgerSchemaPath}: preserve destination kind for ${lifecycle}.`);
    }
  }
}

function validateTargetProvenance({ repoRoot, provenance, errors }) {
  if (provenance?.proposal_path !== lifecycleProposalPath) {
    errors.push(`${provenancePath}: bind provenance to the lifecycle migration proposal.`);
  }
  if (provenance?.proposal_anchor !== lifecycleProposalAnchor) {
    errors.push(`${provenancePath}: bind provenance to the frozen-source-universe section.`);
  }
  validateProvenanceAnchor({ repoRoot, provenance, errors });
  if (!isFullGitSha(provenance?.base_commit)) {
    errors.push(`${provenancePath}: record the full bootstrap base commit.`);
  }
  if (provenance?.target_kind !== "proposal-section" || provenance?.default_branch !== "main") {
    errors.push(`${provenancePath}: preserve the bootstrap target kind and default branch.`);
  }
  const frozenArtifacts = new Set(provenance?.frozen_artifact_set ?? []);
  for (const required of [universePath, inventoryPath, manifestPath, ledgerSchemaPath, ledgerFixturePath]) {
    if (!frozenArtifacts.has(required)) {
      errors.push(`${provenancePath}: include ${required} in the frozen artifact set.`);
    }
  }
}

function validateProvenanceAnchor({ repoRoot, provenance, errors }) {
  if (!isRepoMarkdownPath(provenance?.proposal_path)) {
    return;
  }
  const proposalFile = path.resolve(repoRoot, provenance.proposal_path);
  if (!fs.existsSync(proposalFile)) {
    errors.push(`${provenancePath}: resolve proposal path ${provenance.proposal_path}.`);
    return;
  }
  const proposal = fs.readFileSync(proposalFile, "utf8");
  if (!markdownAnchors(proposal).has(provenance.proposal_anchor)) {
    errors.push(`${provenancePath}: resolve proposal anchor ${provenance.proposal_anchor} in ${provenance.proposal_path}.`);
  }
}

export function validateDiffScope({ repoRoot, paths }) {
  const changedPaths = paths.length > 0 ? paths : changedFiles(repoRoot);
  const errors = [];
  const frozenTouched = changedPaths.some((changedPath) => changedPath.startsWith(`${lifecycleDir}/`));
  if (!frozenTouched) {
    return success("No frozen lifecycle artifact changes require bootstrap diff-scope validation.");
  }
  const provenance = fs.existsSync(path.resolve(repoRoot, provenancePath))
    ? readJson(repoRoot, provenancePath)
    : undefined;
  const phase = diffScopePhase({ repoRoot, provenance });
  errors.push(...phase.errors);
  if (!phase.bootstrap) {
    for (const changedPath of changedPaths) {
      if (protectedAfterBootstrap.has(changedPath)) {
        errors.push(`${changedPath}: frozen lifecycle bootstrap files are immutable after the provenance base has merged.`);
      }
    }
    return errors.length === 0
      ? success("No post-bootstrap immutable lifecycle files changed.")
      : failure("Restore frozen lifecycle bootstrap files or start a new reviewed migration target.", errors);
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

function diffScopePhase({ repoRoot, provenance }) {
  const errors = [];
  const base = process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA;
  const head = process.env.AGENT_LANGUAGE_SERVICES_HEAD_SHA;
  const requested = process.env.AGENT_LANGUAGE_SERVICES_BOOTSTRAP === "1";
  const validProvenance = provenance?.proposal_path === lifecycleProposalPath
    && provenance?.proposal_anchor === lifecycleProposalAnchor
    && provenance?.target_kind === "proposal-section"
    && provenance?.default_branch === "main"
    && isFullGitSha(provenance?.base_commit)
    && Array.isArray(provenance?.frozen_artifact_set);
  if (!validProvenance) {
    errors.push(`${provenancePath}: resolve valid target provenance before applying lifecycle diff-scope rules.`);
  }
  if (!base || !head || /^0+$/.test(base) || /^0+$/.test(head ?? "")) {
    if (requested) {
      errors.push("AGENT_LANGUAGE_SERVICES_BOOTSTRAP: provide concrete base and head refs before allowing the frozen inventory bootstrap scope.");
    }
    return { bootstrap: false, errors };
  }
  if (!isDefaultBranchBase(repoRoot, base, provenance?.default_branch ?? "main")) {
    errors.push("AGENT_LANGUAGE_SERVICES_BASE_SHA: resolve a base commit from the configured default branch before allowing bootstrap scope.");
  }
  for (const prerequisite of provenance?.prerequisites ?? []) {
    if (!fs.existsSync(path.resolve(repoRoot, prerequisite))) {
      errors.push(`${prerequisite}: complete the lifecycle prerequisite before applying bootstrap scope.`);
    }
    if (!gitObjectExists(repoRoot, `${base}:${prerequisite}`)) {
      errors.push(`${prerequisite}: prerequisite must already exist on the bootstrap base commit.`);
    }
  }
  validateBootstrapReviewedAuthority({ repoRoot, base, errors });
  const frozenArtifacts = provenance?.frozen_artifact_set ?? [];
  const baseHasFrozenArtifact = frozenArtifacts.some((artifact) => gitObjectExists(repoRoot, `${base}:${artifact}`));
  if (baseHasFrozenArtifact || provenance?.base_commit !== base) {
    return { bootstrap: false, errors };
  }
  return { bootstrap: errors.length === 0, errors };
}

function validateBootstrapReviewedAuthority({ repoRoot, base, errors }) {
  const baseAuthority = readJsonFromGit(repoRoot, `${base}:${sourceDecisionsPath}`);
  if (baseAuthority === undefined) {
    errors.push(`${sourceDecisionsPath}: reviewed source-decision authority must already exist on the bootstrap base commit.`);
    return;
  }
  let artifacts;
  try {
    artifacts = readArtifacts(repoRoot);
  } catch (error) {
    errors.push(`${lifecycleDir}: read committed lifecycle artifacts before comparing bootstrap authority.`);
    return;
  }
  const authorityErrors = [];
  validateSourceDecisions({
    sourceDecisions: baseAuthority,
    universe: artifacts.universe,
    inventoryLeaves: inventoryLeavesForAuthority(artifacts.inventory),
    errors: authorityErrors,
  });
  validateIdentities({
    source: fs.readFileSync(path.resolve(repoRoot, sourcePath), "utf8"),
    universe: artifacts.universe,
    sourceDecisions: baseAuthority,
    errors: authorityErrors,
  });
  const sourceDigest = artifacts.universe?.source_digest;
  if (baseAuthority.source_path !== sourcePath || baseAuthority.source_digest !== sourceDigest) {
    authorityErrors.push(`${sourceDecisionsPath}: keep reviewed source path and digest equal to the frozen source universe.`);
  }
  const authorityLeaves = uniqueById(baseAuthority.leaves, "reviewed source-decision leaves", authorityErrors);
  for (const leaf of artifacts.manifest?.leaves ?? []) {
    const authorityLeaf = authorityLeaves.get(leaf.id);
    if (authorityLeaf === undefined) {
      authorityErrors.push(`${sourceDecisionsPath}: add reviewed lifecycle decision for ${leaf.id}.`);
    } else if (authorityLeaf.root_id !== leaf.root_id || authorityLeaf.lifecycle !== leaf.lifecycle) {
      authorityErrors.push(`${sourceDecisionsPath}: keep ${leaf.id} root and lifecycle equal to the frozen manifest.`);
    }
  }
  if (authorityErrors.length > 0) {
    errors.push(...authorityErrors.map((error) => `${error} Read the reviewed authority from ${base}, not from the head revision.`));
  }
}

function reviewedSourceDecisionsForValidation({ repoRoot, provenance, errors }) {
  const base = process.env.AGENT_LANGUAGE_SERVICES_BASE_SHA;
  const head = process.env.AGENT_LANGUAGE_SERVICES_HEAD_SHA;
  if (!base || !head || /^0+$/.test(base) || /^0+$/.test(head)) {
    return undefined;
  }
  if (provenance?.base_commit !== base) {
    errors.push(`${provenancePath}: base_commit must equal AGENT_LANGUAGE_SERVICES_BASE_SHA before validating reviewed source decisions from the merge base.`);
    return undefined;
  }
  const baseAuthority = readJsonFromGit(repoRoot, `${base}:${sourceDecisionsPath}`);
  if (baseAuthority === undefined) {
    errors.push(`${sourceDecisionsPath}: reviewed source-decision authority must already exist on the validation base commit.`);
    return undefined;
  }
  return baseAuthority;
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
      leaves.set(root.id, {
        id: root.id,
        root_id: root.id,
        heading: root.heading,
        lifecycle: root.lifecycle,
        spans: [root.span],
        digest: root.digest,
        conformance: true,
      });
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
    if (child.heading !== root.heading) {
      errors.push(`${inventoryPath}: bind ${child.id} heading to parent source heading ${JSON.stringify(root.heading)}.`);
    }
    if (!Array.isArray(child.spans) || child.spans.length === 0) {
      errors.push(`${inventoryPath}: ${child.id} needs at least one Unicode-scalar span.`);
      continue;
    }
    const text = child.spans.map((span) => sliceScalars(parsed.text, span[0] - parsed.span[0], span[1] - parsed.span[0])).join("");
    if (child.digest !== sha256(text)) {
      errors.push(`${inventoryPath}: update ${child.id} digest from its exact child source span text.`);
    }
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
    leaves.set(child.id, {
      id: child.id,
      root_id: root.id,
      heading: child.heading,
      lifecycle: child.lifecycle,
      spans: child.spans,
      digest: child.digest,
      conformance: true,
    });
  }
  for (let scalar = parsed.span[0]; scalar < parsed.span[1]; scalar += 1) {
    const value = sliceScalars(parsed.text, scalar - parsed.span[0], scalar - parsed.span[0] + 1);
    if (!seenScalars.has(scalar) && !isChildSpanIgnorableScalar(parsed.text, scalar - parsed.span[0])) {
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
  if (!hasStatementScalar(text)) {
    errors.push(`${inventoryPath}: ${label} must contain a lifecycle statement, not only separators.`);
  }
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
  if (decisionRoots.size !== universe.roots.length) {
    errors.push(`${sourceDecisionsPath}: keep one reviewed root decision for every frozen source-universe root.`);
  }
  for (const root of universe.roots) {
    const decision = decisionRoots.get(root.id);
    if (decision === undefined || decision.digest !== root.digest || decision.conformance !== root.conformance) {
      errors.push(`${sourceDecisionsPath}: keep reviewed root ${root.id} equal to the frozen source-universe contract.`);
    }
  }
  const decisionLeaves = uniqueById(sourceDecisions.leaves, "reviewed source-decision leaves", errors);
  if (decisionLeaves.size !== inventoryLeaves.size) {
    errors.push(`${sourceDecisionsPath}: keep one reviewed lifecycle decision for every inventory leaf.`);
  }
  for (const [leafId, leaf] of inventoryLeaves) {
    const decision = decisionLeaves.get(leafId);
    if (decision === undefined) {
      errors.push(`${sourceDecisionsPath}: add reviewed lifecycle decision for ${leafId}.`);
    } else if (JSON.stringify(reviewedLeafDecision(decision)) !== JSON.stringify(reviewedLeafDecision(leaf))) {
      errors.push(`${sourceDecisionsPath}: keep ${leafId} heading, digest, span, root, and lifecycle equal to the frozen inventory leaf.`);
    }
  }
}

function validateIdentities({ source, universe, sourceDecisions, errors }) {
  const expectedIdentities = expectedFiniteIdentities(source);
  const expectedKeys = expectedFiniteIdentityKeys(source);
  const identityKeys = new Set();
  const identityNamesByKind = new Map();
  for (const identity of universe.identities ?? []) {
    const key = finiteIdentityKey(identity);
    if (identityKeys.has(key)) {
      errors.push(`${universePath}: remove duplicate finite identity occurrence ${identity.kind}:${identity.name}.`);
    }
    identityKeys.add(key);
    const names = identityNamesByKind.get(identity.kind) ?? new Set();
    names.add(identity.name);
    identityNamesByKind.set(identity.kind, names);
    const text = sliceScalars(source, identity.span?.[0] ?? 0, identity.span?.[1] ?? 0);
    if (text !== identity.name) {
      errors.push(`${universePath}: bind identity ${identity.name} to its exact source occurrence.`);
    }
  }
  for (const key of expectedKeys) {
    if (!identityKeys.has(key)) {
      errors.push(`${universePath}: preserve the source-bound finite identity occurrence ${key}.`);
    }
  }
  for (const key of identityKeys) {
    if (!expectedKeys.has(key)) {
      errors.push(`${universePath}: remove detached finite identity occurrence ${key}.`);
    }
  }
  const decisionKeys = new Set();
  for (const identity of sourceDecisions.identities ?? []) {
    const key = finiteIdentityKey(identity);
    if (decisionKeys.has(key)) {
      errors.push(`${sourceDecisionsPath}: remove duplicate finite identity occurrence ${identity.kind}:${identity.name}.`);
    }
    decisionKeys.add(key);
    if (!identityKeys.has(key)) {
      errors.push(`${sourceDecisionsPath}: keep identity ${identity.name} source-bound and equal to the source universe.`);
    }
  }
  for (const key of identityKeys) {
    if (!decisionKeys.has(key)) {
      errors.push(`${sourceDecisionsPath}: keep the reviewed identity set equal to the source universe.`);
    }
  }
  for (const [kind, expectedNames] of expectedIdentities) {
    const actualNames = identityNamesByKind.get(kind) ?? new Set();
    for (const missing of expectedNames) {
      if (!actualNames.has(missing)) {
        errors.push(`${universePath}: preserve the named finite ${kind} identity ${missing}.`);
      }
    }
    for (const extra of actualNames) {
      if (!expectedNames.has(extra)) {
        errors.push(`${universePath}: remove unexpected finite ${kind} identity ${extra}.`);
      }
    }
  }
}

function expectedFiniteIdentities(source) {
  const parsed = finiteIdentities(parseMarkdownSource(source));
  const expected = new Map();
  for (const identity of parsed) {
    const names = expected.get(identity.kind) ?? new Set();
    names.add(identity.name);
    expected.set(identity.kind, names);
  }
  expected.set("evidence-gate", new Set(Array.from({ length: 22 }, (_, index) => `Q${String(index + 1).padStart(2, "0")}`)));
  return expected;
}

function expectedFiniteIdentityKeys(source) {
  return new Set(finiteIdentities(parseMarkdownSource(source)).map(finiteIdentityKey));
}

function finiteIdentityKey(identity) {
  return `${identity.kind}:${identity.name}:${identity.root_id}:${identity.span?.join("-")}`;
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
        heading: root.heading,
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
    if (isSentenceTerminatorAt(root.text, index)) {
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
    .filter((segment) => hasStatementScalar(segment.text));
  const lifecycles = new Set(segments.map((segment) => segment.lifecycle));
  return lifecycles.size > 1 ? segments : [{
    start: 0,
    end: scalars.length,
    text: root.text,
    lifecycle: primaryLifecycle(root.text),
  }];
}

function isSentenceTerminatorAt(text, index) {
  const scalar = Array.from(text)[index];
  return /[.!?。]/u.test(scalar)
    && !isInsideCodeSpan(text, index)
    && !isInsideMarkdownLinkDestination(text, index);
}

function isInsideCodeSpan(text, index) {
  const scalars = Array.from(text);
  const lineStart = lineStartBefore(scalars, index);
  let backtickCount = 0;
  for (let cursor = lineStart; cursor < index; cursor += 1) {
    if (scalars[cursor] === "`") {
      backtickCount += 1;
    }
  }
  return backtickCount % 2 === 1;
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
    ["resource-kind", /\b(?:language-reference index|individual language topics|package-documentation indexes|modules|public declarations|standard-library documentation|virtual source files)\b/g],
    ["package-document-declaration-kind", /`(?:index|module|declaration|status|module-id|declaration-id)`/g],
    ["encoding", /UTF-(?:8|16|32)/g],
    ["client-platform", /`(?:codex|claude-code)\/x86_64-unknown-linux-gnu`/g],
    ["plugin-compatibility-cell", /`(?:client|platform|host_build|manifest_schema_revision|validator_version|validator_digest|veln_contract|mcp_contract|lsp_contract|language_service_contract|reference_schema_contract)`/g],
    ["lsp-field", /`(?:rootUri|veln\/virtualDocument)`/g],
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

function validateDestination({ repoRoot, label, entry, ledgerSchema, errors }) {
  const destination = entry.destination;
  if (!destination || typeof destination !== "object") {
    errors.push(`${label}: add a destination object.`);
    return;
  }
  const expectedKind = ledgerSchema.x_veln_semantic?.destination_kinds_by_lifecycle?.[entry.lifecycle];
  if (destination.kind !== expectedKind) {
    errors.push(`${label}: route ${entry.lifecycle} leaves to a ${expectedKind} destination.`);
  }
  if (entry.lifecycle !== "removed") {
    if (!isRepoMarkdownPath(destination.path)) {
      errors.push(`${label}: use a repository-relative Markdown destination path.`);
    } else {
      const destinationPath = path.resolve(repoRoot, destination.path);
      if (!fs.existsSync(destinationPath)) {
        errors.push(`${label}: resolve destination path ${destination.path}.`);
      } else {
        const markdown = fs.readFileSync(destinationPath, "utf8");
        validateDestinationRole({ label, lifecycle: entry.lifecycle, destination, markdown, errors });
        if (!markdownAnchors(markdown).has(destination.anchor)) {
          errors.push(`${label}: resolve destination anchor ${destination.anchor} in ${destination.path}.`);
        }
      }
    }
    if (!/^#[a-z0-9][a-z0-9-]*$/.test(destination.anchor ?? "")) {
      errors.push(`${label}: use a concrete Markdown heading anchor.`);
    }
    if (!Array.isArray(destination.evidence) || destination.evidence.length === 0) {
      errors.push(`${label}: list checked evidence for the destination.`);
    } else {
      const evidenceSeen = new Set();
      for (const evidence of destination.evidence) {
        if (evidenceSeen.has(evidence)) {
          errors.push(`${label}: list each checked evidence path once.`);
        }
        evidenceSeen.add(evidence);
        if (!isAllowedEvidencePath(evidence) || !fs.existsSync(path.resolve(repoRoot, evidence))) {
          errors.push(`${label}: resolve checked evidence ${evidence}.`);
        }
      }
    }
  } else if (typeof destination.rationale !== "string" || destination.rationale.trim() === "") {
    errors.push(`${label}: removed supporting leaves need a rationale.`);
  }
}

function validateDestinationRole({ label, lifecycle, destination, markdown, errors }) {
  const frontmatter = markdownFrontmatter(markdown);
  const role = frontmatter?.closed ? frontmatterField(frontmatter, "role")[0]?.parsed?.value : undefined;
  const expectedRole = lifecycle === "current"
    ? "specification"
    : lifecycle === "completed"
      ? "implementation-record"
      : "proposal";
  if (role !== expectedRole) {
    errors.push(`${label}: route ${lifecycle} leaves to a ${expectedRole} document.`);
  }
}

function markdownAnchors(markdown) {
  const anchors = new Set();
  for (const line of markdown.split(/\r?\n/)) {
    const match = /^(#{1,6})\s+(.+?)\s*$/.exec(line);
    if (match) {
      anchors.add(`#${match[2].replace(/\s+#+$/, "").toLowerCase().replace(/[^a-z0-9\s-]/g, "").trim().replace(/\s+/g, "-")}`);
    }
  }
  return anchors;
}

function migrationLedgerSchema() {
  return {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: "https://veln-lang.invalid/schemas/agent-language-services-migration-ledger.schema.json",
    title: "Agent language-services migration ledger",
    description: "Structural contract for the later migration ledger. The semantic validator resolves leaf IDs, lifecycle equality, and destination roles.",
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
          required: ["leaf_id", "lifecycle", "destination"],
          properties: {
            leaf_id: {
              type: "string",
              pattern: "^ALS-S[0-9]{4}(\\.[0-9]+)?$",
              not: {
                anyOf: [
                  { pattern: "\\*" },
                  { pattern: "\\.\\." },
                  { pattern: "^all$" },
                  { pattern: "^remaining$" },
                ],
              },
            },
            lifecycle: {
              type: "string",
              enum: ["current", "completed", "planned", "removed"],
            },
            destination: {
              type: "object",
              additionalProperties: false,
              required: ["kind"],
              properties: {
                kind: {
                  type: "string",
                  enum: ["specification", "implementation-record", "proposal", "removed"],
                },
                path: {
                  type: "string",
                  pattern: "^docs/(?:proposals|specification|reference)/.+\\.md$",
                },
                anchor: {
                  type: "string",
                  pattern: "^#[a-z0-9][a-z0-9-]*$",
                },
                evidence: {
                  type: "array",
                  minItems: 1,
                  uniqueItems: true,
                  items: {
                    type: "string",
                    anyOf: [
                      { pattern: "^examples/specification/.+" },
                      { pattern: "^docs/reference/agent-language-services-lifecycle/.+\\.json$" },
                      { pattern: "^docs/(?:proposals|specification|reference)/.+\\.md$" },
                    ],
                  },
                },
                rationale: {
                  type: "string",
                  minLength: 1,
                },
              },
            },
          },
        },
      },
    },
    x_veln_semantic: {
      forbidden_leaf_id_patterns: ["\\*", "\\.\\.", "^all$", "^remaining$"],
      destination_kinds_by_lifecycle: {
        current: "specification",
        completed: "implementation-record",
        planned: "proposal",
        removed: "removed",
      },
    },
  };
}

function targetProvenance(repoRoot) {
  const existing = fs.existsSync(path.resolve(repoRoot, provenancePath))
    ? readJson(repoRoot, provenancePath)
    : undefined;
  const baseCommit = existing?.base_commit ?? gitOutput(repoRoot, ["rev-parse", "HEAD"]) ?? "0".repeat(40);
  return {
    schema_version: 1,
    proposal_path: lifecycleProposalPath,
    proposal_anchor: lifecycleProposalAnchor,
    target_kind: "proposal-section",
    default_branch: "main",
    base_commit: baseCommit.trim(),
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
  return /\s/.test(value) || "|-`[]().,:;!?".includes(value);
}

function hasStatementScalar(text) {
  return Array.from(text).some((scalar) => !isSeparatorScalar(scalar));
}

function isChildSpanIgnorableScalar(text, index) {
  const scalar = Array.from(text)[index];
  if (/\s/.test(scalar)) {
    return true;
  }
  if (scalar === "`") {
    return true;
  }
  if (scalar === "|" && isInsideTableRow(text, index)) {
    return true;
  }
  if (isInsideMarkdownLinkDestination(text, index)) {
    return true;
  }
  if ((scalar === "-" || scalar === "*" || /^\d$/u.test(scalar)) && isListMarkerScalar(text, index)) {
    return true;
  }
  return false;
}

function isInsideTableRow(text, index) {
  const scalars = Array.from(text);
  const lineStart = lineStartBefore(scalars, index);
  const lineEnd = lineEndAfter(scalars, index);
  const line = scalars.slice(lineStart, lineEnd === -1 ? scalars.length : lineEnd).join("");
  return isTableRow(line);
}

function isListMarkerScalar(text, index) {
  const scalars = Array.from(text);
  const lineStart = lineStartBefore(scalars, index);
  const prefix = scalars.slice(lineStart, index + 1).join("");
  const line = scalars.slice(lineStart).join("");
  if (/^\s*[-*]$/.test(prefix) && /^\s*[-*]\s+/.test(line)) {
    return true;
  }
  return /^\s*\d+$/.test(prefix) && /^\s*\d+\.\s+/.test(line);
}

function isInsideMarkdownLinkDestination(text, index) {
  const scalars = Array.from(text);
  let open = -1;
  for (let cursor = index; cursor >= 1; cursor -= 1) {
    if (scalars[cursor - 1] === "]" && scalars[cursor] === "(") {
      open = cursor;
      break;
    }
    if (scalars[cursor] === "\n") {
      break;
    }
  }
  if (open === -1) {
    return false;
  }
  for (let cursor = open + 1; cursor < scalars.length; cursor += 1) {
    if (scalars[cursor] === ")") {
      return index < cursor;
    }
    if (scalars[cursor] === "\n") {
      return false;
    }
  }
  return false;
}

function lineStartBefore(scalars, index) {
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    if (scalars[cursor] === "\n") {
      return cursor + 1;
    }
  }
  return 0;
}

function lineEndAfter(scalars, index) {
  for (let cursor = index; cursor < scalars.length; cursor += 1) {
    if (scalars[cursor] === "\n") {
      return cursor;
    }
  }
  return -1;
}

function sliceScalars(text, start, end) {
  return Array.from(text).slice(start, end).join("");
}

function inventoryLeafIds(inventory) {
  return new Set(inventory.roots.flatMap((root) => root.children?.length > 0
    ? root.children.map((child) => child.id)
    : [root.id]));
}

function inventoryLeavesForAuthority(inventory) {
  const leaves = new Map();
  for (const root of inventory.roots ?? []) {
    if (root.children?.length > 0) {
      for (const child of root.children) {
        leaves.set(child.id, {
          id: child.id,
          root_id: root.id,
          heading: child.heading,
          lifecycle: child.lifecycle,
          spans: child.spans,
          digest: child.digest,
          conformance: true,
        });
      }
    } else {
      leaves.set(root.id, {
        id: root.id,
        root_id: root.id,
        heading: root.heading,
        lifecycle: root.lifecycle,
        spans: [root.span],
        digest: root.digest,
        conformance: true,
      });
    }
  }
  return leaves;
}

function reviewedLeafDecision(leaf) {
  return {
    id: leaf.id,
    root_id: leaf.root_id,
    heading: leaf.heading,
    lifecycle: leaf.lifecycle,
    spans: leaf.spans,
    digest: leaf.digest,
  };
}

function isStableLeafId(value, pattern = /^ALS-S\d{4}(?:\.\d+)?$/) {
  return pattern.test(value ?? "");
}

function isRepoMarkdownPath(value) {
  return typeof value === "string" && /^docs\/(?:proposals|specification|reference)\/.+\.md$/.test(value);
}

function isAllowedEvidencePath(value) {
  return typeof value === "string"
    && (/^examples\/specification\/.+/.test(value)
      || /^docs\/reference\/agent-language-services-lifecycle\/.+\.json$/.test(value)
      || isRepoMarkdownPath(value));
}

function isFullGitSha(value) {
  return typeof value === "string" && /^[0-9a-f]{40}$/.test(value);
}

function isUsableRegExp(pattern) {
  if (typeof pattern !== "string" || pattern === "") {
    return false;
  }
  try {
    new RegExp(pattern);
    return true;
  } catch {
    return false;
  }
}

function forbiddenLeafIdPatterns(ledgerSchema) {
  const notAnyOf = ledgerSchema?.properties?.entries?.items?.properties?.leaf_id?.not?.anyOf ?? [];
  const structural = notAnyOf.map((schema) => schema.pattern).filter((pattern) => typeof pattern === "string");
  const semantic = ledgerSchema?.x_veln_semantic?.forbidden_leaf_id_patterns ?? [];
  return [...new Set([...structural, ...semantic])];
}

function validateJsonSchemaSubset({ schema, value, label, errors }) {
  validateJsonSchemaNode({ schema, value, label, errors });
}

function validateJsonSchemaNode({ schema, value, label, errors }) {
  if (!schema || typeof schema !== "object") {
    errors.push(`${label}: schema node must be an object.`);
    return;
  }
  if (schema.anyOf) {
    const anyValid = schema.anyOf.some((candidate) => {
      const candidateErrors = [];
      validateJsonSchemaNode({ schema: candidate, value, label, errors: candidateErrors });
      return candidateErrors.length === 0;
    });
    if (!anyValid) {
      errors.push(`${label}: match one allowed schema shape.`);
    }
  }
  if (schema.not) {
    const notErrors = [];
    validateJsonSchemaNode({ schema: schema.not, value, label, errors: notErrors });
    if (notErrors.length === 0) {
      errors.push(`${label}: must not match a forbidden schema shape.`);
    }
  }
  if (schema.const !== undefined && value !== schema.const) {
    errors.push(`${label}: use ${JSON.stringify(schema.const)}.`);
  }
  if (schema.enum && !schema.enum.includes(value)) {
    errors.push(`${label}: use one of ${schema.enum.join(", ")}.`);
  }
  if (schema.pattern) {
    if (typeof value !== "string") {
      errors.push(`${label}: use a string matching pattern ${schema.pattern}.`);
    } else if (isUsableRegExp(schema.pattern) && !new RegExp(schema.pattern).test(value)) {
      errors.push(`${label}: match pattern ${schema.pattern}.`);
    }
  }
  if (schema.type === "object") {
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      errors.push(`${label}: use an object.`);
      return;
    }
    for (const required of schema.required ?? []) {
      if (!Object.hasOwn(value, required)) {
        errors.push(`${label}: add required field ${required}.`);
      }
    }
    if (schema.additionalProperties === false) {
      const allowed = new Set(Object.keys(schema.properties ?? {}));
      for (const key of Object.keys(value)) {
        if (!allowed.has(key)) {
          errors.push(`${label}: remove unsupported field ${key}.`);
        }
      }
    }
    for (const [key, childSchema] of Object.entries(schema.properties ?? {})) {
      if (Object.hasOwn(value, key)) {
        validateJsonSchemaNode({ schema: childSchema, value: value[key], label: `${label}.${key}`, errors });
      }
    }
  } else if (schema.type === "array") {
    if (!Array.isArray(value)) {
      errors.push(`${label}: use an array.`);
      return;
    }
    if (Number.isInteger(schema.minItems) && value.length < schema.minItems) {
      errors.push(`${label}: add at least ${schema.minItems} item.`);
    }
    if (schema.uniqueItems) {
      const seen = new Set();
      for (const item of value) {
        const key = JSON.stringify(item);
        if (seen.has(key)) {
          errors.push(`${label}: list each item once.`);
        }
        seen.add(key);
      }
    }
    for (const [index, item] of value.entries()) {
      validateJsonSchemaNode({ schema: schema.items ?? {}, value: item, label: `${label}[${index}]`, errors });
    }
  } else if (schema.type === "string") {
    if (typeof value !== "string") {
      errors.push(`${label}: use a string.`);
      return;
    }
    if (Number.isInteger(schema.minLength) && value.length < schema.minLength) {
      errors.push(`${label}: use a non-empty string.`);
    }
  }
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

function isDefaultBranchBase(repoRoot, base, defaultBranch) {
  const candidates = [
    `refs/remotes/origin/${defaultBranch}`,
    process.env.GITHUB_BASE_REF ? `refs/remotes/origin/${process.env.GITHUB_BASE_REF}` : undefined,
    process.env.GITHUB_BASE_REF ? `refs/heads/${process.env.GITHUB_BASE_REF}` : undefined,
    `refs/heads/${defaultBranch}`,
    defaultBranch,
  ].filter(Boolean);
  for (const candidate of candidates) {
    const ref = gitOutput(repoRoot, ["rev-parse", "--verify", candidate]);
    if (ref === undefined) {
      continue;
    }
    if (ref.trim() === base) {
      return true;
    }
    const result = spawnSync("git", ["merge-base", "--is-ancestor", base, candidate], { cwd: repoRoot, encoding: "utf8" });
    if (result.status === 0) {
      return true;
    }
  }
  return false;
}

function gitObjectExists(repoRoot, objectName) {
  const result = spawnSync("git", ["cat-file", "-e", objectName], { cwd: repoRoot, encoding: "utf8" });
  return result.status === 0;
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

function readJsonFromGit(repoRoot, objectName) {
  const result = spawnSync("git", ["show", objectName], { cwd: repoRoot, encoding: "utf8" });
  if (result.status !== 0) {
    return undefined;
  }
  return JSON.parse(result.stdout);
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
