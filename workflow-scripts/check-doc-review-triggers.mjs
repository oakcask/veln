import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const vagueTriggerPattern = /^(?:always|as needed|periodically|regularly|tbd|todo|when necessary)[.!]?$/i;

if (isMainModule()) {
  const repoRoot = process.cwd();
  const requestedPaths = process.argv.slice(2);
  const files = requestedPaths.length > 0
    ? documentationMarkdownPaths(requestedPaths)
    : changedDocumentationMarkdownPaths({
        repoRoot,
        baseSha: process.env.DOC_REVIEW_BASE_SHA,
        headSha: process.env.DOC_REVIEW_HEAD_SHA,
      });
  const documents = files.map((file) => ({
    file,
    text: fs.readFileSync(path.resolve(repoRoot, file), "utf8"),
  }));
  const result = validateDocumentationReviewTriggers(documents);

  if (!result.valid) {
    const message = [
      "Add a concrete review-when: condition to each listed document before merging; review triggers keep documentation tied to the project changes that can make it stale.",
      ...result.errors.map((error) => `- ${error}`),
    ].join("\n");
    if (process.env.GITHUB_ACTIONS === "true") {
      console.error(`::error title=Missing documentation review trigger::${escapeGitHubAnnotation(message)}`);
    }
    console.error(message);
    process.exit(1);
  }

  console.log(`Documentation review triggers are present in ${documents.length} changed file(s).`);
}

export function validateDocumentationReviewTriggers(documents) {
  const errors = documents.flatMap(validateDocumentReviewTrigger);
  return {
    errors,
    valid: errors.length === 0,
  };
}

export function validateDocumentReviewTrigger({ file, text }) {
  const frontmatter = markdownFrontmatter(text);
  if (frontmatter === undefined) {
    return [
      `${file}: add YAML frontmatter at the start of the document with one review-when: field`,
    ];
  }
  if (!frontmatter.closed) {
    return [
      `${file}: close the opening YAML frontmatter with --- before the document body`,
    ];
  }

  const matches = [...frontmatter.text.matchAll(/^review-when:[ \t]*(.*?)[ \t]*$/gm)];

  if (matches.length === 0) {
    return [
      `${file}: add exactly one review-when: field to the YAML frontmatter naming the project-state change that requires this document to be checked again`,
    ];
  }
  if (matches.length > 1) {
    return [
      `${file}:${lineNumberAt(frontmatter.text, matches[1].index) + 1}: keep exactly one review-when: field so the document has one review contract`,
    ];
  }

  const parsed = parseYamlScalar(matches[0][1].trim());
  if (!parsed.valid) {
    return [
      `${file}:${lineNumberAt(frontmatter.text, matches[0].index) + 1}: use a single-line plain or quoted YAML scalar for review-when:`,
    ];
  }
  const trigger = parsed.value;
  if (trigger === "") {
    return [
      `${file}:${lineNumberAt(frontmatter.text, matches[0].index) + 1}: name the project-state change after review-when: so maintainers can tell when this document may be stale`,
    ];
  }
  if (vagueTriggerPattern.test(trigger)) {
    return [
      `${file}:${lineNumberAt(frontmatter.text, matches[0].index) + 1}: replace vague review trigger "${trigger}" with a concrete project-state change`,
    ];
  }

  return [];
}

export function documentationMarkdownPaths(files) {
  return [...new Set(files
    .map((file) => file.replaceAll("\\", "/"))
    .filter((file) => file.startsWith("docs/") && file.toLowerCase().endsWith(".md")))]
    .sort();
}

export function changedDocumentationMarkdownPaths({ repoRoot, baseSha, headSha }) {
  if (!baseSha || !headSha) {
    throw new Error(
      "Pass changed documentation paths, or set DOC_REVIEW_BASE_SHA and DOC_REVIEW_HEAD_SHA to select the change range.",
    );
  }
  if (/^0+$/.test(baseSha)) {
    throw new Error(
      "DOC_REVIEW_BASE_SHA cannot be the all-zero revision; pass the changed documentation paths explicitly.",
    );
  }

  const result = spawnSync(
    "git",
    ["diff", "--name-only", "--diff-filter=ACMR", "-z", baseSha, headSha, "--", "docs"],
    { cwd: repoRoot, encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(
      `Unable to list changed documentation between ${baseSha} and ${headSha}: ${result.stderr.trim()}`,
    );
  }

  return documentationMarkdownPaths(result.stdout.split("\0").filter(Boolean));
}

function markdownFrontmatter(text) {
  const lines = text.split("\n");
  if (lines[0]?.trim() !== "---") {
    return undefined;
  }
  const closingIndex = lines.findIndex((line, index) => index > 0 && line.trim() === "---");
  if (closingIndex === -1) {
    return { closed: false, text: lines.slice(1).join("\n") };
  }
  return {
    closed: true,
    text: lines.slice(1, closingIndex).join("\n"),
  };
}

function parseYamlScalar(source) {
  if (source.startsWith('"') || source.endsWith('"')) {
    try {
      const value = JSON.parse(source);
      return { valid: typeof value === "string", value };
    } catch {
      return { valid: false, value: "" };
    }
  }
  if (source.startsWith("'") || source.endsWith("'")) {
    if (!source.startsWith("'") || !source.endsWith("'")) {
      return { valid: false, value: "" };
    }
    return { valid: true, value: source.slice(1, -1).replaceAll("''", "'") };
  }
  if (/^[>|]/.test(source)) {
    return { valid: false, value: "" };
  }
  return { valid: true, value: source };
}

function lineNumberAt(text, index) {
  return text.slice(0, index).split("\n").length;
}

function isMainModule() {
  return process.argv[1] !== undefined && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
}

function escapeGitHubAnnotation(value) {
  return value
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A")
    .replaceAll(":", "%3A")
    .replaceAll(",", "%2C");
}
