import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const vagueUpdateTriggerPattern = /^(?:always|as needed|periodically|regularly|tbd|todo|when necessary)[.!]?$/i;
const roleRules = new Map([
  ["implementation-record", { authority: "optional", values: ["supporting"], statuses: ["superseded"] }],
  ["proposal", { authority: "forbidden", values: [], statuses: ["closed", "rejected", "superseded"] }],
  ["reference", { authority: "required", values: ["normative", "supporting"], statuses: ["superseded"] }],
  ["review", { authority: "optional", values: ["supporting"], statuses: ["superseded"] }],
  ["routing", { authority: "forbidden", values: [], statuses: ["closed", "superseded"] }],
  ["specification", { authority: "required", values: ["normative"], statuses: [] }],
]);
const allowedStatuses = ["closed", "rejected", "superseded"];

if (isMainModule()) {
  const repoRoot = process.cwd();
  const requestedPaths = process.argv.slice(2);
  const files = requestedPaths.length > 0
    ? documentationMarkdownPaths(requestedPaths)
    : changedDocumentationMarkdownPaths({
        repoRoot,
        baseSha: process.env.DOC_FRONTMATTER_BASE_SHA,
        headSha: process.env.DOC_FRONTMATTER_HEAD_SHA,
      });
  const documents = files.map((file) => ({
    file,
    text: fs.readFileSync(path.resolve(repoRoot, file), "utf8"),
  }));
  const result = validateDocumentationFrontmatter(documents);

  if (!result.valid) {
    const message = [
      "Update the frontmatter in each listed document before merging; document roles, authority, lifecycle, and update triggers keep documentation aligned with project state.",
      ...result.errors.map((error) => `- ${error}`),
    ].join("\n");
    if (process.env.GITHUB_ACTIONS === "true") {
      console.error(renderGitHubErrorAnnotation(message));
    }
    console.error(message);
    process.exit(1);
  }

  console.log(`Documentation frontmatter is valid in ${documents.length} changed file(s).`);
}

export function validateDocumentationFrontmatter(documents) {
  const errors = documents.flatMap(validateDocumentFrontmatter);
  return {
    errors,
    valid: errors.length === 0,
  };
}

export function validateDocumentFrontmatter({ file, text }) {
  const frontmatter = markdownFrontmatter(text);
  if (frontmatter === undefined) {
    return [
      `${file}: add YAML frontmatter at the start of the document with one role: field and one update-when: field`,
    ];
  }
  if (!frontmatter.closed) {
    return [
      `${file}: close the opening YAML frontmatter with --- before the document body`,
    ];
  }

  return [
    ...validateRole({ file, frontmatter }),
    ...validateAuthority({ file, frontmatter }),
    ...validateStatus({ file, frontmatter }),
    ...validateUpdateTrigger({ file, frontmatter }),
    ...validateLegacyBodyStatus({ file, text, frontmatter }),
  ];
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
      "Pass changed documentation paths, or set DOC_FRONTMATTER_BASE_SHA and DOC_FRONTMATTER_HEAD_SHA to select the change range.",
    );
  }
  if (/^0+$/.test(baseSha)) {
    throw new Error(
      "DOC_FRONTMATTER_BASE_SHA cannot be the all-zero revision; pass the changed documentation paths explicitly.",
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

export function markdownFrontmatter(text) {
  const lines = text.split("\n");
  if (lines[0]?.trim() !== "---") {
    return undefined;
  }
  const closingIndex = lines.findIndex((line, index) => index > 0 && line.trim() === "---");
  if (closingIndex === -1) {
    return { closed: false, endLine: lines.length, text: lines.slice(1).join("\n") };
  }
  return {
    closed: true,
    endLine: closingIndex + 1,
    text: lines.slice(1, closingIndex).join("\n"),
  };
}

export function frontmatterField(frontmatter, name) {
  const matches = [
    ...frontmatter.text.matchAll(new RegExp(`^${name}:[ \\t]*(.*?)[ \\t]*$`, "gm")),
  ];
  return matches.map((match) => ({
    line: lineNumberAt(frontmatter.text, match.index) + 1,
    parsed: parseYamlScalar(match[1].trim()),
  }));
}

function validateRole({ file, frontmatter }) {
  const fields = frontmatterField(frontmatter, "role");
  const structuralErrors = validateSingleLineField({
    file,
    fields,
    name: "role",
    missingAction: "add exactly one role: field so readers know why the document should be opened",
    duplicateAction: "keep exactly one role: field so the document has one purpose",
  });
  if (structuralErrors.length > 0) {
    return structuralErrors;
  }

  const role = fields[0].parsed.value;
  if (!roleRules.has(role)) {
    return [
      `${file}:${fields[0].line}: replace unsupported role "${role}" with one of: ${[...roleRules.keys()].join(", ")}`,
    ];
  }
  return [];
}

function validateAuthority({ file, frontmatter }) {
  const roles = frontmatterField(frontmatter, "role");
  if (roles.length !== 1 || !roles[0].parsed.valid || !roleRules.has(roles[0].parsed.value)) {
    return [];
  }

  const role = roles[0].parsed.value;
  const rule = roleRules.get(role);
  const fields = frontmatterField(frontmatter, "authority");
  if (fields.length > 1) {
    return [
      `${file}:${fields[1].line}: keep at most one authority: field so the document makes one authority claim`,
    ];
  }
  if (fields.length === 1 && !fields[0].parsed.valid) {
    return [
      `${file}:${fields[0].line}: use a single-line plain or quoted YAML scalar for authority:`,
    ];
  }
  if (rule.authority === "forbidden" && fields.length === 1) {
    return [
      `${file}:${fields[0].line}: remove authority from role "${role}"; this role does not make an authority claim`,
    ];
  }
  if (rule.authority === "required" && fields.length === 0) {
    return [
      `${file}: add authority: ${rule.values.join(" or ")} for role "${role}" so readers know how its claims may be used`,
    ];
  }
  if (fields.length === 1 && !rule.values.includes(fields[0].parsed.value)) {
    return [
      `${file}:${fields[0].line}: replace authority "${fields[0].parsed.value}" with ${rule.values.join(" or ")} for role "${role}"`,
    ];
  }
  return [];
}

function validateStatus({ file, frontmatter }) {
  const fields = frontmatterField(frontmatter, "status");
  if (fields.length === 0) {
    return [];
  }
  if (fields.length > 1) {
    return [
      `${file}:${fields[1].line}: keep at most one status: field so the exceptional lifecycle state is unambiguous`,
    ];
  }
  if (!fields[0].parsed.valid) {
    return [
      `${file}:${fields[0].line}: use a single-line plain or quoted YAML scalar for status:`,
    ];
  }
  if (!allowedStatuses.includes(fields[0].parsed.value)) {
    return [
      `${file}:${fields[0].line}: remove status "${fields[0].parsed.value}" or replace it with one of: ${allowedStatuses.join(", ")}; status records only exceptional lifecycle states`,
    ];
  }
  const roles = frontmatterField(frontmatter, "role");
  if (roles.length === 1 && roles[0].parsed.valid && roleRules.has(roles[0].parsed.value)) {
    const role = roles[0].parsed.value;
    if (!roleRules.get(role).statuses.includes(fields[0].parsed.value)) {
      return [
        `${file}:${fields[0].line}: remove status "${fields[0].parsed.value}" from role "${role}" or reclassify the document; this lifecycle state is not valid for the role`,
      ];
    }
  }
  return [];
}

function validateUpdateTrigger({ file, frontmatter }) {
  const fields = frontmatterField(frontmatter, "update-when");
  const legacyFields = frontmatterField(frontmatter, "review-when");
  if (legacyFields.length > 0) {
    return [
      `${file}:${legacyFields[0].line}: replace review-when: with update-when: so the field identifies when project changes can make the document stale`,
    ];
  }
  const structuralErrors = validateSingleLineField({
    file,
    fields,
    name: "update-when",
    missingAction: "add exactly one update-when: field naming the project-state change that can make this document stale",
    duplicateAction: "keep exactly one update-when: field so the document has one update contract",
  });
  if (structuralErrors.length > 0) {
    return structuralErrors;
  }

  const trigger = fields[0].parsed.value;
  if (trigger === "") {
    return [
      `${file}:${fields[0].line}: name the project-state change after update-when: so maintainers can tell when to update this document`,
    ];
  }
  if (vagueUpdateTriggerPattern.test(trigger)) {
    return [
      `${file}:${fields[0].line}: replace vague update trigger "${trigger}" with a concrete project-state change`,
    ];
  }
  return [];
}

function validateLegacyBodyStatus({ file, text, frontmatter }) {
  const lines = text.split("\n").slice(frontmatter.endLine);
  let fenced = false;
  for (const [index, line] of lines.entries()) {
    if (/^\s*(```|~~~)/.test(line)) {
      fenced = !fenced;
      continue;
    }
    if (!fenced && /^Status:\s*/.test(line)) {
      const lineNumber = frontmatter.endLine + index + 1;
      return [
        `${file}:${lineNumber}: remove the legacy Status: line; use role, authority, and exceptional frontmatter status instead`,
      ];
    }
  }
  return [];
}

function validateSingleLineField({ file, fields, name, missingAction, duplicateAction }) {
  if (fields.length === 0) {
    return [`${file}: ${missingAction}`];
  }
  if (fields.length > 1) {
    return [`${file}:${fields[1].line}: ${duplicateAction}`];
  }
  if (!fields[0].parsed.valid) {
    return [
      `${file}:${fields[0].line}: use a single-line plain or quoted YAML scalar for ${name}:`,
    ];
  }
  return [];
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

export function renderGitHubErrorAnnotation(message) {
  return `::error title=Invalid documentation frontmatter::${escapeGitHubAnnotationMessage(message)}`;
}

function escapeGitHubAnnotationMessage(value) {
  return value
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A");
}
