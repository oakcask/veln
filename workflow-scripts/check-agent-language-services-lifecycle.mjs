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
  return failure("Use validate, validate-range, or write-structural-skeleton.", [`unknown command: ${command}`]);
}

export function validateContent({ repoRoot, sourcePath = umbrellaPath, authorityFile = authorityPath }) {
  const source = readUtf8(path.resolve(repoRoot, sourcePath));
  const authority = readJson(path.resolve(repoRoot, authorityFile));
  const parsedRoots = parseMarkdownSource({ source });
  const errors = validateAuthorityShape({ authority, parsedRoots, sourcePath });
  return errors.length === 0
    ? success("Agent language-services lifecycle authority matches the umbrella proposal.")
    : failure("Agent language-services lifecycle authority is stale or incomplete.", errors);
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
  return changedPaths
    .filter((file) => file.startsWith("docs/reference/agent-language-services-lifecycle/"))
    .map((file) => `${file}: frozen lifecycle artifacts are immutable after bootstrap`);
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
