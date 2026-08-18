import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  frontmatterField,
  markdownFrontmatter,
} from "./check-doc-frontmatter.mjs";

const manifestPath = "docs/reference/proposal-target-readiness/manifest.json";
const manifestSchemaPath = "docs/reference/proposal-target-readiness/manifest.schema.json";
const targetSchemaPath = "docs/reference/proposal-target-readiness/target.schema.json";
const validStates = new Set(["ready", "blocked"]);
const validTargetKinds = new Set(["proposal", "proposal-section", "no-target"]);

if (isMainModule()) {
  const repoRoot = process.cwd();
  const command = process.argv[2] ?? "validate";
  const metadataPath = process.argv[3];
  const result = runCommand({ repoRoot, command, metadataPath });
  if (!result.valid) {
    console.error(result.summary);
    for (const error of result.errors) {
      console.error(`- ${error}`);
    }
    process.exit(1);
  }
  console.log(result.summary);
}

export function runCommand({ repoRoot, command, metadataPath }) {
  if (command !== "validate") {
    return failure("Use `validate` with an optional target metadata path.", [`unknown command: ${command}`]);
  }

  const manifest = readJson(path.resolve(repoRoot, manifestPath));
  const manifestSchema = readJson(path.resolve(repoRoot, manifestSchemaPath));
  const targetSchema = readJson(path.resolve(repoRoot, targetSchemaPath));
  const errors = [
    ...validateManifestShape({ manifest, schema: manifestSchema }),
    ...validateTargetSchema(targetSchema),
    ...validateCatalogCoverage({ repoRoot, manifest }),
  ];

  if (metadataPath !== undefined) {
    const metadata = readJson(path.resolve(repoRoot, metadataPath));
    errors.push(
      ...validateTargetShape({ metadata, schema: targetSchema }),
      ...validateTargetReadiness({ repoRoot, manifest, metadata }),
    );
  }

  return errors.length === 0
    ? success(metadataPath === undefined
      ? "Proposal target readiness manifest matches the proposal catalog."
      : "Proposal target metadata is ready for implementation.")
    : failure("Update the proposal target readiness metadata before implementation; blocked or stale targets must not create handoff prompts.", errors);
}

export function validateManifestShape({ manifest, schema }) {
  const errors = [];
  if (schema?.$id !== "https://veln-lang.invalid/schemas/proposal-target-readiness-manifest.schema.json") {
    errors.push("manifest.schema.json: keep the stable readiness manifest $id");
  }
  if (!Array.isArray(manifest?.entries)) {
    return ["manifest.json: add an entries array"];
  }
  const seen = new Set();
  for (const [index, entry] of manifest.entries.entries()) {
    const label = `manifest.entries[${index}]`;
    errors.push(...validateManifestEntry({ label, entry }));
    const key = `${entry.proposal_path}${entry.proposal_anchor}`;
    if (seen.has(key)) {
      errors.push(`${label}: remove duplicate readiness entry for ${entry.proposal_path}${entry.proposal_anchor}`);
    }
    seen.add(key);
  }
  return errors;
}

export function validateTargetShape({ metadata, schema }) {
  const errors = [];
  if (schema?.$id !== "https://veln-lang.invalid/schemas/proposal-target-readiness-target.schema.json") {
    errors.push("target.schema.json: keep the stable target metadata $id");
  }
  if (!metadata || typeof metadata !== "object" || Array.isArray(metadata)) {
    return ["target metadata: use an object"];
  }
  for (const field of ["proposal_path", "proposal_anchor", "default_branch", "base_commit", "prerequisites", "target_kind"]) {
    if (!(field in metadata)) {
      errors.push(`target metadata: add required field ${field}`);
    }
  }
  if (!validTargetKinds.has(metadata.target_kind)) {
    errors.push(`target metadata: replace target_kind "${metadata.target_kind}" with proposal, proposal-section, or no-target`);
  }
  if (!isRepoRelativeMarkdown(metadata.proposal_path, "docs/proposals/")) {
    errors.push(`target metadata: use a repository-relative proposal_path under docs/proposals/`);
  }
  if (!validAnchor(metadata.proposal_anchor)) {
    errors.push("target metadata: use a nonempty Markdown heading anchor");
  }
  if (!nonemptyString(metadata.default_branch)) {
    errors.push("target metadata: name the default branch used to issue the handoff");
  }
  if (!/^[0-9a-f]{7,40}$/.test(metadata.base_commit ?? "")) {
    errors.push("target metadata: use the exact hexadecimal base commit");
  }
  if (!Array.isArray(metadata.prerequisites)) {
    errors.push("target metadata: add a prerequisites array");
  } else {
    const seen = new Set();
    for (const [index, prerequisite] of metadata.prerequisites.entries()) {
      if (!isRepoRelativeMarkdown(prerequisite, "docs/proposals/")) {
        errors.push(`target metadata.prerequisites[${index}]: use a repository-relative proposal path under docs/proposals/`);
      }
      if (seen.has(prerequisite)) {
        errors.push(`target metadata.prerequisites[${index}]: remove duplicate prerequisite ${prerequisite}`);
      }
      seen.add(prerequisite);
    }
  }
  return errors;
}

export function validateCatalogCoverage({ repoRoot, manifest }) {
  const catalogText = fs.readFileSync(path.resolve(repoRoot, "docs/proposals/README.md"), "utf8");
  const catalogEntries = proposalCatalogEntries({ catalogText, repoRoot });
  const manifestEntries = Array.isArray(manifest?.entries) ? manifest.entries : [];
  return compareCatalogAndManifest({ catalogEntries, manifestEntries });
}

export function validateTargetReadiness({ repoRoot, manifest, metadata }) {
  if (metadata?.target_kind === "no-target") {
    const ready = manifest.entries.filter((entry) => entry.state === "ready");
    return ready.length === 0
      ? []
      : [`target metadata: remove no-target handoff; Ready still contains ${ready[0].proposal_path}${ready[0].proposal_anchor}`];
  }

  const targetKey = `${metadata?.proposal_path}${metadata?.proposal_anchor}`;
  const entry = manifest.entries.find((candidate) => `${candidate.proposal_path}${candidate.proposal_anchor}` === targetKey);
  if (entry === undefined) {
    return [`target metadata: select a proposal listed under Ready; ${targetKey} is absent from the readiness manifest`];
  }
  const errors = [];
  if (entry.state !== "ready") {
    errors.push(`${targetKey}: select a Ready prerequisite before this blocked target`);
  }
  const expectedPrerequisites = [...entry.prerequisites].sort();
  const actualPrerequisites = [...(metadata.prerequisites ?? [])].sort();
  if (JSON.stringify(expectedPrerequisites) !== JSON.stringify(actualPrerequisites)) {
    errors.push(`${targetKey}: set prerequisites to the manifest entry exactly`);
  }
  for (const prerequisite of entry.prerequisites) {
    if (fs.existsSync(path.resolve(repoRoot, prerequisite))) {
      errors.push(`${targetKey}: complete ${prerequisite} before issuing this target`);
    }
    const record = `docs/reference/implemented-proposals/${path.basename(prerequisite)}`;
    if (!fs.existsSync(path.resolve(repoRoot, record))) {
      errors.push(`${targetKey}: link the completed implementation record for ${prerequisite}`);
    }
  }
  if (hasGitObject({ repoRoot, revision: metadata.base_commit })) {
    if (!isAncestor({ repoRoot, ancestor: metadata.base_commit, descendant: metadata.default_branch })) {
      errors.push(`${targetKey}: regenerate the target from ${metadata.default_branch}; the declared base is not on that branch`);
    }
    const mergeBase = gitOutput({ repoRoot, args: ["merge-base", "HEAD", metadata.default_branch] });
    if (mergeBase !== undefined && mergeBase.trim() !== metadata.base_commit) {
      errors.push(`${targetKey}: regenerate the target; the working branch merge base is ${mergeBase.trim()}`);
    }
  }
  return errors;
}

function validateManifestEntry({ label, entry }) {
  const errors = [];
  if (!isRepoRelativeMarkdown(entry?.proposal_path, "docs/proposals/")) {
    errors.push(`${label}: use a repository-relative proposal_path under docs/proposals/`);
  }
  if (!validAnchor(entry?.proposal_anchor)) {
    errors.push(`${label}: use a nonempty proposal heading anchor`);
  }
  if (!validStates.has(entry?.state)) {
    errors.push(`${label}: replace state "${entry?.state}" with ready or blocked`);
  }
  if (!Array.isArray(entry?.prerequisites)) {
    errors.push(`${label}: add a prerequisites array`);
  } else {
    const seen = new Set();
    for (const [index, prerequisite] of entry.prerequisites.entries()) {
      if (!isRepoRelativeMarkdown(prerequisite, "docs/proposals/")) {
        errors.push(`${label}.prerequisites[${index}]: use a repository-relative proposal path under docs/proposals/`);
      }
      if (seen.has(prerequisite)) {
        errors.push(`${label}.prerequisites[${index}]: remove duplicate prerequisite ${prerequisite}`);
      }
      seen.add(prerequisite);
    }
  }
  return errors;
}

function validateTargetSchema(schema) {
  const errors = [];
  const kinds = schema?.properties?.target_kind?.enum ?? [];
  for (const kind of validTargetKinds) {
    if (!kinds.includes(kind)) {
      errors.push(`target.schema.json: include target_kind enum value ${kind}`);
    }
  }
  if (schema?.additionalProperties !== false) {
    errors.push("target.schema.json: reject unknown target metadata fields");
  }
  return errors;
}

function compareCatalogAndManifest({ catalogEntries, manifestEntries }) {
  const errors = [];
  const catalogByKey = new Map(catalogEntries.map((entry) => [`${entry.proposal_path}${entry.proposal_anchor}`, entry]));
  const manifestByKey = new Map(manifestEntries.map((entry) => [`${entry.proposal_path}${entry.proposal_anchor}`, entry]));
  for (const [key, catalogEntry] of catalogByKey) {
    const manifestEntry = manifestByKey.get(key);
    if (manifestEntry === undefined) {
      errors.push(`manifest.json: add ${key} from the ${catalogEntry.state} catalog`);
      continue;
    }
    if (manifestEntry.state !== catalogEntry.state) {
      errors.push(`${key}: set state to ${catalogEntry.state} to match docs/proposals/README.md`);
    }
  }
  for (const [key] of manifestByKey) {
    if (!catalogByKey.has(key)) {
      errors.push(`manifest.json: remove ${key}; it is not listed under Ready or Blocked`);
    }
  }
  return errors;
}

function proposalCatalogEntries({ catalogText, repoRoot }) {
  const sections = { ready: sectionText(catalogText, "Ready"), blocked: sectionText(catalogText, "Blocked") };
  return Object.entries(sections).flatMap(([state, text]) => markdownLinks(text).map((link) => {
    const [file, rawAnchor = ""] = link.split("#", 2);
    const proposalPath = `docs/proposals/${file}`;
    const proposalText = fs.readFileSync(path.resolve(repoRoot, proposalPath), "utf8");
    const frontmatter = markdownFrontmatter(proposalText);
    const role = frontmatter === undefined ? undefined : frontmatterField(frontmatter, "role")[0]?.parsed.value;
    if (role !== "proposal") {
      return { proposal_path: proposalPath, proposal_anchor: rawAnchor === "" ? firstHeadingAnchor(proposalText) : `#${rawAnchor}`, state, prerequisites: [] };
    }
    return { proposal_path: proposalPath, proposal_anchor: rawAnchor === "" ? firstHeadingAnchor(proposalText) : `#${rawAnchor}`, state, prerequisites: [] };
  }));
}

function sectionText(text, heading) {
  const startMatch = new RegExp(`^## ${escapeRegExp(heading)}\\s*$`, "m").exec(text);
  if (startMatch === null) {
    return "";
  }
  const start = startMatch.index + startMatch[0].length;
  const next = /^##\s+/m.exec(text.slice(start));
  return next === null ? text.slice(start) : text.slice(start, start + next.index);
}

function markdownLinks(text) {
  return [...text.matchAll(/\[[^\]\n]+\]\(([^)\n\s]+)(?:\s+"[^"]*")?\)/g)]
    .map((match) => match[1])
    .filter((target) => !target.startsWith("../") && (target.endsWith(".md") || target.includes(".md#")));
}

function firstHeadingAnchor(text) {
  const body = stripFrontmatter(text);
  const heading = /^#\s+(.+?)\s*$/m.exec(body)?.[1] ?? "";
  return `#${slug(heading)}`;
}

function stripFrontmatter(text) {
  if (!text.startsWith("---\n")) {
    return text;
  }
  const end = text.indexOf("\n---\n", 4);
  return end === -1 ? text : text.slice(end + 5);
}

function slug(value) {
  return value.toLowerCase().replaceAll(/`/g, "").replaceAll(/[^a-z0-9]+/g, "-").replaceAll(/^-|-$/g, "");
}

function isRepoRelativeMarkdown(value, prefix) {
  return nonemptyString(value) && value.startsWith(prefix) && value.endsWith(".md") && !value.startsWith("/") && !value.includes("..");
}

function validAnchor(value) {
  return nonemptyString(value) && /^#[a-z0-9][a-z0-9-]*$/.test(value);
}

function nonemptyString(value) {
  return typeof value === "string" && value.length > 0;
}

function hasGitObject({ repoRoot, revision }) {
  return spawnSync("git", ["cat-file", "-e", revision], { cwd: repoRoot }).status === 0;
}

function isAncestor({ repoRoot, ancestor, descendant }) {
  return spawnSync("git", ["merge-base", "--is-ancestor", ancestor, descendant], { cwd: repoRoot }).status === 0;
}

function gitOutput({ repoRoot, args }) {
  const result = spawnSync("git", args, { cwd: repoRoot, encoding: "utf8" });
  return result.status === 0 ? result.stdout : undefined;
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}

function success(summary) {
  return { valid: true, summary, errors: [] };
}

function failure(summary, errors) {
  return { valid: false, summary, errors };
}
