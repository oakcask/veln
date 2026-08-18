import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const SOURCE_PATH = "docs/proposals/agent-language-services.md";
export const INVENTORY_PATH =
  "docs/proposals/agent-language-services-source-inventory.json";
export const SCHEMA_PATH =
  "docs/proposals/agent-language-services-migration-ledger.schema.json";

const LIFECYCLES = new Set(["current", "completed", "planned", "removed"]);
const CURRENT = /\b(?:current|currently|existing|implemented|specified|exposes?)\b/iu;
const COMPLETED = /\b(?:completed|historical|previously|shipped)\b/iu;
const PLANNED = /\b(?:planned|remain(?:s|ing)?|future|eventual|not yet|must|will|beyond|broader)\b/iu;
const REMOVED = /\b(?:obsolete|removed|deleted|superseded)\b/iu;
const MARKDOWN_DELIMITER = /[`*_#|\[\](){}<>+\-\\]/u;

if (isMainModule()) {
  const root = process.cwd();
  if (process.argv.includes("--write-inventory")) {
    const source = fs.readFileSync(path.join(root, SOURCE_PATH), "utf8");
    fs.writeFileSync(
      path.join(root, INVENTORY_PATH),
      `${JSON.stringify(buildInventory(source), null, 2)}\n`,
    );
    console.log(`Wrote ${INVENTORY_PATH}.`);
    process.exit(0);
  }
  let changed;
  if (process.argv.includes("--check-diff")) {
    const base = process.env.INVENTORY_BASE_SHA;
    const head = process.env.INVENTORY_HEAD_SHA ?? "HEAD";
    if (!base) {
      console.error(
        "Set INVENTORY_BASE_SHA before checking diff scope so the frozen proposal and executable MCP evidence cannot change unnoticed.",
      );
      process.exit(2);
    }
    changed = changedPaths(base, head, root);
  }
  const result = validateRepository(root, { changedPaths: changed });
  if (!result.valid) {
    console.error("Frozen agent-language-services inventory validation failed:");
    for (const error of result.errors) console.error(`- ${error}`);
    process.exit(1);
  }
  console.log("Frozen agent-language-services inventory is valid.");
}

export function validateRepository(root, options = {}) {
  const source = fs.readFileSync(path.join(root, SOURCE_PATH), "utf8");
  const inventory = JSON.parse(
    fs.readFileSync(path.join(root, INVENTORY_PATH), "utf8"),
  );
  const schema = JSON.parse(fs.readFileSync(path.join(root, SCHEMA_PATH), "utf8"));
  const errors = [
    ...validateInventory(source, inventory),
    ...validateLedgerSchema(schema),
  ];
  if (options.changedPaths !== undefined) {
    errors.push(...validateDiffScope(options.changedPaths));
  }
  return { errors, valid: errors.length === 0 };
}

export function buildInventory(source) {
  const records = extractRecords(source);
  const items = [];
  for (const [index, record] of records.entries()) {
    const id = `ALS-${String(index + 1).padStart(4, "0")}`;
    const classes = statedLifecycles(record.text);
    const base = {
      id,
      kind: record.kind,
      heading: record.heading,
      identity: namedIdentity(record.text),
      digest: digest(record.text),
      text: record.text,
    };
    if (isStableDomainCodeRecord(record.text)) {
      const children = splitStableDomainCodeRecord(id, record.heading, record.text);
      items.push({ ...base, child_count: children.length, children });
      continue;
    }
    if (record.parts !== undefined) {
      const children = record.parts.map((part, childIndex) => {
        const child = childRecord(
          `${id}.${childIndex + 1}`,
          record.heading,
          record.text,
          part.spans,
        );
        const childClasses = statedLifecycles(child.text);
        if (childClasses.size > 1) {
          delete child.lifecycle;
          child.children = splitMixedRecord(
            child.id,
            record.heading,
            child.text,
            childClasses,
          );
          child.child_count = child.children.length;
        }
        return child;
      });
      items.push({ ...base, child_count: children.length, children });
      continue;
    } else if (classes.size <= 1) {
      items.push({ ...base, lifecycle: first(classes) ?? "planned" });
      continue;
    }
    const children = splitMixedRecord(id, record.heading, record.text, classes);
    items.push({ ...base, child_count: children.length, children });
  }
  return {
    format: 1,
    source: SOURCE_PATH,
    source_digest: digest(source),
    scalar_indexing: "zero-based Unicode scalar half-open ranges",
    items,
  };
}

export function validateInventory(source, inventory) {
  const errors = [];
  const expected = buildInventory(source);
  if (inventory.source !== SOURCE_PATH) errors.push("inventory source path is invalid");
  if (inventory.source_digest !== digest(source)) errors.push("source digest changed");
  if (!Array.isArray(inventory.items)) return [...errors, "inventory items are missing"];

  const expectedById = new Map(expected.items.map((item) => [item.id, item]));
  const seen = new Set();
  for (const item of inventory.items) {
    if (seen.has(item.id)) errors.push(`duplicate inventory item ${item.id}`);
    seen.add(item.id);
    const wanted = expectedById.get(item.id);
    if (wanted === undefined) {
      errors.push(`unexpected inventory item ${item.id}`);
      continue;
    }
    if (item.heading !== wanted.heading || item.kind !== wanted.kind) {
      errors.push(`${item.id} source location changed`);
    }
    if (item.text !== wanted.text || item.digest !== digest(wanted.text)) {
      errors.push(`${item.id} digest or exact source text changed`);
    }
    if (JSON.stringify(item.identity) !== JSON.stringify(wanted.identity)) {
      errors.push(`${item.id} named identity changed`);
    }
    if (
      item.lifecycle !== wanted.lifecycle ||
      JSON.stringify(item.children) !== JSON.stringify(wanted.children)
    ) {
      errors.push(`${item.id} frozen lifecycle partition changed`);
    }
    errors.push(...validateItemLifecycle(item));
  }
  for (const id of expectedById.keys()) {
    if (!seen.has(id)) errors.push(`missing inventory item ${id}`);
  }
  return errors;
}

function validateItemLifecycle(item) {
  return validateLifecycleNode(item);
}

function validateLifecycleNode(item) {
  const errors = [];
  const stated = statedLifecycles(item.text);
  if (item.children === undefined) {
    if (!LIFECYCLES.has(item.lifecycle)) errors.push(`${item.id} has invalid lifecycle`);
    if (stated.size > 1) errors.push(`${item.id} mixes lifecycle statements without children`);
    if (stated.size === 1 && !stated.has(item.lifecycle)) {
      errors.push(`${item.id} has wrong lifecycle`);
    }
    return errors;
  }

  if (item.lifecycle !== undefined) errors.push(`${item.id} parent declares a lifecycle`);
  if (!Number.isInteger(item.child_count) || item.child_count !== item.children.length) {
    errors.push(`${item.id} child count does not match children`);
  }
  const scalars = [...item.text];
  const covered = new Array(scalars.length).fill(0);
  const childClasses = new Set();
  for (const [index, child] of item.children.entries()) {
    const expectedId = `${item.id}.${index + 1}`;
    if (child.id !== expectedId) errors.push(`${item.id} has missing or non-contiguous child ${expectedId}`);
    for (const lifecycle of leafLifecycles(child)) childClasses.add(lifecycle);
    if (!Array.isArray(child.spans) || child.spans.length === 0) {
      errors.push(`${child.id} has no scalar spans`);
      continue;
    }
    let childText = "";
    for (const span of child.spans) {
      if (!Array.isArray(span) || span.length !== 2 || !span.every(Number.isInteger)) {
        errors.push(`${child.id} has an invalid scalar span`);
        continue;
      }
      const [start, end] = span;
      if (start < 0 || end <= start || end > scalars.length) {
        errors.push(`${child.id} has an out-of-range scalar span`);
        continue;
      }
      childText += scalars.slice(start, end).join("");
      for (let scalar = start; scalar < end; scalar += 1) covered[scalar] += 1;
    }
    if (
      child.heading !== item.heading ||
      child.text !== childText ||
      child.digest !== digest(childText) ||
      JSON.stringify(child.identity) !== JSON.stringify(namedIdentity(childText))
    ) {
      errors.push(`${child.id} digest or exact child source text changed`);
    }
    errors.push(...validateLifecycleNode(child));
  }
  for (const [index, count] of covered.entries()) {
    if (isMeaningfulScalar(scalars[index]) && count === 0) {
      errors.push(`${item.id} has an uncovered source scalar at ${index}`);
      break;
    }
    if (count > 1) {
      errors.push(`${item.id} has overlapping child spans at ${index}`);
      break;
    }
  }
  for (const lifecycle of stated) {
    if (!childClasses.has(lifecycle)) errors.push(`${item.id} has an uncovered ${lifecycle} statement`);
  }
  return errors;
}

function leafLifecycles(item) {
  if (item.children === undefined) return [item.lifecycle];
  return item.children.flatMap(leafLifecycles);
}

export function validateLedger(inventory, ledger) {
  const errors = [];
  if (!ledger || !Array.isArray(ledger.entries)) return ["ledger entries are missing"];
  const leaves = new Set();
  const parents = new Set();
  for (const item of inventory.items) collectLedgerIds(item, parents, leaves);
  const seen = new Set();
  for (const entry of ledger.entries) {
    const id = entry?.source_id;
    if (typeof id !== "string" || /[*?]|\.\.|\ball\b/iu.test(id)) {
      errors.push(`ledger source ID ${String(id)} is a range, wildcard, or catch-all`);
      continue;
    }
    if (parents.has(id)) errors.push(`ledger maps parent ${id} directly`);
    if (!leaves.has(id)) errors.push(`ledger maps unknown leaf ${id}`);
    if (seen.has(id)) errors.push(`ledger maps leaf ${id} more than once`);
    seen.add(id);
    if (!LIFECYCLES.has(entry.lifecycle)) errors.push(`ledger leaf ${id} has invalid lifecycle`);
    if (entry.lifecycle === "removed" && leaves.has(id)) {
      errors.push(`ledger removes frozen leaf ${id}`);
    }
    const expectedKind = {
      current: "specification",
      completed: "implementation-record",
      planned: "proposal",
      removed: "removal",
    }[entry.lifecycle];
    const destination = entry.destination;
    if (!destination || destination.kind !== expectedKind) {
      errors.push(`ledger leaf ${id} has no valid ${String(expectedKind)} destination`);
    } else if (
      typeof destination.path !== "string" ||
      !/^(?:docs\/(?:specification|reference\/implemented-proposals|proposals)\/[^/]+\.md)$/u.test(destination.path)
    ) {
      errors.push(`ledger leaf ${id} has an invalid destination path`);
    } else if (
      entry.lifecycle === "current" &&
      (!Array.isArray(destination.evidence) || destination.evidence.length === 0)
    ) {
      errors.push(`ledger leaf ${id} current destination has no checked evidence`);
    } else if (entry.lifecycle === "removed" && !destination.rationale) {
      errors.push(`ledger leaf ${id} removal has no rationale`);
    }
  }
  for (const id of leaves) if (!seen.has(id)) errors.push(`ledger is missing leaf ${id}`);
  return errors;
}

function collectLedgerIds(item, parents, leaves) {
  if (item.children === undefined) {
    leaves.add(item.id);
    return;
  }
  parents.add(item.id);
  for (const child of item.children) collectLedgerIds(child, parents, leaves);
}

export function validateLedgerSchema(schema) {
  const errors = [];
  if (schema?.$schema !== "https://json-schema.org/draft/2020-12/schema") {
    errors.push("ledger schema must use JSON Schema draft 2020-12");
  }
  const entry = schema?.$defs?.entry;
  if (!entry || entry.additionalProperties !== false) errors.push("ledger entry schema must be closed");
  for (const field of ["source_id", "lifecycle", "destination"]) {
    if (!entry?.required?.includes(field)) errors.push(`ledger schema does not require ${field}`);
  }
  if (JSON.stringify(entry?.properties?.lifecycle?.enum) !== JSON.stringify([...LIFECYCLES])) {
    errors.push("ledger schema lifecycle enum is invalid");
  }
  return errors;
}

export function validateDiffScope(changedPaths) {
  const protectedPatterns = [
    /^crates\/veln-cli\/tests\/toolchain_harness(?:\.rs|\/)/u,
    /^crates\/veln-cli\/tests\/toolchain_cases\/mcp\//u,
    /^crates\/veln-cli\/tests\/toolchain-case-semantics\.baseline$/u,
    /^crates\/veln-mcp\//u,
    /^examples\/specification\/mcp\//u,
  ];
  const allowed = new Set([
    SOURCE_PATH,
    INVENTORY_PATH,
    SCHEMA_PATH,
    "docs/proposals/agent-language-services-lifecycle-migration.md",
    ".github/workflows/workflow--test-scripts.yaml",
    "workflow-scripts/check-agent-language-services-inventory.mjs",
    "workflow-scripts/check-agent-language-services-inventory.test.mjs",
  ]);
  const errors = [];
  for (const file of changedPaths) {
    if (file === SOURCE_PATH) errors.push(`diff scope changes frozen umbrella proposal ${file}`);
    if (protectedPatterns.some((pattern) => pattern.test(file))) {
      errors.push(`diff scope changes protected MCP or semantic evidence ${file}`);
    } else if (!allowed.has(file)) {
      errors.push(`diff scope contains unrelated path ${file}`);
    }
  }
  return errors;
}

export function changedPaths(base, head = "HEAD", root = process.cwd()) {
  const output = execFileSync(
    "git",
    ["diff", "--name-status", "--diff-filter=ACMRD", base, head, "--"],
    { cwd: root, encoding: "utf8" },
  );
  return parseChangedPathStatus(output);
}

export function parseChangedPathStatus(output) {
  return output
    .split("\n")
    .filter(Boolean)
    .flatMap((line) => line.split("\t").slice(1));
}

function extractRecords(source) {
  const lines = source.split("\n");
  const records = [];
  let heading = "Document";
  let paragraph = [];
  let paragraphHeading = heading;
  let paragraphKind = "paragraph";
  let fenced = false;
  let fenceLines = [];
  let fenceHeading = heading;
  const flushParagraph = () => {
    if (paragraph.length > 0) {
      records.push({ kind: paragraphKind, heading: paragraphHeading, text: paragraph.join("\n") });
      paragraph = [];
      paragraphKind = "paragraph";
    }
  };
  const flushFence = () => {
    for (const line of fenceLines) {
      if (line.trim() !== "") records.push({ kind: "schema-or-example-line", heading: fenceHeading, text: line });
    }
    fenceLines = [];
  };

  let frontmatter = lines[0] === "---";
  for (let lineIndex = frontmatter ? 1 : 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];
    if (frontmatter) {
      if (line === "---") frontmatter = false;
      continue;
    }
    if (/^```/u.test(line)) {
      flushParagraph();
      if (fenced) flushFence();
      else fenceHeading = heading;
      fenced = !fenced;
      continue;
    }
    if (fenced) {
      fenceLines.push(line);
      continue;
    }
    const headingMatch = /^(#{1,6})\s+(.+)$/u.exec(line);
    if (headingMatch) {
      flushParagraph();
      heading = headingMatch[2].trim();
      continue;
    }
    if (line.trim() === "") {
      flushParagraph();
      continue;
    }
    if (/^\s*(?:[-*+] |\d+\. )/u.test(line)) {
      flushParagraph();
      paragraphHeading = heading;
      paragraphKind = "list-item";
      paragraph.push(line);
      continue;
    }
    if (/^\s*\|.*\|\s*$/u.test(line)) {
      flushParagraph();
      const text = line.trim();
      const parts = tableParts(text);
      if (parts.every((part) => /^:?-{3,}:?$/u.test(part.text.trim()))) continue;
      records.push({ kind: "table-row", heading, text, parts });
      continue;
    }
    if (paragraph.length === 0) {
      paragraphHeading = heading;
      paragraphKind = "paragraph";
    }
    paragraph.push(line);
  }
  flushParagraph();
  if (fenceLines.length > 0) flushFence();
  return records;
}

function tableParts(text) {
  const scalars = [...text];
  const parts = [];
  let start = 1;
  let escaped = false;
  for (let index = 1; index < scalars.length; index += 1) {
    const scalar = scalars[index];
    if (scalar === "|" && !escaped) {
      parts.push({ text: scalars.slice(start, index).join(""), spans: [[start, index]] });
      start = index + 1;
    }
    escaped = scalar === "\\" && !escaped;
    if (scalar !== "\\") escaped = false;
  }
  return parts.filter((part) => part.text.trim() !== "");
}

function splitMixedRecord(id, heading, text, classes) {
  const scalars = [...text];
  const markers = lifecycleMarkers(text);
  const children = [];
  let start = 0;
  let lifecycle = markers[0]?.lifecycle ?? first(classes);
  for (let markerIndex = 1; markerIndex < markers.length; markerIndex += 1) {
    const marker = markers[markerIndex];
    if (marker.lifecycle === lifecycle) continue;
    const end = codeUnitToScalarIndex(text, marker.index);
    if (end > start) children.push({ lifecycle, spans: [[start, end]] });
    start = end;
    lifecycle = marker.lifecycle;
  }
  if (start < scalars.length) children.push({ lifecycle, spans: [[start, scalars.length]] });
  return children.map((child, index) =>
    childRecord(`${id}.${index + 1}`, heading, text, child.spans, child.lifecycle),
  );
}

function splitStableDomainCodeRecord(id, heading, text) {
  const prefixEnd = text.indexOf(". The request spelling");
  const domainText = prefixEnd === -1 ? text : text.slice(0, prefixEnd);
  const codeSpans = [...domainText.matchAll(/`([^`]+)`/gu)].map((match) => [
    codeUnitToScalarIndex(text, match.index),
    codeUnitToScalarIndex(text, match.index + match[0].length),
  ]);
  const scalars = [...text];
  const parts = [];
  let start = 0;
  for (const [codeStart, codeEnd] of codeSpans) {
    if (start < codeStart) parts.push([[start, codeStart]]);
    parts.push([[codeStart, codeEnd]]);
    start = codeEnd;
  }
  if (start < scalars.length) parts.push([[start, scalars.length]]);
  return parts.map((spans, index) =>
    childRecord(`${id}.${index + 1}`, heading, text, spans, "planned"),
  );
}

function childRecord(id, heading, parentText, spans, lifecycle) {
  const scalars = [...parentText];
  const text = spans.map(([start, end]) => scalars.slice(start, end).join("")).join("");
  return {
    id,
    heading,
    identity: namedIdentity(text),
    digest: digest(text),
    text,
    lifecycle: lifecycle ?? first(statedLifecycles(text)) ?? "planned",
    spans,
  };
}

function isStableDomainCodeRecord(text) {
  return text.startsWith("The stable v1 domain codes are ");
}

function statedLifecycles(text) {
  return new Set(lifecycleMarkers(text).map((marker) => marker.lifecycle));
}

function lifecycleMarkers(text) {
  const patterns = [
    ["current", CURRENT],
    ["completed", COMPLETED],
    ["planned", PLANNED],
    ["removed", REMOVED],
  ];
  const markers = [];
  for (const [lifecycle, pattern] of patterns) {
    const global = new RegExp(pattern.source, `${pattern.flags}g`);
    for (const match of text.matchAll(global)) markers.push({ index: match.index, lifecycle });
  }
  return markers.sort((left, right) => left.index - right.index);
}

function namedIdentity(text) {
  const names = [];
  for (const match of text.matchAll(/\bQ(?:0[1-9]|1[0-9]|2[0-2])\b/gu)) names.push(match[0]);
  for (const match of text.matchAll(/`([^`]+)`/gu)) names.push(match[1]);
  return [...new Set(names)];
}

function digest(text) {
  return createHash("sha256").update(text, "utf8").digest("hex");
}

function codeUnitToScalarIndex(text, codeUnitIndex) {
  return [...text.slice(0, codeUnitIndex)].length;
}

function isMeaningfulScalar(scalar) {
  return !/\s/u.test(scalar) && !MARKDOWN_DELIMITER.test(scalar);
}

function first(set) {
  return set.values().next().value;
}

function isMainModule() {
  return process.argv[1] !== undefined && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
}
