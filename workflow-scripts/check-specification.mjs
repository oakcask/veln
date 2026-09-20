import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  frontmatterField,
  markdownFrontmatter,
  validateDocumentationFrontmatter,
} from "./check-doc-frontmatter.mjs";

const concerns = ["usage", "behavior", "limits"];
const referenceHeading = /^(?:references|evidence|verification|implementation and tests|executable evidence)(?:\s|$)/i;

export function specificationDocuments(root) {
  return filesUnder(path.join(root, "docs/specification"), ".md").map((file) => ({
    file: path.relative(root, file).replaceAll(path.sep, "/"),
    text: fs.readFileSync(file, "utf8"),
  }));
}

// Use repository identifiers, not guesses based on naming style. Public field
// names and prose that happens to contain "test" are not evidence inventories.
export function repositoryEvidenceNames(root) {
  const names = new Set();
  for (const file of filesUnder(path.join(root, "examples/specification"), "case.toml")) {
    names.add(path.basename(path.dirname(file)));
  }
  const rustFiles = ["crates", "tools"].flatMap((directory) =>
    filesUnder(path.join(root, directory), ".rs")
  );
  for (const file of rustFiles) {
    const text = fs.readFileSync(file, "utf8");
    const testFunction = /#\[(?:test|tokio::test)(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g;
    for (const match of text.matchAll(testFunction)) names.add(match[1]);
  }
  return names;
}

export function validateSpecifications(documents, evidenceNames = new Set()) {
  const errors = [...validateDocumentationFrontmatter(documents).errors];
  if (documents.length === 0) {
    errors.push("docs/specification: restore the specification Markdown or run this check from the repository root; an empty scan cannot verify coverage");
  }
  const paths = new Set(documents.map(({ file }) => file));
  for (const { file, text } of documents) {
    if (file.endsWith("-full.md") && paths.has(file.replace(/-full\.md$/, ".md"))) {
      errors.push(`${file}: merge this summary/detail pair into one subject authority and update its links; readers must not compare duplicate specifications`);
    }
    const metadata = markdownFrontmatter(text);
    if (!metadata?.closed) continue;
    const role = frontmatterField(metadata, "role")[0]?.parsed.value;
    const fields = frontmatterField(metadata, "specification-coverage");
    if (role === "routing") {
      if (fields.length) errors.push(`${file}: remove specification-coverage from this routing page; put behavior and its coverage in the destination specification`);
      continue;
    }
    if (role !== "specification") {
      errors.push(`${file}: use role: specification for current behavior or role: routing for discovery; relocate supporting material to docs/reference`);
      continue;
    }
    if (fields.length !== 1 || !fields[0]?.parsed.valid) {
      errors.push(`${file}: add one specification-coverage map for usage, behavior, and limits; point each concern to its explanatory heading (see docs/reference/documentation-authoring.md)`);
      continue;
    }
    const sections = markdownSections(text.split("\n").slice(metadata.endLine).join("\n"));
    const coverage = new Map();
    for (const entry of fields[0].parsed.value.split(";")) {
      const match = entry.trim().match(/^(usage|behavior|limits)\s*=\s*#([^\s;]+)$/);
      if (!match || coverage.has(match[1])) {
        errors.push(`${file}: replace invalid or duplicate coverage entry "${entry.trim()}" with concern=#heading; each concern needs one unambiguous destination`);
      } else coverage.set(match[1], match[2]);
    }
    for (const concern of concerns) {
      const anchor = coverage.get(concern);
      const section = sections.find((item) => item.anchor === anchor);
      if (!section) {
        errors.push(`${file}: map ${concern} to an existing explanatory heading; ${anchor ? `#${anchor} does not resolve` : "the concern has no destination"}`);
      } else if (referenceHeading.test(section.title) || !hasExplanation(explanationBody(section, sections), evidenceNames)) {
        errors.push(`${file}#${anchor}: explain ${concern} in the specification itself; links, test names, fixtures, and verification commands alone do not describe the contract`);
      }
    }
    // Inspect each section's own text once (not its descendants). Evidence may
    // support a rule anywhere, but a list led by test identifiers belongs in
    // References and must not stand in for the rule itself.
    for (const section of sections) {
      if (section.inReferences) continue;
      const prose = withoutFences(section.ownBody);
      for (const paragraph of prose.split(/\n\s*\n/)) {
        const unwrapped = paragraph.replace(/\s+/g, " ").trim();
        if (/^(?:(?:The|These|Those) )?(?:checked|executable|focused|adjacent|same adjacent|same checked) (?:cases?|tests?|fixtures?|examples?)\b/i.test(unwrapped) || evidenceIdentifier(unwrapped, evidenceNames)) {
          errors.push(`${file}#${section.anchor}: rewrite this test-coverage narrative as the observable rule or move its supporting evidence to References: ${unwrapped}`);
        }
      }
      for (const line of prose.split("\n")) {
        if (evidenceListItem(line, evidenceNames) || evidenceTableRow(line, evidenceNames)) {
          errors.push(`${file}#${section.anchor}: replace this evidence-led item with its observable rule, or move the supporting reference to References: ${line.trim()}`);
        }
      }
    }
  }
  return { valid: errors.length === 0, errors };
}

function explanationBody(section, sections) {
  return sections
    .filter((item) => item.index >= section.index && item.index < section.end && !item.inReferences)
    .map((item) => item.ownBody)
    .join("\n");
}

export function markdownSections(text) {
  const lines = text.split("\n");
  const headings = [];
  const counts = new Map();
  let fence;
  for (const [index, line] of lines.entries()) {
    const marker = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (marker) {
      if (!fence) fence = marker[1];
      else if (marker[1][0] === fence[0] && marker[1].length >= fence.length) fence = undefined;
      continue;
    }
    if (fence) continue;
    const match = line.match(/^(#{1,6})\s+(.+?)\s*#*$/);
    if (!match) continue;
    const title = match[2];
    const base = title.toLowerCase()
      .replace(/`([^`]*)`/g, "$1")
      .replace(/<[^>]+>/g, "")
      .replace(/[^\p{Letter}\p{Number} _-]/gu, "")
      .trim()
      .replace(/[ \t]+/g, "-");
    const count = counts.get(base) ?? 0;
    counts.set(base, count + 1);
    headings.push({ title, level: match[1].length, anchor: count ? `${base}-${count}` : base, index });
  }
  return headings.map((heading, index) => {
    const end = headings.slice(index + 1).find((next) => next.level <= heading.level)?.index ?? lines.length;
    const parents = headings.slice(0, index).filter((candidate, i) => candidate.level < heading.level && !headings.slice(i + 1, index).some((next) => next.level <= candidate.level));
    return {
      ...heading,
      end,
      ownBody: lines.slice(heading.index + 1, headings[index + 1]?.index ?? lines.length).join("\n"),
      inReferences: [heading, ...parents].some(({ title }) => referenceHeading.test(title)),
    };
  });
}

function hasExplanation(body, names) {
  return withoutFences(body).replace(/<!--[\s\S]*?-->/g, "").split("\n").some((line) => {
    if (/^\s*(?:#|\[.*\]:)/.test(line) || evidenceListItem(line, names) || evidenceTableRow(line, names) || evidenceIdentifier(line.trim(), names)) return false;
    const plain = line.replace(/!?\[[^\]]*\]\([^)]*\)/g, "").replace(/`[^`]*`/g, "").replace(/^\s*(?:[-*+]|\d+[.)])\s+/, "").replace(/[|:*_>-]/g, " ").trim();
    if (/^(?:(?:see|use|read|run|checked by|verified by|evidence)[ .,:;]*|(?:node|cargo|bash|pnpm)\s.*)$/i.test(plain)) return false;
    // A naked identifier is not an explanation; no word-count threshold or
    // preferred normative verbs are imposed on real sentences and tables.
    return /\p{Letter}/u.test(plain) && !/^[\p{Letter}\p{Number}_.\/-]+[.,;:]?$/u.test(plain);
  });
}

function evidenceListItem(line, names) {
  const item = line.match(/^\s*(?:[-*+]|\d+[.)])\s+(.*)$/)?.[1];
  if (!item) return false;
  if (/^(?:`?(?:\.\.\/)*examples\/specification\/|\[[^\]]*\]\((?:\.\.\/)*examples\/specification\/)/.test(item)) return true;
  return evidenceIdentifier(item, names);
}

function evidenceTableRow(line, names) {
  const firstCell = line.match(/^\s*\|\s*([^|]+)\|/)?.[1]?.trim();
  return firstCell !== undefined && evidenceIdentifier(firstCell, names);
}

function evidenceIdentifier(text, names) {
  const identifier = text.match(/^(?:`([^`]+)`|([A-Za-z_][A-Za-z0-9_-]*))(?=\s|[:.,;]|$)/);
  return identifier !== null && names.has(identifier[1] ?? identifier[2]);
}

function withoutFences(text) {
  let fence;
  return text.split("\n").filter((line) => {
    const marker = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (marker) {
      if (!fence) fence = marker[1];
      else if (marker[1][0] === fence[0] && marker[1].length >= fence.length) fence = undefined;
      return false;
    }
    return !fence;
  }).join("\n");
}

function filesUnder(root, suffix) {
  if (!fs.existsSync(root)) return [];
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const file = path.join(root, entry.name);
    return entry.isDirectory() ? filesUnder(file, suffix) : entry.isFile() && file.endsWith(suffix) ? [file] : [];
  }).sort();
}

export function renderSpecificationFailure(errors) {
  return ["Repair the specification explanations and coverage destinations before merging; readers need usage, behavior, and limits rather than a test inventory.", ...errors.map((error) => `- ${error}`)].join("\n");
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = process.cwd();
  const documents = specificationDocuments(root);
  const result = validateSpecifications(documents, repositoryEvidenceNames(root));
  if (!result.valid) {
    console.error(renderSpecificationFailure(result.errors));
    process.exitCode = 1;
  } else console.log(`Specification explanations and coverage destinations checked in ${documents.length} Markdown file(s). Review semantic completeness against implementation and tests.`);
}
