import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const umbrellaPath = "docs/proposals/agent-language-services.md";
const gateProposalPath = "docs/proposals/agent-language-services-inventory-review-gate.md";
const gateRecordPath = "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md";
const authorityPath = "docs/reference/agent-language-services-lifecycle-review/source-decisions.json";
const lifecycleScriptPath = "workflow-scripts/check-agent-language-services-lifecycle.mjs";
const lifecycleTestPath = "workflow-scripts/check-agent-language-services-lifecycle.test.mjs";
const targetProvenancePath = "docs/reference/agent-language-services-lifecycle/target-provenance.json";
const frozenDirectory = "docs/reference/agent-language-services-lifecycle/";
const sourceUniversePath = `${frozenDirectory}source-universe.json`;
const frozenInventoryPath = `${frozenDirectory}frozen-inventory.json`;
const lifecycleManifestPath = `${frozenDirectory}lifecycle-manifest.json`;
const ledgerSchemaPath = `${frozenDirectory}migration-ledger.schema.json`;
const validLedgerFixturePath = `${frozenDirectory}ledger-fixtures/valid-ledger.json`;
const invalidLedgerFixtureDirectory = `${frozenDirectory}ledger-fixtures/invalid/`;
const workflowPath = ".github/workflows/workflow--test-scripts.yaml";

const g0ToG1Allowlist = new Set([
  ".github/workflows/workflow--test-scripts.yaml",
  "docs/proposals/README.md",
  "docs/proposals/agent-language-services-inventory-review-gate.md",
  "docs/proposals/agent-language-services-lifecycle-migration.md",
  "docs/reference/README.md",
  "docs/reference/agent-language-services-lifecycle-review/source-decisions.json",
  "docs/reference/implemented-proposals/README.md",
  "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md",
  "docs/reference/proposal-target-readiness/manifest.json",
  lifecycleScriptPath,
  lifecycleTestPath,
]);

const immutableG1Paths = new Set([gateRecordPath, authorityPath]);
const g1ToG2AllowlistPrefixes = [
  frozenDirectory,
];
const g1ToG2AllowlistPaths = new Set([
  lifecycleScriptPath,
  lifecycleTestPath,
  workflowPath,
]);
const immutableG2Paths = new Set([
  lifecycleScriptPath,
  lifecycleTestPath,
  workflowPath,
  sourceUniversePath,
  frozenInventoryPath,
  lifecycleManifestPath,
  ledgerSchemaPath,
  validLedgerFixturePath,
]);
const requiredRangeOptions = ["base", "head", "event-base-ref", "default-ref"];
const zeroRevision = /^0{40}$/;

if (isMainModule()) {
  const repoRoot = process.cwd();
  const command = process.argv[2] ?? "validate";
  const result = runCommand({ repoRoot, argv: process.argv.slice(3), command });
  if (!result.valid) {
    console.error(result.summary);
    for (const error of result.errors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }
  console.log(result.summary);
}

export function runCommand({ repoRoot, command, argv = [] }) {
  if (command === "validate") {
    const options = parseOptions(argv);
    return validateContent({
      repoRoot,
      sourcePath: options.source ?? umbrellaPath,
      authorityFile: options.authority ?? authorityPath,
    });
  }
  if (command === "validate-range") {
    return validateRange({ repoRoot, options: parseOptions(argv) });
  }
  if (command === "write-structural-skeleton") {
    const options = parseOptions(argv);
    if (options.output === undefined) {
      return failure("Structural skeleton output is required.", ["write-structural-skeleton: pass --output <path>"]);
    }
    if (normalizeRepoPath(options.output) === authorityPath) {
      return failure("Structural skeleton writer cannot overwrite reviewed authority.", [
        `${authorityPath}: reviewed source decisions must be edited separately from generated structure`,
      ]);
    }
    const source = readUtf8(path.resolve(repoRoot, options.source ?? umbrellaPath));
    const roots = parseMarkdownSource({ source });
    const skeleton = {
      schema_version: 1,
      source_path: options.source ?? umbrellaPath,
      roots: roots.map((root) => ({
        id: root.id,
        kind: root.kind,
        heading: root.heading,
        start_scalar: root.start_scalar,
        end_scalar: root.end_scalar,
        text: root.text,
        digest: digest(root.text),
      })),
    };
    fs.mkdirSync(path.dirname(path.resolve(repoRoot, options.output)), { recursive: true });
    fs.writeFileSync(path.resolve(repoRoot, options.output), `${JSON.stringify(skeleton, null, 2)}\n`);
    return success(`Wrote structural skeleton to ${options.output}.`);
  }
  if (command === "write-frozen-artifacts") {
    const options = parseOptions(argv);
    const baseCommit = options["base-commit"];
    if (baseCommit === undefined || !/^[0-9a-f]{40}$/u.test(baseCommit)) {
      return failure("Frozen artifact writer needs target provenance.", ["write-frozen-artifacts: pass --base-commit <40-character-sha>"]);
    }
    const source = readUtf8(path.resolve(repoRoot, options.source ?? umbrellaPath));
    const authority = readJson(path.resolve(repoRoot, options.authority ?? authorityPath));
    const parsedRoots = parseMarkdownSource({ source });
    const authorityErrors = validateAuthorityShape({ authority, parsedRoots, sourcePath: options.source ?? umbrellaPath });
    if (authorityErrors.length !== 0) {
      return failure("Reviewed authority must be valid before freezing.", authorityErrors);
    }
    writeFrozenArtifacts({ repoRoot, authority, baseCommit });
    return success(`Wrote frozen lifecycle artifacts to ${frozenDirectory}.`);
  }
  return failure("Use validate, validate-range, write-structural-skeleton, or write-frozen-artifacts.", [`unknown command: ${command}`]);
}

export function validateContent({ repoRoot, sourcePath = umbrellaPath, authorityFile = authorityPath }) {
  const source = readUtf8(path.resolve(repoRoot, sourcePath));
  const authority = readJson(path.resolve(repoRoot, authorityFile));
  const parsedRoots = parseMarkdownSource({ source });
  const errors = [
    ...validateAuthorityShape({ authority, parsedRoots, sourcePath }),
    ...validateFrozenArtifacts({ repoRoot, authority, parsedRoots, sourcePath }),
  ];
  return errors.length === 0
    ? success("Agent language-services lifecycle authority and frozen artifacts match the umbrella proposal.")
    : failure("Agent language-services lifecycle authority or frozen artifacts are stale or incomplete.", errors);
}

export function validateFrozenArtifacts({ repoRoot, authority, parsedRoots, sourcePath }) {
  if (!fs.existsSync(path.resolve(repoRoot, frozenDirectory))) {
    return [];
  }
  const errors = [];
  const sourceUniverse = readJsonIfExists(repoRoot, sourceUniversePath, errors);
  const inventory = readJsonIfExists(repoRoot, frozenInventoryPath, errors);
  const manifest = readJsonIfExists(repoRoot, lifecycleManifestPath, errors);
  const schema = readJsonIfExists(repoRoot, ledgerSchemaPath, errors);
  const validLedger = readJsonIfExists(repoRoot, validLedgerFixturePath, errors);
  if ([sourceUniverse, inventory, manifest, schema, validLedger].includes(undefined)) {
    return errors;
  }
  const leafIndex = reviewedLeafIndex(authority);
  errors.push(...validateSourceUniverse({ sourceUniverse, authority, parsedRoots, sourcePath }));
  errors.push(...validateFrozenInventory({ inventory, authority, parsedRoots, sourcePath }));
  errors.push(...validateLifecycleManifest({ manifest, leafIndex }));
  errors.push(...validateLedgerSchema({ schema, leafIndex }));
  errors.push(...validateLedgerFixture({ repoRoot, ledger: validLedger, schema, leafIndex, label: validLedgerFixturePath, expectValid: true }));
  for (const file of listJsonFiles(path.resolve(repoRoot, invalidLedgerFixtureDirectory))) {
    const relative = normalizeRepoPath(path.relative(repoRoot, file));
    const fixture = readJson(file);
    errors.push(...validateLedgerFixture({ repoRoot, ledger: fixture.ledger, schema, leafIndex, label: relative, expectValid: false, expectedError: fixture.expected_error }));
  }
  return errors;
}

export function validateRange({ repoRoot, options }) {
  const inputErrors = validateRangeInputs({ repoRoot, options });
  if (inputErrors.length !== 0) {
    return failure("Agent language-services lifecycle range validation needs complete base/head inputs.", inputErrors);
  }

  const defaultRevision = resolveRevision({ repoRoot, revision: options["default-ref"] });
  const base = resolveRevision({ repoRoot, revision: options.base });
  const head = resolveRevision({ repoRoot, revision: options.head });
  const defaultName = refName(options["default-ref"]);
  const eventBaseName = refName(options["event-base-ref"]);
  const errors = [];
  if (defaultName !== eventBaseName) {
    errors.push(`event base ${options["event-base-ref"]}: match independently resolved default branch ${options["default-ref"]}`);
  }
  if (base !== defaultRevision && head !== defaultRevision) {
    errors.push(`default ref ${options["default-ref"]}: resolve to the event base for pull requests or the event head for pushes`);
  }

  const baseState = lifecycleState({ repoRoot, revision: base });
  const headState = lifecycleState({ repoRoot, revision: head });
  const changedPaths = changedPathSet({ repoRoot, base, head });
  errors.push(...validateTransition({ repoRoot, base, head, baseState, headState, changedPaths }));

  return errors.length === 0
    ? success(`Agent language-services lifecycle transition ${baseState} -> ${headState} is valid.`)
    : failure(`Agent language-services lifecycle transition ${baseState} -> ${headState} is invalid.`, errors);
}

export function parseMarkdownSource({ source }) {
  const lines = source.split(/(?<=\n)/u);
  const roots = [];
  let scalarOffset = 0;
  let inFrontmatter = false;
  let frontmatterDone = false;
  let inFence = false;
  let heading = "";

  for (let index = 0; index < lines.length;) {
    const line = lines[index];
    const bare = line.replace(/\r?\n$/u, "");
    const trimmed = bare.trim();
    const lineStart = scalarOffset;

    if (index === 0 && trimmed === "---") {
      inFrontmatter = true;
      scalarOffset += scalarLength(line);
      index += 1;
      continue;
    }
    if (inFrontmatter) {
      scalarOffset += scalarLength(line);
      index += 1;
      if (trimmed === "---") {
        inFrontmatter = false;
        frontmatterDone = true;
      }
      continue;
    }

    if (trimmed.startsWith("```")) {
      inFence = !inFence;
      scalarOffset += scalarLength(line);
      index += 1;
      continue;
    }
    if (inFence) {
      if (trimmed !== "") {
        roots.push(rootRecord({ kind: "fence_line", heading, start: lineStart, text: bare }));
      }
      scalarOffset += scalarLength(line);
      index += 1;
      continue;
    }
    if (trimmed === "") {
      scalarOffset += scalarLength(line);
      index += 1;
      continue;
    }
    const headingMatch = /^(#{1,6})\s+(.+?)\s*#*\s*$/u.exec(trimmed);
    if (headingMatch) {
      heading = headingMatch[2];
      scalarOffset += scalarLength(line);
      index += 1;
      continue;
    }
    if (/^\|.*\|$/u.test(trimmed)) {
      if (!/^\|\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)+\|?$/u.test(trimmed)) {
        roots.push(rootRecord({ kind: "table_row", heading, start: lineStart, text: bare }));
      }
      scalarOffset += scalarLength(line);
      index += 1;
      continue;
    }
    if (/^\s*(?:[-*+]\s+|\d+\.\s+)/u.test(bare)) {
      const parts = [bare];
      let endIndex = index + 1;
      let endOffset = lineStart + scalarLength(line);
      while (endIndex < lines.length) {
        const next = lines[endIndex];
        const nextBare = next.replace(/\r?\n$/u, "");
        const nextTrimmed = nextBare.trim();
        if (
          nextTrimmed === "" ||
          /^(#{1,6})\s+/u.test(nextTrimmed) ||
          /^\|.*\|$/u.test(nextTrimmed) ||
          /^\s*(?:[-*+]\s+|\d+\.\s+)/u.test(nextBare) ||
          nextTrimmed.startsWith("```")
        ) {
          break;
        }
        parts.push(nextBare);
        endOffset += scalarLength(next);
        endIndex += 1;
      }
      roots.push(rootRecord({ kind: "list_item", heading, start: lineStart, text: parts.join("\n") }));
      while (index < endIndex) {
        scalarOffset += scalarLength(lines[index]);
        index += 1;
      }
      continue;
    }

    const parts = [bare];
    let endIndex = index + 1;
    while (endIndex < lines.length) {
      const nextBare = lines[endIndex].replace(/\r?\n$/u, "");
      const nextTrimmed = nextBare.trim();
      if (
        nextTrimmed === "" ||
        /^(#{1,6})\s+/u.test(nextTrimmed) ||
        /^\|.*\|$/u.test(nextTrimmed) ||
        /^\s*(?:[-*+]\s+|\d+\.\s+)/u.test(nextBare) ||
        nextTrimmed.startsWith("```")
      ) {
        break;
      }
      parts.push(nextBare);
      endIndex += 1;
    }
    roots.push(rootRecord({ kind: "paragraph", heading, start: lineStart, text: parts.join("\n") }));
    while (index < endIndex) {
      scalarOffset += scalarLength(lines[index]);
      index += 1;
    }
    if (!frontmatterDone) {
      frontmatterDone = true;
    }
  }

  return roots.map((root, index) => ({ ...root, id: `ALS-R${String(index + 1).padStart(4, "0")}` }));
}

export function validateAuthorityShape({ authority, parsedRoots, sourcePath }) {
  const errors = [];
  if (authority?.schema_version !== 1) {
    errors.push("source-decisions.json: set schema_version to 1");
  }
  if (authority?.source_path !== sourcePath) {
    errors.push(`source-decisions.json: set source_path to ${sourcePath}`);
  }
  if (!Array.isArray(authority?.roots)) {
    return [...errors, "source-decisions.json: add roots array"];
  }
  if (authority.roots.length !== parsedRoots.length) {
    errors.push(`source-decisions.json: expected ${parsedRoots.length} roots, found ${authority.roots.length}`);
  }
  const seenRoots = new Set();
  const seenLeaves = new Set();
  const leaves = new Map();
  const parsedById = new Map(parsedRoots.map((root) => [root.id, root]));

  for (const [index, root] of authority.roots.entries()) {
    const label = `roots[${index}]`;
    if (seenRoots.has(root?.id)) {
      errors.push(`${label}: duplicate root id ${root.id}`);
    }
    seenRoots.add(root?.id);
    const parsed = parsedById.get(root?.id);
    if (parsed === undefined) {
      errors.push(`${label}: unknown root id ${root?.id}`);
      continue;
    }
    for (const field of ["kind", "heading", "start_scalar", "end_scalar", "text"]) {
      if (root?.[field] !== parsed[field]) {
        errors.push(`${root.id}: ${field} differs from parsed source`);
      }
    }
    if (root?.digest !== digest(parsed.text)) {
      errors.push(`${root.id}: digest changed for source text`);
    }
    if (!["conformance", "supporting"].includes(root?.source_class)) {
      errors.push(`${root.id}: source_class must be conformance or supporting`);
    }
    if (!Array.isArray(root?.leaves) || root.leaves.length === 0) {
      errors.push(`${root.id}: add at least one semantic leaf`);
      continue;
    }
    const covered = new Set();
    for (const [leafIndex, leaf] of root.leaves.entries()) {
      const leafLabel = `${root.id}.leaves[${leafIndex}]`;
      if (!new RegExp(`^${root.id}-L\\d{2}$`).test(leaf?.id ?? "")) {
        errors.push(`${leafLabel}: use contiguous leaf id ${root.id}-LNN`);
      }
      if (seenLeaves.has(leaf?.id)) {
        errors.push(`${leafLabel}: duplicate leaf id ${leaf.id}`);
      }
      seenLeaves.add(leaf?.id);
      leaves.set(leaf?.id, { ...leaf, root });
      if (!["current", "completed", "planned", "removed"].includes(leaf?.lifecycle)) {
        errors.push(`${leafLabel}: lifecycle must be current, completed, planned, or removed`);
      }
      if (root.source_class === "conformance" && leaf.lifecycle === "removed") {
        errors.push(`${leaf.id}: conformance leaf cannot be removed`);
      }
      if (!Array.isArray(leaf?.spans) || leaf.spans.length === 0) {
        errors.push(`${leafLabel}: add at least one source span`);
        continue;
      }
      for (const span of leaf.spans) {
        const spanError = validateSpan({ span, text: root.text });
        if (spanError !== undefined) {
          errors.push(`${leaf.id}: ${spanError}`);
          continue;
        }
        for (let scalar = span[0]; scalar < span[1]; scalar += 1) {
          if (covered.has(scalar)) {
            errors.push(`${leaf.id}: span overlaps another semantic leaf at scalar ${scalar}`);
          }
          covered.add(scalar);
        }
      }
    }
    const expectedLeafIds = root.leaves.map((_, leafIndex) => `${root.id}-L${String(leafIndex + 1).padStart(2, "0")}`);
    const actualLeafIds = root.leaves.map((leaf) => leaf.id);
    if (JSON.stringify(expectedLeafIds) !== JSON.stringify(actualLeafIds)) {
      errors.push(`${root.id}: leaf ids must be contiguous and ordered`);
    }
    for (const scalar of meaningfulScalars(root.text)) {
      if (!covered.has(scalar)) {
        errors.push(`${root.id}: semantic leaves do not cover scalar ${scalar}`);
        break;
      }
    }
  }

  for (const parsed of parsedRoots) {
    if (!seenRoots.has(parsed.id)) {
      errors.push(`${parsed.id}: missing reviewed source root`);
    }
  }

  if (!Array.isArray(authority?.identities)) {
    errors.push("source-decisions.json: add identities array");
  } else {
    const seenIdentities = new Set();
    for (const [index, identity] of authority.identities.entries()) {
      const label = `identities[${index}]`;
      const key = `${identity?.kind}:${identity?.name}:${identity?.root}:${identity?.leaf}:${JSON.stringify(identity?.span)}`;
      if (seenIdentities.has(key)) {
        errors.push(`${label}: duplicate finite identity ${identity.kind}:${identity.name}`);
      }
      seenIdentities.add(key);
      const leaf = leaves.get(identity?.leaf);
      if (leaf === undefined) {
        errors.push(`${label}: identity leaf ${identity?.leaf} is not reviewed`);
        continue;
      }
      if (identity.root !== leaf.root.id) {
        errors.push(`${label}: identity root must match leaf root ${leaf.root.id}`);
      }
      const spanError = validateSpan({ span: identity?.span, text: leaf.root.text });
      if (spanError !== undefined) {
        errors.push(`${label}: ${spanError}`);
      }
    }
    errors.push(...validateRequiredIdentitySets(authority.identities));
  }
  return errors;
}

function validateRequiredIdentitySets(identities) {
  const errors = [];
  const byKind = new Map();
  for (const identity of identities) {
    const names = byKind.get(identity.kind) ?? new Set();
    names.add(identity.name);
    byKind.set(identity.kind, names);
  }
  const qNames = byKind.get("evidence_gate") ?? new Set();
  for (let index = 1; index <= 22; index += 1) {
    const name = `Q${String(index).padStart(2, "0")}`;
    if (!qNames.has(name)) {
      errors.push(`identities: add evidence gate identity ${name}`);
    }
  }
  const requiredKinds = [
    "saved_reference_row",
    "navigation_matrix_row",
    "topic_matrix_row",
    "tool_kind",
    "resource_kind",
    "package_document_declaration_kind",
    "lsp_encoding",
    "plugin_compatibility_cell",
    "unresolved_acceptance_row",
  ];
  for (const kind of requiredKinds) {
    if ((byKind.get(kind)?.size ?? 0) === 0) {
      errors.push(`identities: add at least one ${kind} identity`);
    }
  }
  const savedCount = byKind.get("saved_reference_row")?.size ?? 0;
  if (savedCount !== 6) {
    errors.push(`identities: expected six saved_reference_row identities, found ${savedCount}`);
  }
  return errors;
}

function validateSourceUniverse({ sourceUniverse, authority, parsedRoots, sourcePath }) {
  const errors = [];
  if (sourceUniverse?.schema_version !== 1) {
    errors.push(`${sourceUniversePath}: set schema_version to 1`);
  }
  if (sourceUniverse?.source_path !== sourcePath) {
    errors.push(`${sourceUniversePath}: set source_path to ${sourcePath}`);
  }
  if (!Array.isArray(sourceUniverse?.roots)) {
    return [...errors, `${sourceUniversePath}: add roots array`];
  }
  const parsedById = new Map(parsedRoots.map((root) => [root.id, root]));
  const authorityById = new Map(authority.roots.map((root) => [root.id, root]));
  if (sourceUniverse.roots.length !== parsedRoots.length) {
    errors.push(`${sourceUniversePath}: expected ${parsedRoots.length} roots, found ${sourceUniverse.roots.length}`);
  }
  for (const [index, root] of sourceUniverse.roots.entries()) {
    const label = `${sourceUniversePath}: roots[${index}]`;
    const parsed = parsedById.get(root?.id);
    const reviewed = authorityById.get(root?.id);
    if (parsed === undefined || reviewed === undefined) {
      errors.push(`${label}: unknown root id ${root?.id}`);
      continue;
    }
    for (const field of ["kind", "heading", "start_scalar", "end_scalar", "text"]) {
      if (root?.[field] !== parsed[field]) {
        errors.push(`${sourceUniversePath}: ${root.id} ${field} differs from parsed source`);
      }
    }
    if (root?.digest !== digest(parsed.text)) {
      errors.push(`${sourceUniversePath}: ${root.id} digest changed for source text`);
    }
    if (root?.source_class !== reviewed.source_class) {
      errors.push(`${sourceUniversePath}: ${root.id} source_class differs from reviewed authority`);
    }
    if (!Array.isArray(root?.leaves) || root.leaves.length !== reviewed.leaves.length) {
      errors.push(`${sourceUniversePath}: ${root.id} leaf count differs from reviewed authority`);
      continue;
    }
    for (const [leafIndex, leaf] of root.leaves.entries()) {
      const reviewedLeaf = reviewed.leaves[leafIndex];
      if (leaf?.id !== reviewedLeaf.id) {
        errors.push(`${sourceUniversePath}: ${root.id} leaf ${leafIndex + 1} id differs from reviewed authority`);
      }
      if (JSON.stringify(leaf?.spans) !== JSON.stringify(reviewedLeaf.spans)) {
        errors.push(`${sourceUniversePath}: ${leaf?.id} spans differ from reviewed authority`);
      }
      if (leaf?.source_class !== reviewed.source_class) {
        errors.push(`${sourceUniversePath}: ${leaf?.id} source_class must match parent source_class`);
      }
    }
  }
  if (JSON.stringify(sourceUniverse.identities) !== JSON.stringify(authority.identities)) {
    errors.push(`${sourceUniversePath}: finite identities must remain byte-equivalent to reviewed authority`);
  }
  return errors;
}

function validateFrozenInventory({ inventory, authority, parsedRoots, sourcePath }) {
  const errors = [];
  if (inventory?.schema_version !== 1) {
    errors.push(`${frozenInventoryPath}: set schema_version to 1`);
  }
  if (inventory?.source_path !== sourcePath) {
    errors.push(`${frozenInventoryPath}: set source_path to ${sourcePath}`);
  }
  if (!Array.isArray(inventory?.roots)) {
    return [...errors, `${frozenInventoryPath}: add roots array`];
  }
  const parsedById = new Map(parsedRoots.map((root) => [root.id, root]));
  const authorityById = new Map(authority.roots.map((root) => [root.id, root]));
  const seenRoots = new Set();
  if (inventory.roots.length !== authority.roots.length) {
    errors.push(`${frozenInventoryPath}: expected ${authority.roots.length} roots, found ${inventory.roots.length}`);
  }
  for (const [index, root] of inventory.roots.entries()) {
    if (seenRoots.has(root?.id)) {
      errors.push(`${frozenInventoryPath}: duplicate inventory root ${root.id}`);
    }
    seenRoots.add(root?.id);
    const reviewed = authorityById.get(root?.id);
    const parsed = parsedById.get(root?.id);
    if (reviewed === undefined || parsed === undefined) {
      errors.push(`${frozenInventoryPath}: roots[${index}] unknown root id ${root?.id}`);
      continue;
    }
    for (const field of ["kind", "heading", "start_scalar", "end_scalar", "text", "digest", "source_class"]) {
      if (root?.[field] !== reviewed[field]) {
        errors.push(`${frozenInventoryPath}: ${root.id} ${field} differs from reviewed authority`);
      }
    }
    if (root?.child_count !== reviewed.leaves.length) {
      errors.push(`${frozenInventoryPath}: ${root.id} child_count must be ${reviewed.leaves.length}`);
    }
    const partitionErrors = validateChildPartition({ root, parsed });
    errors.push(...partitionErrors.map((error) => `${frozenInventoryPath}: ${error}`));
    if (!Array.isArray(root?.children) || root.children.length !== reviewed.leaves.length) {
      errors.push(`${frozenInventoryPath}: ${root.id} children must match reviewed leaves`);
      continue;
    }
    for (const [childIndex, child] of root.children.entries()) {
      const reviewedLeaf = reviewed.leaves[childIndex];
      const expectedText = spanText(parsed.text, reviewedLeaf.spans);
      if (child?.id !== reviewedLeaf.id) {
        errors.push(`${frozenInventoryPath}: ${root.id} child ${childIndex + 1} id differs from reviewed authority`);
      }
      if (child?.heading !== reviewed.heading) {
        errors.push(`${frozenInventoryPath}: ${child?.id} heading must match source heading`);
      }
      if (child?.lifecycle !== reviewedLeaf.lifecycle) {
        errors.push(`${frozenInventoryPath}: ${child?.id} lifecycle differs from reviewed authority`);
      }
      if (JSON.stringify(child?.spans) !== JSON.stringify(reviewedLeaf.spans)) {
        errors.push(`${frozenInventoryPath}: ${child?.id} spans differ from reviewed authority`);
      }
      if (child?.text !== expectedText) {
        errors.push(`${frozenInventoryPath}: ${child?.id} text differs from reviewed span text`);
      }
      if (child?.digest !== digest(expectedText)) {
        errors.push(`${frozenInventoryPath}: ${child?.id} digest changed for child text`);
      }
    }
  }
  for (const root of authority.roots) {
    if (!seenRoots.has(root.id)) {
      errors.push(`${frozenInventoryPath}: missing inventory root ${root.id}`);
    }
  }
  return errors;
}

function validateLifecycleManifest({ manifest, leafIndex }) {
  const errors = [];
  if (manifest?.schema_version !== 1) {
    errors.push(`${lifecycleManifestPath}: set schema_version to 1`);
  }
  if (!Array.isArray(manifest?.leaves)) {
    return [...errors, `${lifecycleManifestPath}: add leaves array`];
  }
  const seen = new Set();
  if (manifest.leaves.length !== leafIndex.size) {
    errors.push(`${lifecycleManifestPath}: expected ${leafIndex.size} leaves, found ${manifest.leaves.length}`);
  }
  for (const leaf of manifest.leaves) {
    const reviewed = leafIndex.get(leaf?.id);
    if (seen.has(leaf?.id)) {
      errors.push(`${lifecycleManifestPath}: duplicate leaf ${leaf.id}`);
    }
    seen.add(leaf?.id);
    if (reviewed === undefined) {
      errors.push(`${lifecycleManifestPath}: unknown leaf ${leaf?.id}`);
      continue;
    }
    for (const field of ["root", "lifecycle", "source_class"]) {
      if (leaf?.[field] !== reviewed[field]) {
        errors.push(`${lifecycleManifestPath}: ${leaf.id} ${field} differs from reviewed authority`);
      }
    }
    if (JSON.stringify(leaf?.spans) !== JSON.stringify(reviewed.spans)) {
      errors.push(`${lifecycleManifestPath}: ${leaf.id} spans differ from reviewed authority`);
    }
  }
  for (const leaf of leafIndex.keys()) {
    if (!seen.has(leaf)) {
      errors.push(`${lifecycleManifestPath}: missing leaf ${leaf}`);
    }
  }
  return errors;
}

function validateChildPartition({ root, parsed }) {
  const errors = [];
  if (!Array.isArray(root?.children)) {
    return [`${root?.id}: add children array`];
  }
  const covered = new Set();
  for (const child of root.children) {
    if (!Array.isArray(child?.spans) || child.spans.length === 0) {
      errors.push(`${child?.id}: add at least one source span`);
      continue;
    }
    for (const span of child.spans) {
      const spanError = validateSpan({ span, text: parsed.text });
      if (spanError !== undefined) {
        errors.push(`${child.id}: ${spanError}`);
        continue;
      }
      for (let scalar = span[0]; scalar < span[1]; scalar += 1) {
        if (covered.has(scalar)) {
          errors.push(`${child.id}: span overlaps another child at scalar ${scalar}`);
        }
        covered.add(scalar);
      }
    }
  }
  for (const scalar of meaningfulScalars(parsed.text)) {
    if (!covered.has(scalar)) {
      errors.push(`${root.id}: child spans leave uncovered lifecycle statement at scalar ${scalar}`);
      break;
    }
  }
  return errors;
}

function validateLedgerSchema({ schema, leafIndex }) {
  const errors = [];
  if (schema?.$id !== "https://veln-lang.invalid/schemas/agent-language-services-migration-ledger.schema.json") {
    errors.push(`${ledgerSchemaPath}: keep the stable migration ledger schema $id`);
  }
  if (schema?.type !== "object" || schema?.additionalProperties !== false) {
    errors.push(`${ledgerSchemaPath}: reject unknown ledger fields`);
  }
  for (const field of ["schema_version", "inventory_path", "entries"]) {
    if (!schema?.required?.includes(field)) {
      errors.push(`${ledgerSchemaPath}: require ${field}`);
    }
  }
  const sourcePattern = schema?.properties?.entries?.items?.properties?.source_id?.pattern;
  if (sourcePattern !== "^ALS-R[0-9]{4}-L[0-9]{2}$") {
    errors.push(`${ledgerSchemaPath}: source_id must require one concrete leaf id`);
  }
  const expectedLeafIds = [...leafIndex.keys()];
  const sourceEnum = schema?.properties?.entries?.items?.properties?.source_id?.enum ?? [];
  if (JSON.stringify(sourceEnum) !== JSON.stringify(expectedLeafIds)) {
    errors.push(`${ledgerSchemaPath}: source_id enum must match the frozen leaf set`);
  }
  if (schema?.properties?.entries?.minItems !== leafIndex.size || schema?.properties?.entries?.maxItems !== leafIndex.size) {
    errors.push(`${ledgerSchemaPath}: entries bounds must match the frozen leaf count`);
  }
  if (schema?.properties?.entries?.uniqueItems !== true) {
    errors.push(`${ledgerSchemaPath}: entries must reject byte-identical duplicates`);
  }
  const lifecycleValues = schema?.properties?.entries?.items?.properties?.lifecycle?.enum ?? [];
  for (const lifecycle of ["current", "completed", "planned", "removed"]) {
    if (!lifecycleValues.includes(lifecycle)) {
      errors.push(`${ledgerSchemaPath}: lifecycle enum must include ${lifecycle}`);
    }
  }
  const variants = schema?.properties?.entries?.items?.allOf ?? [];
  const variantLifecycles = new Set((variants ?? [])
    .map((variant) => variant.if?.properties?.lifecycle?.const)
    .filter((lifecycle) => lifecycle !== undefined));
  for (const lifecycle of ["current", "completed", "planned", "removed"]) {
    if (!variantLifecycles.has(lifecycle)) {
      errors.push(`${ledgerSchemaPath}: destination schema must constrain ${lifecycle} entries`);
    }
  }
  return errors;
}

function validateLedgerFixture({ repoRoot, ledger, schema, leafIndex, label, expectValid, expectedError }) {
  const schemaErrors = validateLedgerAgainstSchemaShape({ ledger, schema });
  const semanticErrors = validateLedgerSemantics({ repoRoot, ledger, leafIndex });
  const errors = [...schemaErrors, ...semanticErrors];
  if (expectValid) {
    return errors.map((error) => `${label}: ${error}`);
  }
  if (errors.length === 0) {
    return [`${label}: invalid fixture unexpectedly passed`];
  }
  if (expectedError !== undefined && !errors.some((error) => error.includes(expectedError))) {
    return [`${label}: expected error containing ${JSON.stringify(expectedError)}, got ${errors.join("; ")}`];
  }
  return [];
}

function validateLedgerAgainstSchemaShape({ ledger, schema }) {
  const errors = [];
  const allowedLeafIds = new Set(schema?.properties?.entries?.items?.properties?.source_id?.enum ?? []);
  if (ledger?.schema_version !== 1) {
    errors.push("ledger schema: set schema_version to 1");
  }
  if (ledger?.inventory_path !== frozenInventoryPath) {
    errors.push(`ledger schema: inventory_path must be ${frozenInventoryPath}`);
  }
  if (!Array.isArray(ledger?.entries)) {
    return [...errors, "ledger schema: entries must be an array"];
  }
  for (const [index, entry] of ledger.entries.entries()) {
    const label = `ledger schema: entries[${index}]`;
    const allowed = new Set(["source_id", "lifecycle", "destination"]);
    for (const key of Object.keys(entry ?? {})) {
      if (!allowed.has(key)) {
        errors.push(`${label}: unknown field ${key}`);
      }
    }
    if (!/^ALS-R[0-9]{4}-L[0-9]{2}$/u.test(entry?.source_id ?? "")) {
      errors.push(`${label}: source_id must be one concrete leaf id`);
    } else if (allowedLeafIds.size !== 0 && !allowedLeafIds.has(entry.source_id)) {
      errors.push(`${label}: source_id must be one of the frozen inventory leaves`);
    }
    if (!["current", "completed", "planned", "removed"].includes(entry?.lifecycle)) {
      errors.push(`${label}: lifecycle must be current, completed, planned, or removed`);
    }
    if (typeof entry?.destination !== "object" || entry.destination === null || Array.isArray(entry.destination)) {
      errors.push(`${label}: destination must be an object`);
    } else {
      errors.push(...validateDestinationSchemaShape({ destination: entry.destination, lifecycle: entry.lifecycle, label }));
    }
  }
  return errors;
}

function validateDestinationSchemaShape({ destination, lifecycle, label }) {
  const errors = [];
  if (lifecycle === "current") {
    errors.push(...validateObjectKeys({ object: destination, allowed: ["kind", "path", "anchor", "evidence"], required: ["kind", "path", "anchor", "evidence"], label: `${label}.destination` }));
    if (destination.kind !== "specification") {
      errors.push(`${label}: current destination kind must be specification`);
    }
    if (typeof destination.path !== "string" || !destination.path.startsWith("docs/specification/") || !destination.path.endsWith(".md")) {
      errors.push(`${label}: current destination path must be a specification Markdown page`);
    }
    if (typeof destination.anchor !== "string" || destination.anchor === "") {
      errors.push(`${label}: current destination anchor is required`);
    }
    if (!Array.isArray(destination.evidence) || destination.evidence.length === 0 || !destination.evidence.every((item) => typeof item === "string" && item.startsWith("examples/specification/"))) {
      errors.push(`${label}: current destination evidence must list checked examples`);
    }
  } else if (lifecycle === "completed") {
    errors.push(...validateObjectKeys({ object: destination, allowed: ["kind", "path", "anchor"], required: ["kind", "path", "anchor"], label: `${label}.destination` }));
    if (destination.kind !== "implementation-record") {
      errors.push(`${label}: completed destination kind must be implementation-record`);
    }
    if (typeof destination.path !== "string" || !destination.path.startsWith("docs/reference/implemented-proposals/") || !destination.path.endsWith(".md")) {
      errors.push(`${label}: completed destination path must be an implemented-proposal record`);
    }
    if (typeof destination.anchor !== "string" || destination.anchor === "") {
      errors.push(`${label}: completed destination anchor is required`);
    }
  } else if (lifecycle === "planned") {
    errors.push(...validateObjectKeys({ object: destination, allowed: ["kind", "path", "anchor"], required: ["kind", "path", "anchor"], label: `${label}.destination` }));
    if (destination.kind !== "proposal") {
      errors.push(`${label}: planned destination kind must be proposal`);
    }
    if (typeof destination.path !== "string" || !destination.path.startsWith("docs/proposals/") || !destination.path.endsWith(".md")) {
      errors.push(`${label}: planned destination path must be a proposal Markdown page`);
    }
    if (typeof destination.anchor !== "string" || destination.anchor === "") {
      errors.push(`${label}: planned destination anchor is required`);
    }
  } else if (lifecycle === "removed") {
    errors.push(...validateObjectKeys({ object: destination, allowed: ["kind", "rationale", "superseded_by"], required: ["kind", "rationale", "superseded_by"], label: `${label}.destination` }));
    if (destination.kind !== "removed") {
      errors.push(`${label}: removed destination kind must be removed`);
    }
    if (typeof destination.rationale !== "string" || destination.rationale.trim() === "") {
      errors.push(`${label}: removed destination rationale is required`);
    }
    if (typeof destination.superseded_by !== "string" || destination.superseded_by.trim() === "") {
      errors.push(`${label}: removed destination superseded_by is required`);
    }
  }
  return errors;
}

function validateObjectKeys({ object, allowed, required, label }) {
  const errors = [];
  const allowedSet = new Set(allowed);
  for (const key of Object.keys(object ?? {})) {
    if (!allowedSet.has(key)) {
      errors.push(`${label}: unknown field ${key}`);
    }
  }
  for (const key of required) {
    if (!(key in (object ?? {}))) {
      errors.push(`${label}: missing required field ${key}`);
    }
  }
  return errors;
}

function validateLedgerSemantics({ repoRoot, ledger, leafIndex }) {
  if (!Array.isArray(ledger?.entries)) {
    return [];
  }
  const errors = [];
  const seen = new Set();
  for (const [index, entry] of ledger.entries.entries()) {
    const label = `ledger entries[${index}]`;
    if (/(?:\*|\.\.|ALL_REMAINING|remaining)/u.test(entry?.source_id ?? "")) {
      errors.push(`${label}: range, wildcard, and catch-all source ids are forbidden`);
    }
    if (/^ALS-R[0-9]{4}$/u.test(entry?.source_id ?? "")) {
      errors.push(`${label}: parent source ids cannot be mapped directly`);
    }
    if (seen.has(entry?.source_id)) {
      errors.push(`${label}: duplicate ledger leaf ${entry.source_id}`);
    }
    seen.add(entry?.source_id);
    const reviewed = leafIndex.get(entry?.source_id);
    if (reviewed === undefined) {
      errors.push(`${label}: unknown ledger leaf ${entry?.source_id}`);
      continue;
    }
    if (entry.lifecycle !== reviewed.lifecycle) {
      errors.push(`${label}: lifecycle differs from frozen manifest`);
    }
    if (reviewed.source_class === "conformance" && entry.lifecycle === "removed") {
      errors.push(`${label}: conformance leaf cannot use removed destination`);
    }
    errors.push(...validateDestination({ repoRoot, entry, reviewed, label }));
  }
  for (const leaf of leafIndex.keys()) {
    if (!seen.has(leaf)) {
      errors.push(`ledger: missing ledger leaf ${leaf}`);
    }
  }
  return errors;
}

function validateDestination({ repoRoot, entry, reviewed, label }) {
  const destination = entry.destination;
  const errors = [];
  if (typeof destination !== "object" || destination === null || Array.isArray(destination)) {
    return errors;
  }
  if (entry.lifecycle === "current") {
    if (destination.kind !== "specification") {
      errors.push(`${label}: current destination must be specification`);
    }
    errors.push(...validateMarkdownDestination({ repoRoot, destination, label, role: "specification", pathPrefix: "docs/specification/" }));
    if (!Array.isArray(destination.evidence) || destination.evidence.length === 0) {
      errors.push(`${label}: current destination needs checked evidence`);
    } else {
      const evidence = new Set();
      for (const item of destination.evidence) {
        if (evidence.has(item)) {
          errors.push(`${label}: duplicate checked evidence ${item}`);
        }
        evidence.add(item);
        if (!String(item).startsWith("examples/specification/")) {
          errors.push(`${label}: checked evidence must resolve under examples/specification/`);
        }
        if (!fs.existsSync(path.resolve(repoRoot, item))) {
          errors.push(`${label}: checked evidence ${item} does not exist`);
        }
      }
    }
  } else if (entry.lifecycle === "completed") {
    if (destination.kind !== "implementation-record") {
      errors.push(`${label}: completed destination must be implementation-record`);
    }
    errors.push(...validateMarkdownDestination({ repoRoot, destination, label, role: "implementation-record", pathPrefix: "docs/reference/implemented-proposals/" }));
  } else if (entry.lifecycle === "planned") {
    if (destination.kind !== "proposal") {
      errors.push(`${label}: planned destination must be proposal`);
    }
    errors.push(...validateMarkdownDestination({ repoRoot, destination, label, role: "proposal", pathPrefix: "docs/proposals/" }));
  } else if (entry.lifecycle === "removed") {
    if (reviewed.source_class === "conformance") {
      errors.push(`${label}: removed is only available for supporting leaves`);
    }
    if (typeof destination.rationale !== "string" || destination.rationale.trim() === "") {
      errors.push(`${label}: removed destination needs rationale`);
    }
    if (typeof destination.superseded_by !== "string" || destination.superseded_by.trim() === "") {
      errors.push(`${label}: removed destination needs superseded_by`);
    }
  }
  return errors;
}

function validateMarkdownDestination({ repoRoot, destination, label, role, pathPrefix }) {
  const errors = [];
  if (typeof destination.path !== "string" || !destination.path.startsWith(pathPrefix) || !destination.path.endsWith(".md")) {
    errors.push(`${label}: destination path must be under ${pathPrefix}`);
    return errors;
  }
  const fullPath = path.resolve(repoRoot, destination.path);
  if (!fs.existsSync(fullPath)) {
    errors.push(`${label}: destination path ${destination.path} does not exist`);
    return errors;
  }
  const text = readUtf8(fullPath);
  if (frontmatterRole(text) !== role) {
    errors.push(`${label}: destination role must be ${role}`);
  }
  if (typeof destination.anchor !== "string" || destination.anchor === "") {
    errors.push(`${label}: destination anchor is required`);
  } else if (!markdownAnchors(text).has(destination.anchor)) {
    errors.push(`${label}: destination anchor ${destination.anchor} does not exist`);
  }
  return errors;
}

function writeFrozenArtifacts({ repoRoot, authority, baseCommit }) {
  const leafIndex = reviewedLeafIndex(authority);
  const sourceUniverse = {
    schema_version: 1,
    source_path: authority.source_path,
    roots: authority.roots.map((root) => ({
      id: root.id,
      kind: root.kind,
      heading: root.heading,
      start_scalar: root.start_scalar,
      end_scalar: root.end_scalar,
      text: root.text,
      digest: root.digest,
      source_class: root.source_class,
      leaves: root.leaves.map((leaf) => ({
        id: leaf.id,
        source_class: root.source_class,
        spans: leaf.spans,
      })),
    })),
    identities: authority.identities,
  };
  const inventory = {
    schema_version: 1,
    source_path: authority.source_path,
    roots: authority.roots.map((root) => ({
      id: root.id,
      kind: root.kind,
      heading: root.heading,
      start_scalar: root.start_scalar,
      end_scalar: root.end_scalar,
      text: root.text,
      digest: root.digest,
      source_class: root.source_class,
      child_count: root.leaves.length,
      children: root.leaves.map((leaf) => {
        const text = spanText(root.text, leaf.spans);
        return {
          id: leaf.id,
          heading: root.heading,
          lifecycle: leaf.lifecycle,
          spans: leaf.spans,
          text,
          digest: digest(text),
        };
      }),
    })),
  };
  const manifest = {
    schema_version: 1,
    inventory_path: frozenInventoryPath,
    leaves: [...leafIndex.values()].map((leaf) => ({
      id: leaf.id,
      root: leaf.root,
      source_class: leaf.source_class,
      lifecycle: leaf.lifecycle,
      spans: leaf.spans,
    })),
  };
  const schema = migrationLedgerSchema(leafIndex);
  const validLedger = {
    schema_version: 1,
    inventory_path: frozenInventoryPath,
    entries: [...leafIndex.values()].map((leaf) => ({
      source_id: leaf.id,
      lifecycle: leaf.lifecycle,
      destination: ledgerDestinationFor(leaf),
    })),
  };
  const invalidFixtures = invalidLedgerFixtures(validLedger, leafIndex);
  writeJson(repoRoot, targetProvenancePath, {
    schema_version: 1,
    base_commit: baseCommit,
    proposal_path: "docs/proposals/agent-language-services-lifecycle-migration.md",
    proposal_anchor: "#frozen-source-universe",
  });
  writeJson(repoRoot, sourceUniversePath, sourceUniverse);
  writeJson(repoRoot, frozenInventoryPath, inventory);
  writeJson(repoRoot, lifecycleManifestPath, manifest);
  writeJson(repoRoot, ledgerSchemaPath, schema);
  writeJson(repoRoot, validLedgerFixturePath, validLedger);
  for (const [name, fixture] of Object.entries(invalidFixtures)) {
    writeJson(repoRoot, `${invalidLedgerFixtureDirectory}${name}.json`, fixture);
  }
}

function validateTransition({ repoRoot, base, head, baseState, headState, changedPaths }) {
  const errors = [];
  const transition = `${baseState}->${headState}`;
  if (transition === "G0->G0") {
    if (addsLaterStateArtifact({ repoRoot, base, head })) {
      errors.push("G0 -> G0: ordinary changes cannot add G1 authority, target provenance, or frozen artifacts");
    }
    return errors;
  }
  if (transition === "G0->G1") {
    for (const changedPath of changedPaths) {
      if (!g0ToG1Allowlist.has(changedPath)) {
        errors.push(`G0 -> G1: ${changedPath} is outside the closed review-gate allowlist`);
      }
    }
    errors.push(...validateHeadContent({ repoRoot, head }));
    return errors;
  }
  if (transition === "G0->G2") {
    return ["G0 -> G2: complete the review gate before adding frozen lifecycle artifacts"];
  }
  if (transition === "G1->G1") {
    errors.push(...rejectChangedImmutablePaths({ repoRoot, base, head, paths: immutableG1Paths, label: "G1 authority" }));
    return errors;
  }
  if (transition === "G1->G2") {
    errors.push(...rejectChangedImmutablePaths({ repoRoot, base, head, paths: immutableG1Paths, label: "G1 authority" }));
    errors.push(...validateTargetProvenance({ repoRoot, base, head }));
    for (const changedPath of changedPaths) {
      if (!isG1ToG2AllowedPath(changedPath)) {
        errors.push(`G1 -> G2: ${changedPath} is outside the frozen-inventory bootstrap allowlist`);
      }
    }
    errors.push(...validateFrozenHeadContent({ repoRoot, head }));
    return errors;
  }
  if (transition === "G2->G2") {
    errors.push(...rejectFrozenArtifactChanges({ changedPaths }));
    return errors;
  }
  return [`${transition}: unrecognized lifecycle transition`];
}

function validateHeadContent({ repoRoot, head }) {
  const authorityText = gitFileAtRevision({ repoRoot, revision: head, file: authorityPath });
  const sourceText = gitFileAtRevision({ repoRoot, revision: head, file: umbrellaPath });
  if (authorityText === undefined || sourceText === undefined) {
    return [`${authorityPath}: reviewed authority and ${umbrellaPath} must both exist in G1`];
  }
  let authority;
  try {
    authority = JSON.parse(authorityText);
  } catch {
    return [`${authorityPath}: must be valid JSON`];
  }
  const errors = validateAuthorityShape({ authority, parsedRoots: parseMarkdownSource({ source: sourceText }), sourcePath: umbrellaPath });
  return errors.map((error) => `${authorityPath}: ${error}`);
}

function validateTargetProvenance({ repoRoot, base, head }) {
  const text = gitFileAtRevision({ repoRoot, revision: head, file: targetProvenancePath });
  if (text === undefined) {
    return [`${targetProvenancePath}: add tracked G1 target provenance before the frozen inventory`];
  }
  let metadata;
  try {
    metadata = JSON.parse(text);
  } catch {
    return [`${targetProvenancePath}: use valid JSON`];
  }
  const errors = [];
  if (metadata.base_commit !== base) {
    errors.push(`${targetProvenancePath}: base_commit must equal exact G1 base ${base}`);
  }
  if (metadata.proposal_path !== "docs/proposals/agent-language-services-lifecycle-migration.md") {
    errors.push(`${targetProvenancePath}: proposal_path must name the lifecycle migration proposal`);
  }
  if (metadata.proposal_anchor !== "#frozen-source-universe") {
    errors.push(`${targetProvenancePath}: proposal_anchor must be #frozen-source-universe`);
  }
  return errors;
}

function lifecycleState({ repoRoot, revision }) {
  const hasGateProposal = gitPathExists({ repoRoot, revision, file: gateProposalPath });
  const hasGateRecord = gitPathExists({ repoRoot, revision, file: gateRecordPath });
  const hasAuthority = gitPathExists({ repoRoot, revision, file: authorityPath });
  const hasFrozen = gitTreeHasPrefix({ repoRoot, revision, prefix: "docs/reference/agent-language-services-lifecycle/" });
  if (hasGateProposal && !hasGateRecord && !hasAuthority && !hasFrozen) {
    return "G0";
  }
  if (!hasGateProposal && hasGateRecord && hasAuthority && !hasFrozen) {
    return "G1";
  }
  if (!hasGateProposal && hasGateRecord && hasAuthority && hasFrozen) {
    return "G2";
  }
  return "invalid";
}

function addsLaterStateArtifact({ repoRoot, base, head }) {
  return [gateRecordPath, authorityPath, targetProvenancePath].some((file) => !gitPathExists({ repoRoot, revision: base, file }) && gitPathExists({ repoRoot, revision: head, file }))
    || (!gitTreeHasPrefix({ repoRoot, revision: base, prefix: "docs/reference/agent-language-services-lifecycle/" })
      && gitTreeHasPrefix({ repoRoot, revision: head, prefix: "docs/reference/agent-language-services-lifecycle/" }));
}

function rejectChangedImmutablePaths({ repoRoot, base, head, paths, label }) {
  const errors = [];
  for (const file of paths) {
    const baseText = gitFileAtRevision({ repoRoot, revision: base, file });
    const headText = gitFileAtRevision({ repoRoot, revision: head, file });
    if (baseText !== headText) {
      errors.push(`${file}: ${label} must remain byte-identical`);
    }
  }
  return errors;
}

function rejectFrozenArtifactChanges({ changedPaths }) {
  return [...changedPaths]
    .filter((file) => file.startsWith(frozenDirectory) || immutableG2Paths.has(file))
    .map((file) => `${file}: frozen lifecycle artifacts are immutable after bootstrap`);
}

function isG1ToG2AllowedPath(file) {
  return g1ToG2AllowlistPaths.has(file) || g1ToG2AllowlistPrefixes.some((prefix) => file.startsWith(prefix));
}

function validateFrozenHeadContent({ repoRoot, head }) {
  const required = [sourceUniversePath, frozenInventoryPath, lifecycleManifestPath, ledgerSchemaPath, validLedgerFixturePath];
  const missing = required.filter((file) => !gitPathExists({ repoRoot, revision: head, file }));
  if (missing.length !== 0) {
    return missing.map((file) => `${file}: add required frozen lifecycle artifact`);
  }
  const sourceText = gitFileAtRevision({ repoRoot, revision: head, file: umbrellaPath });
  const authorityText = gitFileAtRevision({ repoRoot, revision: head, file: authorityPath });
  if (sourceText === undefined || authorityText === undefined) {
    return [`${umbrellaPath}: source and reviewed authority must both exist in G2`];
  }
  const fixtureRoot = fs.mkdtempSync(path.join("/tmp", "als-g2-"));
  try {
    for (const file of [sourceUniversePath, frozenInventoryPath, lifecycleManifestPath, ledgerSchemaPath, validLedgerFixturePath]) {
      const text = gitFileAtRevision({ repoRoot, revision: head, file });
      writeFileRaw(fixtureRoot, file, text);
    }
    const invalidFiles = gitLsTree({ repoRoot, revision: head, prefix: invalidLedgerFixtureDirectory });
    for (const file of invalidFiles) {
      writeFileRaw(fixtureRoot, file, gitFileAtRevision({ repoRoot, revision: head, file }));
    }
    writeFileRaw(fixtureRoot, umbrellaPath, sourceText);
    writeFileRaw(fixtureRoot, authorityPath, authorityText);
    for (const file of [
      "docs/specification/README.md",
      "docs/specification/mcp.md",
      "docs/proposals/agent-language-services.md",
      "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md",
      "examples/specification/README.md",
    ]) {
      const text = gitFileAtRevision({ repoRoot, revision: head, file });
      if (text !== undefined) {
        writeFileRaw(fixtureRoot, file, text);
      }
    }
    const authority = JSON.parse(authorityText);
    return validateFrozenArtifacts({
      repoRoot: fixtureRoot,
      authority,
      parsedRoots: parseMarkdownSource({ source: sourceText }),
      sourcePath: umbrellaPath,
    }).map((error) => `G1 -> G2: ${error}`);
  } finally {
    fs.rmSync(fixtureRoot, { recursive: true, force: true });
  }
}

function changedPathSet({ repoRoot, base, head }) {
  const result = git({ repoRoot, args: ["diff", "--name-status", "--find-renames", "--find-copies", base, head] });
  const paths = new Set();
  for (const line of result.stdout.trim().split("\n").filter(Boolean)) {
    const parts = line.split("\t");
    for (const candidate of parts.slice(1)) {
      paths.add(candidate);
    }
  }
  return paths;
}

function validateRangeInputs({ repoRoot, options }) {
  const errors = [];
  for (const option of requiredRangeOptions) {
    const value = options[option];
    if (value === undefined || value.trim() === "") {
      errors.push(`--${option}: required range input is missing`);
    } else if (zeroRevision.test(value)) {
      errors.push(`--${option}: all-zero revisions are not valid lifecycle inputs`);
    }
  }
  if (errors.length !== 0) {
    return errors;
  }
  for (const option of ["base", "head", "default-ref"]) {
    if (resolveRevision({ repoRoot, revision: options[option] }) === undefined) {
      errors.push(`--${option}: revision ${options[option]} does not resolve`);
    }
  }
  return errors;
}

function validateSpan({ span, text }) {
  if (!Array.isArray(span) || span.length !== 2 || !Number.isInteger(span[0]) || !Number.isInteger(span[1])) {
    return "span must be [start, end] integers";
  }
  if (span[0] < 0 || span[1] <= span[0] || span[1] > scalarLength(text)) {
    return `span ${JSON.stringify(span)} is out of range`;
  }
  return undefined;
}

function reviewedLeafIndex(authority) {
  const leaves = new Map();
  for (const root of authority.roots ?? []) {
    for (const leaf of root.leaves ?? []) {
      leaves.set(leaf.id, {
        id: leaf.id,
        root: root.id,
        heading: root.heading,
        source_class: root.source_class,
        lifecycle: leaf.lifecycle,
        spans: leaf.spans,
      });
    }
  }
  return leaves;
}

function spanText(text, spans) {
  const scalars = [...text];
  return spans.map(([start, end]) => scalars.slice(start, end).join("")).join("");
}

function migrationLedgerSchema(leafIndex) {
  const leafIds = [...leafIndex.keys()];
  return {
    $schema: "https://json-schema.org/draft/2020-12/schema",
    $id: "https://veln-lang.invalid/schemas/agent-language-services-migration-ledger.schema.json",
    type: "object",
    additionalProperties: false,
    required: ["schema_version", "inventory_path", "entries"],
    properties: {
      schema_version: { const: 1 },
      inventory_path: { const: frozenInventoryPath },
      entries: {
        type: "array",
        minItems: leafIds.length,
        maxItems: leafIds.length,
        uniqueItems: true,
        items: {
          type: "object",
          additionalProperties: false,
          required: ["source_id", "lifecycle", "destination"],
          properties: {
            source_id: { type: "string", pattern: "^ALS-R[0-9]{4}-L[0-9]{2}$", enum: leafIds },
            lifecycle: { type: "string", enum: ["current", "completed", "planned", "removed"] },
            destination: {
              type: "object",
            },
          },
          allOf: [
            {
              if: { properties: { lifecycle: { const: "current" } }, required: ["lifecycle"] },
              then: {
                properties: {
                  destination: {
                    type: "object",
                    additionalProperties: false,
                    required: ["kind", "path", "anchor", "evidence"],
                    properties: {
                      kind: { const: "specification" },
                      path: { type: "string", pattern: "^docs/specification/.*\\.md$" },
                      anchor: { type: "string", minLength: 1 },
                      evidence: {
                        type: "array",
                        minItems: 1,
                        uniqueItems: true,
                        items: { type: "string", pattern: "^examples/specification/" },
                      },
                    },
                  },
                },
              },
            },
            {
              if: { properties: { lifecycle: { const: "completed" } }, required: ["lifecycle"] },
              then: {
                properties: {
                  destination: {
                    type: "object",
                    additionalProperties: false,
                    required: ["kind", "path", "anchor"],
                    properties: {
                      kind: { const: "implementation-record" },
                      path: { type: "string", pattern: "^docs/reference/implemented-proposals/.*\\.md$" },
                      anchor: { type: "string", minLength: 1 },
                    },
                  },
                },
              },
            },
            {
              if: { properties: { lifecycle: { const: "planned" } }, required: ["lifecycle"] },
              then: {
                properties: {
                  destination: {
                    type: "object",
                    additionalProperties: false,
                    required: ["kind", "path", "anchor"],
                    properties: {
                      kind: { const: "proposal" },
                      path: { type: "string", pattern: "^docs/proposals/.*\\.md$" },
                      anchor: { type: "string", minLength: 1 },
                    },
                  },
                },
              },
            },
            {
              if: { properties: { lifecycle: { const: "removed" } }, required: ["lifecycle"] },
              then: {
                properties: {
                  destination: {
                    type: "object",
                    additionalProperties: false,
                    required: ["kind", "rationale", "superseded_by"],
                    properties: {
                      kind: { const: "removed" },
                      rationale: { type: "string", minLength: 1 },
                      superseded_by: { type: "string", minLength: 1 },
                    },
                  },
                },
              },
            },
          ],
        },
      },
    },
  };
}

function ledgerDestinationFor(leaf) {
  if (leaf.lifecycle === "current") {
    return {
      kind: "specification",
      path: "docs/specification/mcp.md",
      anchor: "#mcp-workspace-projects-diagnostics-and-definitions",
      evidence: ["examples/specification/README.md"],
    };
  }
  if (leaf.lifecycle === "completed") {
    return {
      kind: "implementation-record",
      path: "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md",
      anchor: "#agent-language-services-inventory-review-gate",
    };
  }
  if (leaf.lifecycle === "planned") {
    return {
      kind: "proposal",
      path: "docs/proposals/agent-language-services.md",
      anchor: "#agent-language-services",
    };
  }
  return {
    kind: "removed",
    rationale: "Supporting explanation is duplicated by the lifecycle migration record.",
    superseded_by: "docs/reference/implemented-proposals/agent-language-services-inventory-review-gate.md#agent-language-services-inventory-review-gate",
  };
}

function invalidLedgerFixtures(validLedger, leafIndex) {
  const first = validLedger.entries[0];
  const second = validLedger.entries[1];
  const conformance = validLedger.entries.find((entry) => leafIndex.get(entry.source_id)?.source_class === "conformance") ?? first;
  const fixtures = {};
  fixtures.missing_leaf = fixtureFrom(validLedger, "missing ledger leaf", (ledger) => {
    ledger.entries.shift();
  });
  fixtures.duplicate_leaf = fixtureFrom(validLedger, "duplicate ledger leaf", (ledger) => {
    ledger.entries.splice(1, 0, structuredClone(first));
  });
  fixtures.parent_mapping = fixtureFrom(validLedger, "parent source ids cannot be mapped directly", (ledger) => {
    ledger.entries[0].source_id = first.source_id.replace(/-L[0-9]{2}$/u, "");
  });
  fixtures.wildcard_leaf = fixtureFrom(validLedger, "range, wildcard, and catch-all", (ledger) => {
    ledger.entries[0].source_id = "ALS-R0001-L*";
  });
  fixtures.range_leaf = fixtureFrom(validLedger, "range, wildcard, and catch-all", (ledger) => {
    ledger.entries[0].source_id = "ALS-R0001-L01..ALS-R0002-L01";
  });
  fixtures.catch_all_leaf = fixtureFrom(validLedger, "range, wildcard, and catch-all", (ledger) => {
    ledger.entries[0].source_id = "ALL_REMAINING";
  });
  fixtures.unknown_leaf = fixtureFrom(validLedger, "unknown ledger leaf", (ledger) => {
    ledger.entries[0].source_id = "ALS-R9999-L99";
  });
  fixtures.lifecycle_mismatch = fixtureFrom(validLedger, "lifecycle differs from frozen manifest", (ledger) => {
    ledger.entries[0].lifecycle = first.lifecycle === "planned" ? "current" : "planned";
  });
  fixtures.invalid_removed_conformance = fixtureFrom(validLedger, "conformance leaf cannot use removed destination", (ledger) => {
    ledger.entries[0] = {
      ...structuredClone(conformance),
      lifecycle: "removed",
      destination: ledgerDestinationFor({ ...leafIndex.get(conformance.source_id), lifecycle: "removed" }),
    };
  });
  fixtures.direct_parent_duplicate_guard = fixtureFrom(validLedger, "duplicate ledger leaf", (ledger) => {
    ledger.entries[0] = structuredClone(second);
  });
  fixtures.invalid_destination_shape = fixtureFrom(validLedger, "planned destination kind must be proposal", (ledger) => {
    const planned = ledger.entries.find((entry) => entry.lifecycle === "planned") ?? first;
    planned.destination = {
      kind: "specification",
      path: "docs/specification/mcp.md",
      anchor: "#mcp-workspace-projects-diagnostics-and-definitions",
      evidence: ["examples/specification/README.md"],
    };
  });
  return fixtures;
}

function fixtureFrom(validLedger, expectedError, mutate) {
  const ledger = structuredClone(validLedger);
  mutate(ledger);
  return {
    expected_error: expectedError,
    ledger,
  };
}

function readJsonIfExists(repoRoot, file, errors) {
  const fullPath = path.resolve(repoRoot, file);
  if (!fs.existsSync(fullPath)) {
    errors.push(`${file}: required lifecycle artifact is missing`);
    return undefined;
  }
  try {
    return readJson(fullPath);
  } catch {
    errors.push(`${file}: must be valid JSON`);
    return undefined;
  }
}

function listJsonFiles(directory) {
  if (!fs.existsSync(directory)) {
    return [];
  }
  return fs.readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".json"))
    .map((entry) => path.join(directory, entry.name))
    .sort();
}

function frontmatterRole(text) {
  const match = /^---\n(?<frontmatter>[\s\S]*?)\n---/u.exec(text);
  if (match === null) {
    return undefined;
  }
  const role = /^role:\s*(.+?)\s*$/mu.exec(match.groups.frontmatter);
  return role?.[1];
}

function markdownAnchors(text) {
  const anchors = new Set();
  for (const line of text.split("\n")) {
    const match = /^(#{1,6})\s+(.+?)\s*#*\s*$/u.exec(line.trim());
    if (match !== null) {
      anchors.add(`#${slugifyHeading(match[2])}`);
    }
  }
  return anchors;
}

function slugifyHeading(heading) {
  return heading
    .toLowerCase()
    .replace(/`([^`]+)`/gu, "$1")
    .replace(/[^\p{Letter}\p{Number}\s-]/gu, "")
    .trim()
    .replace(/\s+/gu, "-");
}

function writeJson(repoRoot, file, value) {
  writeFileRaw(repoRoot, file, `${JSON.stringify(value, null, 2)}\n`);
}

function writeFileRaw(repoRoot, file, text) {
  const destination = path.resolve(repoRoot, file);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, text);
}

function meaningfulScalars(text) {
  const scalars = [];
  let index = 0;
  for (const char of text) {
    if (!/\s/u.test(char) && !["|", "`"].includes(char)) {
      scalars.push(index);
    }
    index += 1;
  }
  return scalars;
}

function rootRecord({ kind, heading, start, text }) {
  return {
    id: "",
    kind,
    heading,
    start_scalar: start,
    end_scalar: start + scalarLength(text),
    text,
    digest: digest(text),
  };
}

function digest(text) {
  return crypto.createHash("sha256").update(text, "utf8").digest("hex");
}

function scalarLength(text) {
  return [...text].length;
}

function parseOptions(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (!key.startsWith("--")) {
      continue;
    }
    options[key.slice(2)] = argv[index + 1] ?? "";
    index += 1;
  }
  return options;
}

function refName(ref) {
  return ref.replace(/^refs\/heads\//u, "").replace(/^origin\//u, "");
}

function normalizeRepoPath(file) {
  return file.replaceAll("\\", "/").replace(/^\.\//u, "");
}

function resolveRevision({ repoRoot, revision }) {
  const result = git({ repoRoot, args: ["rev-parse", "--verify", `${revision}^{commit}`], allowFailure: true });
  if (result.status === 0) {
    return result.stdout.trim();
  }
  if (!revision.startsWith("origin/") && !revision.startsWith("refs/") && !/^[0-9a-f]{40}$/u.test(revision)) {
    const remoteResult = git({ repoRoot, args: ["rev-parse", "--verify", `origin/${revision}^{commit}`], allowFailure: true });
    if (remoteResult.status === 0) {
      return remoteResult.stdout.trim();
    }
  }
  return undefined;
}

function gitPathExists({ repoRoot, revision, file }) {
  const result = git({ repoRoot, args: ["cat-file", "-e", `${revision}:${file}`], allowFailure: true });
  return result.status === 0;
}

function gitTreeHasPrefix({ repoRoot, revision, prefix }) {
  const result = git({ repoRoot, args: ["ls-tree", "-r", "--name-only", revision, "--", prefix], allowFailure: true });
  return result.status === 0 && result.stdout.trim() !== "";
}

function gitLsTree({ repoRoot, revision, prefix }) {
  const result = git({ repoRoot, args: ["ls-tree", "-r", "--name-only", revision, "--", prefix], allowFailure: true });
  if (result.status !== 0 || result.stdout.trim() === "") {
    return [];
  }
  return result.stdout.trim().split("\n");
}

function gitFileAtRevision({ repoRoot, revision, file }) {
  const result = git({ repoRoot, args: ["show", `${revision}:${file}`], allowFailure: true });
  return result.status === 0 ? result.stdout : undefined;
}

function git({ repoRoot, args, allowFailure = false }) {
  const result = spawnSync("git", args, { cwd: repoRoot, encoding: "utf8" });
  if (!allowFailure && result.status !== 0) {
    throw new Error(`git ${args.join(" ")} failed: ${result.stderr}`);
  }
  return result;
}

function readUtf8(file) {
  return fs.readFileSync(file, "utf8");
}

function readJson(file) {
  return JSON.parse(readUtf8(file));
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
