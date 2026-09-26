#!/usr/bin/env node

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, realpathSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, posix, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const defaultRepositoryRoot = dirname(dirname(scriptPath));
const defaultSkillPath = join(defaultRepositoryRoot, ".agents", "skills", "veln-language", "SKILL.md");
const catalogPath = join("tools", "veln-repo-language-reference", "generated", "language-reference-catalog-v1.json");
const digestPath = join("tools", "veln-repo-language-reference", "generated", "language-reference-catalog-v1.sha256");
const caseFoldingPath = join("crates", "veln-project", "testdata", "case_folding_17_c_f.txt");
const snapshotEvidencePath = join(
  "workflow-scripts",
  "fixtures",
  "veln-language",
  "snapshot-catalogs.json",
);
const requestSelectionOraclePath = join(
  defaultRepositoryRoot,
  "workflow-scripts",
  "fixtures",
  "veln-language",
  "request-selection-oracle.json",
);

const acceptance = new Map([
  ["language-match", { route: "language", finalStatus: "answered" }],
  ["language-no-match", { route: "language", finalStatus: "no_match" }],
  ["search-unavailable", { route: "language", finalStatus: "search_unavailable", failure: true }],
  ["topic-unreadable", { route: "language", finalStatus: "topic_unavailable", failure: true }],
  ["stale-snapshot-uri", { route: "language", finalStatus: "stale_snapshot", failure: true }],
  ["repository-current", {
    route: "repository",
    finalStatus: "repository_routed",
    authority: "docs/specification/mcp.md",
  }],
  ["repository-implementation-location", {
    route: "repository",
    finalStatus: "repository_routed",
    authority: "docs/specification/source-surface.md",
  }],
  ["repository-change", {
    route: "repository",
    finalStatus: "repository_routed",
    authority: "docs/specification/source-surface.md",
  }],
  ["repository-proposal", {
    route: "repository",
    finalStatus: "repository_routed",
    authority: "docs/proposals/README.md",
    readyOnly: true,
  }],
  ["repository-explicit-target", {
    route: "repository",
    finalStatus: "repository_routed",
    authority: "docs/proposals/README.md",
  }],
  ["repository-reference", {
    route: "repository",
    finalStatus: "repository_routed",
    authority: "docs/reference/documentation-authoring.md",
  }],
  ["repository-unknown", {
    route: "repository",
    finalStatus: "repository_no_route",
    paths: ["docs/README.md", "docs/navigation.md", "docs/navigation-full.md"],
  }],
]);

const snapshotTopicUri = /^veln-doc:\/\/\/language\/snapshot\/[0-9a-f]{64}\/topic\/[a-z0-9]+(?:-[a-z0-9]+)*$/;
const markdownMimeType = "text/markdown; charset=utf-8";
const canonicalSkillDescription = "Use for Veln language questions and for inspecting or changing the Veln repository through its documentation authority.";
const fixtureLimits = {
  scenarios: acceptance.size,
  turnsPerScenario: 2,
  eventsPerTurn: 5,
  claimsPerAnswer: 16,
  searchResults: 50,
  requestCharacters: 1_000,
  resourceTextBytes: 262_144,
  repositoryDocumentBytes: 262_144,
  fixtureBytes: 1_000_000,
  requestSelectionCases: 128,
  skillDescriptionCharacters: 300,
  repositoryDiscoveryDocuments: 64,
  publishedCatalogBytes: 2_000_000,
  publishedCatalogTopics: 512,
  snapshotCatalogs: 32,
  snapshotOverrides: 64,
  snapshotTopicWork: 16_384,
  schemaReferenceDepth: 64,
  schemaTraversalDepth: 128,
};

export function loadPublishedLanguageReference(repositoryRoot) {
  const digest = readFileSync(join(repositoryRoot, digestPath), "utf8").trim();
  assert.match(digest, /^[0-9a-f]{64}$/, "checked language-reference digest must be canonical");
  const absoluteCatalogPath = join(repositoryRoot, catalogPath);
  assert.ok(
    statSync(absoluteCatalogPath).size <= fixtureLimits.publishedCatalogBytes,
    "checked language-reference catalog exceeds the byte limit",
  );
  const catalogBytes = readFileSync(absoluteCatalogPath);
  assert.equal(
    catalogDigest(catalogBytes),
    digest,
    "checked language-reference catalog digest must match its sidecar",
  );
  const catalog = JSON.parse(catalogBytes.toString("utf8"));
  assert.ok(Array.isArray(catalog.topics), "checked language-reference catalog must contain topics");
  assert.ok(
    catalog.topics.length <= fixtureLimits.publishedCatalogTopics,
    "checked language-reference catalog exceeds the topic limit",
  );
  return { digest, catalog, caseFoldMappings: loadCaseFoldMappings(repositoryRoot) };
}

export function loadCaseFoldMappings(repositoryRoot = defaultRepositoryRoot) {
  const mappings = new Map();
  const rows = readFileSync(join(repositoryRoot, caseFoldingPath), "utf8").split("\n");
  for (const [index, row] of rows.entries()) {
    if (row.length === 0 || row.startsWith("#")) continue;
    const match = /^([0-9A-F]+);([0-9A-F]+(?: [0-9A-F]+)*)$/u.exec(row);
    assert.ok(match, `case-folding row ${index + 1} is invalid`);
    const source = String.fromCodePoint(Number.parseInt(match[1], 16));
    const replacement = match[2]
      .split(" ")
      .map((codePoint) => String.fromCodePoint(Number.parseInt(codePoint, 16)))
      .join("");
    assert.equal(mappings.has(source), false, `case-folding row ${index + 1} is duplicated`);
    mappings.set(source, replacement);
  }
  assert.ok(mappings.size > 0, "case-folding data must contain mappings");
  return mappings;
}

function catalogDigest(bytes) {
  const length = Buffer.alloc(8);
  length.writeBigUInt64BE(BigInt(bytes.length));
  return createHash("sha256")
    .update(Buffer.from("veln-language-reference/v1\0"))
    .update(length)
    .update(bytes)
    .digest("hex");
}

export function loadSnapshotEvidence(repositoryRoot, published) {
  const path = join(repositoryRoot, snapshotEvidencePath);
  assert.ok(statSync(path).size <= fixtureLimits.fixtureBytes, "snapshot evidence exceeds the fixture byte limit");
  const document = JSON.parse(readFileSync(path, "utf8"));
  assertExactKeys(document, ["schema_version", "snapshots"], "snapshot evidence");
  assert.equal(document.schema_version, 1, "unsupported snapshot-evidence schema");
  assert.ok(Array.isArray(document.snapshots), "snapshot evidence must contain snapshots");
  assert.ok(
    document.snapshots.length <= fixtureLimits.snapshotCatalogs,
    "snapshot evidence exceeds the snapshot limit",
  );
  assert.ok(
    document.snapshots.length * published.catalog.topics.length <= fixtureLimits.snapshotTopicWork,
    "snapshot evidence exceeds the snapshot-topic work limit",
  );
  const snapshots = new Map();
  for (const [index, snapshot] of document.snapshots.entries()) {
    const context = `snapshot evidence ${index + 1}`;
    assertExactKeys(snapshot, ["base_digest", "digest", "topic_overrides"], context);
    assert.equal(snapshot.base_digest, published.digest, `${context}: base snapshot digest changed`);
    assert.match(snapshot.digest, /^[0-9a-f]{64}$/, `${context}: digest must be canonical`);
    assert.ok(Array.isArray(snapshot.topic_overrides), `${context}: topic overrides must be an array`);
    assert.ok(
      snapshot.topic_overrides.length <= fixtureLimits.snapshotOverrides,
      `${context}: topic overrides exceed the limit`,
    );
    const catalog = materializeSnapshotCatalog(published, snapshot, context);
    const bytes = Buffer.from(`${JSON.stringify(catalog)}\n`);
    assert.equal(catalogDigest(bytes), snapshot.digest, `${context}: catalog digest does not match evidence`);
    assert.equal(snapshots.has(snapshot.digest), false, `${context}: duplicate snapshot digest`);
    snapshots.set(snapshot.digest, snapshot);
  }
  return snapshots;
}

function materializeSnapshotCatalog(published, snapshot, context) {
  const baseTopics = new Map(published.catalog.topics.map((topic) => [topic.id, topic]));
  const replacements = new Map();
  for (const [overrideIndex, override] of snapshot.topic_overrides.entries()) {
    const overrideContext = `${context} override ${overrideIndex + 1}`;
    assertExactKeys(
      override,
      ["topic_id", "id", "title", "summary", "keywords", "body"],
      overrideContext,
    );
    assert.equal(replacements.has(override.topic_id), false, `${overrideContext}: duplicate topic override`);
    const baseTopic = baseTopics.get(override.topic_id);
    assert.ok(baseTopic, `${overrideContext}: base topic does not exist`);
    const { topic_id: _topicId, ...replacement } = override;
    replacements.set(override.topic_id, { ...baseTopic, ...replacement });
  }
  const topics = published.catalog.topics
    .map((topic) => replacements.get(topic.id) ?? topic)
    .sort((left, right) => Buffer.compare(Buffer.from(left.id), Buffer.from(right.id)));
  return { ...published.catalog, topics };
}

function topicUri(digest, topicId) {
  return `veln-doc:///language/snapshot/${digest}/topic/${topicId}`;
}

function renderTopic(topic, digest) {
  let text = `# ${topic.title}\n\n${topic.summary}\n\n`;
  for (const paragraph of topic.body) text += `${paragraph}\n\n`;
  text += "## Grammar\n\n";
  for (const grammar of topic.grammar) {
    text += `### ${grammar.name}\n\n\`\`\`ebnf\n${grammar.text}\n\`\`\`\n\n`;
  }
  text += "## Examples\n\n";
  for (const example of topic.examples) {
    text += `### ${example.display_name}\n\n`;
    for (const file of example.files) {
      text += `#### ${file.path}\n\n\`\`\`veln\n${file.source}\n\`\`\`\n\n`;
    }
  }
  text += "## Keywords\n\n";
  for (const keyword of topic.keywords) text += `- ${keyword}\n`;
  text += "\n## Related Topics\n\n";
  for (const related of topic.related) text += `- [${related}](${topicUri(digest, related)})\n`;
  return text;
}

const unicodeWhitespaceScalar = /^\p{White_Space}$/u;

function trimUnicodeWhitespace(text) {
  let start = 0;
  let end = text.length;
  while (start < end) {
    const codePoint = text.codePointAt(start);
    const character = String.fromCodePoint(codePoint);
    if (!unicodeWhitespaceScalar.test(character)) break;
    start += character.length;
  }
  while (end > start) {
    const codePoint = text.codePointAt(end - 1);
    const characterLength = codePoint >= 0xDC00 && codePoint <= 0xDFFF ? 2 : 1;
    const character = text.slice(end - characterLength, end);
    if (!unicodeWhitespaceScalar.test(character)) break;
    end -= characterLength;
  }
  return text.slice(start, end);
}

export function normalizeSearchText(text, caseFoldMappings) {
  const folded = [...text.normalize("NFC")]
    .map((character) => caseFoldMappings.get(character) ?? character)
    .join("");
  return trimUnicodeWhitespace(folded);
}

function foldedScalarByteEnds(field, caseFoldMappings) {
  const chunks = [];
  const byteEnds = [];
  let byteOffset = 0;
  for (const character of field) {
    const folded = normalizeSearchText(character, caseFoldMappings);
    byteOffset += Buffer.byteLength(folded);
    chunks.push(folded);
    byteEnds.push(byteOffset);
  }
  return { bytes: Buffer.from(chunks.join("")), byteEnds };
}

function firstIndexWhere(values, predicate) {
  let start = 0;
  let end = values.length;
  while (start < end) {
    const middle = start + Math.floor((end - start) / 2);
    if (predicate(values[middle])) end = middle;
    else start = middle + 1;
  }
  return start;
}

function firstTokenSpan(field, tokens, caseFoldMappings) {
  const folded = foldedScalarByteEnds(field, caseFoldMappings);
  const candidates = [];
  for (const token of tokens) {
    const tokenBytes = Buffer.from(token);
    const start = folded.bytes.indexOf(tokenBytes);
    if (start < 0) continue;
    const end = start + tokenBytes.length;
    const first = firstIndexWhere(folded.byteEnds, (byteEnd) => byteEnd > start);
    const last = firstIndexWhere(folded.byteEnds, (byteEnd) => byteEnd >= end);
    if (first < folded.byteEnds.length && last >= first) {
      candidates.push({ start: first, end: last + 1 });
    }
  }
  candidates.sort((left, right) => left.start - right.start);
  return candidates[0];
}

function excerpt(field, matchedScalars) {
  const scalars = [...field];
  if (scalars.length <= 160) {
    return { excerpt: field, prefix_truncated: false, suffix_truncated: false };
  }
  const matchLength = matchedScalars.end - matchedScalars.start;
  const start = matchLength <= 160
    ? Math.min(matchedScalars.start, scalars.length - 160)
    : matchedScalars.start;
  const end = Math.min(start + 160, scalars.length);
  return {
    excerpt: scalars.slice(start, end).join(""),
    prefix_truncated: start > 0,
    suffix_truncated: end < scalars.length,
  };
}

function firstExcerpt(fields, tokens, caseFoldMappings) {
  for (const field of fields) {
    const span = firstTokenSpan(field, tokens, caseFoldMappings);
    if (span !== undefined) return excerpt(field, span);
  }
  return excerpt(fields[0], { start: 0, end: 0 });
}

export function expectedPublishedSearch(arguments_, published) {
  const { caseFoldMappings } = published;
  assert.ok(caseFoldMappings instanceof Map, "published search evidence requires case-folding mappings");
  const query = normalizeSearchText(arguments_.query, caseFoldMappings);
  const tokens = query.split(/\p{White_Space}+/u);
  const matches = [];
  for (const topic of published.catalog.topics) {
    const tiers = [
      [topic.id, topic.title],
      [topic.id, topic.title],
      [topic.title, ...topic.keywords],
      [topic.summary],
      [topic.body.join("\n\n")],
    ];
    for (const [index, fields] of tiers.entries()) {
      const normalized = fields.map((field) => normalizeSearchText(field, caseFoldMappings));
      const matched = index === 0
        ? normalized.some((field) => field === query)
        : index === 1
          ? normalized.some((field) => field.startsWith(query))
          : tokens.every((token) => normalized.some((field) => field.includes(token)));
      if (!matched) continue;
      matches.push({
        rank: index + 1,
        uri: topicUri(published.digest, topic.id),
        title: topic.title,
        summary: topic.summary,
        ...firstExcerpt(fields, tokens, caseFoldMappings),
      });
      break;
    }
  }
  matches.sort((left, right) => left.rank - right.rank || Buffer.compare(Buffer.from(left.uri), Buffer.from(right.uri)));
  return {
    scope: "language",
    results: matches.slice(0, arguments_.limit ?? 10).map(({ rank: _rank, ...result }) => result),
  };
}
const operativePrefix = `

# Veln Language Routing

The JSON contract below is the complete operative instruction set for this
skill. Apply it exactly. Do not add a fallback from other instructions.

`;

function sortedKeys(value) {
  return Object.keys(value).sort();
}

function assertExactKeys(value, keys, context) {
  assert.equal(value !== null && typeof value === "object" && !Array.isArray(value), true, `${context}: expected an object`);
  assert.deepEqual(sortedKeys(value), [...keys].sort(), `${context}: fields must match the closed shape`);
}

function requestSelectionRecord(selection) {
  const { id, text, action, subject } = selection;
  return { id, text, action, subject };
}

function loadRequestSelectionOracle() {
  assert.ok(
    statSync(requestSelectionOraclePath).size <= fixtureLimits.fixtureBytes,
    "request-selection oracle exceeds the byte limit",
  );
  const document = JSON.parse(readFileSync(requestSelectionOraclePath, "utf8"));
  assertExactKeys(document, ["schema_version", "records"], "request-selection oracle");
  assert.equal(document.schema_version, 1, "unsupported request-selection oracle schema");
  assert.ok(Array.isArray(document.records), "request-selection oracle must contain records");
  assert.ok(
    document.records.length <= fixtureLimits.requestSelectionCases,
    "request-selection oracle exceeds the case limit",
  );
  const ids = new Set();
  const texts = new Set();
  const records = document.records
    .map((record) => {
      assertExactKeys(record, ["id", "text", "action", "subject"], "request-selection oracle record");
      assert.equal(typeof record.id, "string", "request-selection oracle ID must be a string");
      assert.ok(record.id.length > 0, "request-selection oracle ID must not be empty");
      assert.equal(ids.has(record.id), false, `duplicate request-selection oracle ID ${record.id}`);
      ids.add(record.id);
      assert.equal(typeof record.text, "string", `${record.id}: oracle text must be a string`);
      assert.ok(record.text.length <= fixtureLimits.requestCharacters, `${record.id}: oracle text exceeds the limit`);
      assert.equal(texts.has(record.text), false, `${record.id}: duplicate request-selection oracle text`);
      texts.add(record.text);
      routeRequestSemantics({ action: record.action, subject: record.subject }, `${record.id}: oracle`);
      return record;
    })
    .sort((left, right) => Buffer.compare(Buffer.from(left.id), Buffer.from(right.id)));
  return new Map(records.map((record) => [record.id, record]));
}

function parseSkillContract(skillText) {
  const frontmatter = skillText.match(/^---\n([\s\S]*?)\n---/);
  assert.match(frontmatter?.[1] ?? "", /^name:\s*veln-language\s*$/m, "canonical skill name must be veln-language");
  const descriptions = [...(frontmatter?.[1] ?? "").matchAll(/^description:\s*(.*?)\s*$/gm)];
  assert.equal(descriptions.length, 1, "canonical skill must have one description");
  const description = descriptions[0][1];
  assert.ok(description.length > 0, "canonical skill description must not be empty");
  assert.ok(
    [...description].length <= fixtureLimits.skillDescriptionCharacters,
    "canonical skill description exceeds the discovery limit",
  );
  assert.match(description, /\bVeln\b.*\blanguage questions?\b/i, "skill description must select Veln language questions");
  assert.match(
    description,
    /\brepositor(?:y|ies)\b.*\b(?:inspect(?:ion|ing)?|chang(?:e|es|ing))\b|\b(?:inspect(?:ion|ing)?|chang(?:e|es|ing))\b.*\brepositor(?:y|ies)\b/i,
    "skill description must select repository inspection or change requests",
  );
  assert.equal(
    description,
    canonicalSkillDescription,
    "canonical skill description must match the checked discovery contract",
  );
  const match = skillText.match(
    /<!-- veln-language-contract:start -->\s*```json\s*([\s\S]*?)\s*```\s*<!-- veln-language-contract:end -->/,
  );
  assert.ok(match, "veln-language skill must contain its scenario contract");
  const bodyStart = frontmatter[0].length;
  const contractStart = skillText.indexOf("<!-- veln-language-contract:start -->");
  const contractEnd = skillText.indexOf("<!-- veln-language-contract:end -->")
    + "<!-- veln-language-contract:end -->".length;
  assert.equal(skillText.slice(bodyStart, contractStart), operativePrefix, "skill must use the checked operative instruction wrapper");
  assert.match(skillText.slice(contractEnd), /^\n?$/, "skill must not contain unchecked operative instructions");
  const contract = JSON.parse(match[1]);
  assertExactKeys(
    contract,
    ["schema_version", "request_selection", "language", "repository", "failure", "maintenance"],
    "veln-language skill contract",
  );
  assert.equal(contract.schema_version, 1, "unsupported veln-language skill contract");
  assert.deepEqual(contract.request_selection, {
    repository_when: "The request asks to inspect, test, or change the Veln repository or makes the repository, codebase, source code, or proposal state its subject.",
    semantic_basis: "Classify the requested action and subject by meaning. Do not decide from a closed vocabulary of verbs, question words, or sentence frames.",
    language_otherwise: true,
  }, "veln-language request-selection contract is inconsistent");
  assert.deepEqual(contract.language, {
    search_tool: "search_docs",
    search_scope: "language",
    read_tool: "read_doc",
    maximum_calls: 2,
    call_order: ["search_docs", "read_doc"],
    selection: "first_search_result",
    read_exact_search_result_uri: true,
    fallback: "forbidden",
    answer_source: "selected_resource_uri",
    report_selected_uri: true,
    no_match: "Report that the published Veln language reference has no matching topic. Do not use proposal text or model memory.",
  }, "veln-language skill language contract is inconsistent");
  assertExactKeys(contract.repository, [
    "entry",
    "follow_selected_links",
    "maximum_reads",
    "repeat_paths",
    "published_reference_is_authority",
    "authority_selection",
    "no_route",
    "semantic_selection",
    "path_prefixes",
  ], "veln-language repository contract");
  assert.equal(contract.repository?.entry, "docs/README.md", "skill must start repository tasks at docs/README.md");
  assert.equal(contract.repository?.follow_selected_links, true, "skill must follow repository documentation links");
  assert.equal(contract.repository?.maximum_reads, 3, "skill must bound repository documentation reads");
  assert.equal(contract.repository?.repeat_paths, "forbidden", "skill must reject repository documentation cycles");
  assert.equal(contract.repository?.published_reference_is_authority, false, "skill must reject published-reference repository authority");
  assert.equal(
    contract.repository?.authority_selection,
    "smallest_current_linked_authority",
    "skill must select the smallest current linked repository authority",
  );
  assert.equal(
    contract.repository?.no_route,
    "Stop and report that no repository documentation route covers the request.",
    "skill must define the missing repository route outcome",
  );
  assert.equal(
    contract.repository?.semantic_selection,
    "A request for the agent to examine, validate, or alter Veln behavior or implementation selects repository work. A question whose subject is a compiler or parser implementation also selects repository work. A request for information about how a Veln language feature behaves selects language work. A repository, codebase, source-code, proposal-state, or repository-path subject selects repository work unless the phrase is incidental or being defined.",
    "skill must route requested actions and subjects by meaning",
  );
  assert.deepEqual(contract.repository?.path_prefixes, [
    ".agents/",
    ".github/",
    "crates/",
    "docs/",
    "editors/",
    "examples/",
    "scripts/",
    "tools/",
    "workflow-scripts/",
  ], "skill must recognize repository paths without relying on generic action words");
  assert.deepEqual(contract.failure, {
    fallback: "forbidden",
    preserve_previous_result: true,
    report_operation: true,
    report_selected_uri: true,
    search_unavailable: "Stop after search_docs and report that published language-reference search is unavailable.",
    topic_unavailable: "Stop after read_doc and report that the selected published topic is unavailable.",
    stale_snapshot: "When read_doc returns resource_not_found for the selected snapshot URI, stop and report that the URI is stale.",
  }, "veln-language skill failure contract is inconsistent");
  assert.deepEqual(contract.maintenance, [
    "docs/specification/language-reference-catalog.md",
    "docs/specification/mcp.md",
    "docs/README.md",
  ], "veln-language skill maintenance routes are incomplete");
  return contract;
}

function loadSkillContract(options) {
  const skillText = options.skillText ?? readFileSync(options.skillPath ?? defaultSkillPath, "utf8");
  return parseSkillContract(skillText);
}

function routeRequestSemantics(semantics, context) {
  assertExactKeys(semantics, ["action", "subject"], `${context}: request semantics`);
  assert.ok(
    ["information", "repository_action"].includes(semantics.action),
    `${context}: unknown requested action class`,
  );
  assert.ok(
    ["implementation", "language_behavior", "repository_material"].includes(semantics.subject),
    `${context}: unknown request subject class`,
  );
  if (semantics.action === "repository_action"
    || semantics.subject === "implementation"
    || semantics.subject === "repository_material") {
    return "repository";
  }
  return "language";
}

export function selectRequestRoute(semantics, options = {}) {
  loadSkillContract(options);
  return routeRequestSemantics(semantics, "request selection");
}

function expectedSelection(text, route, context) {
  const lower = text.toLocaleLowerCase("en-US");
  if (route === "language") {
    const subjects = [
      { match: /\bschemas?\b/u.exec(lower), topic: "schemas", query: "schemas" },
      { match: /\bcontracts?\b/u.exec(lower), topic: "contracts", query: "contracts" },
      { match: /\beffects?\b/u.exec(lower), topic: "effects-handlers", query: "effects" },
      { match: /\bmodules?\b/u.exec(lower), topic: "modules-imports-packages", query: "modules" },
      { match: /\bborrow checker\b/u.exec(lower), topic: undefined, query: "borrow checker" },
    ].filter((subject) => subject.match !== null)
      .sort((left, right) => left.match.index - right.match.index || left.query.localeCompare(right.query));
    if (subjects[0]?.topic === "schemas") {
      return {
        searchArguments: { query: subjects[0].query, scope: "language" },
        topic: "schemas",
      };
    }
    if (subjects.length > 0) {
      return {
        searchArguments: { query: subjects[0].query, scope: "language" },
        topic: subjects[0].topic,
      };
    }
    assert.fail(`${context}: request has no independent language evidence selection rule`);
  }

  if (lower.includes("docs/specification/types.md")) {
    return {
      authority: "docs/specification/types.md",
    };
  }
  if (/\bmcp\b.*\bdocumentation search\b/u.test(lower)) {
    return { authority: "docs/specification/mcp.md" };
  }
  if (/\b(?:compiler crashes|lexer bug|parser recovery|compiler parser|parser implemented|parser implementation)\b/u.test(lower)
    || /\bschema parsing\b/u.test(lower)
    || /\b(?:add|change|debug|examine|fix|implement|inspect|investigate|modify|refactor|remove|review|select|test|update)\b[^.!?]*\bveln schemas?\b/u.test(lower)
    || lower.includes("crates/veln-mcp/")) {
    return {
      authority: "docs/specification/source-surface.md",
    };
  }
  if (/\b(?:next ready|proposal state)\b/u.test(lower)) {
    return { authority: "docs/proposals/README.md" };
  }
  if (/\bdocumentation authoring policy\b/u.test(lower)) {
    return { authority: "docs/reference/documentation-authoring.md" };
  }
  if (/\bundocumented deployment service\b/u.test(lower)) {
    return { paths: ["docs/README.md", "docs/navigation.md", "docs/navigation-full.md"] };
  }
  assert.fail(`${context}: request has no independent repository authority selection rule`);
}

function finalAnswer(events, context) {
  assert.equal(events.at(-1)?.type, "answer", `${context}: answer must be the final event`);
  assert.equal(events.filter((event) => event.type === "answer").length, 1, `${context}: expected one answer`);
  return events.at(-1);
}

function retainedResult(answer) {
  return structuredClone(answer);
}

function validateClaims(answer, expectedSource, resourceText, context) {
  assert.ok(Array.isArray(answer.claims), `${context}: answer claims must be an array`);
  assert.ok(Array.isArray(answer.source_uris), `${context}: answer source_uris must be an array`);
  if (expectedSource === undefined) {
    assert.deepEqual(answer.claims, [], `${context}: bounded outcome must not claim language behavior`);
    assert.deepEqual(answer.source_uris, [], `${context}: bounded outcome must not cite a fallback`);
    return;
  }
  assert.ok(answer.claims.length > 0, `${context}: successful answer must contain a claim`);
  assert.ok(answer.claims.length <= fixtureLimits.claimsPerAnswer, `${context}: answer exceeds the claim limit`);
  assert.deepEqual(answer.source_uris, [expectedSource], `${context}: answer must report only the exact selected URI`);
  assert.ok(Buffer.byteLength(resourceText, "utf8") <= fixtureLimits.resourceTextBytes, `${context}: selected resource exceeds the published byte limit`);
  const evidence = new Set(resourceText
    .split(/(?<=[.!?])(?:\s+|$)|\n+/u)
    .map((statement) => statement.trim())
    .filter(Boolean));
  for (const claim of answer.claims) {
    assert.equal(typeof claim, "string", `${context}: every claim must be text`);
    assert.ok(claim.trim().length > 0, `${context}: claims must not be empty`);
    assert.ok(evidence.has(claim), `${context}: claim must match unambiguous selected-resource evidence`);
  }
}

function snapshotDigest(uri, context) {
  const match = uri.match(snapshotTopicUri);
  assert.ok(match, `${context}: topic URI must contain a canonical snapshot digest`);
  return uri.split("/")[5];
}

export function validateSchema(value, schema, root, context, traversal = undefined) {
  const state = traversal ?? {
    referenceDepth: 0,
    traversalDepth: 0,
    activeReferences: new Set(),
    validatedReferences: new Map(),
  };
  assert.ok(
    state.traversalDepth <= fixtureLimits.schemaTraversalDepth,
    `${context}: schema validation exceeded the ${fixtureLimits.schemaTraversalDepth}-traversal depth bound`,
  );
  const descend = () => ({
    ...state,
    traversalDepth: state.traversalDepth + 1,
  });
  if (schema.$ref !== undefined) {
    assert.match(schema.$ref, /^#\/\$defs\/[A-Za-z0-9_-]+$/, `${context}: unsupported schema reference`);
    assert.ok(
      state.referenceDepth < fixtureLimits.schemaReferenceDepth,
      `${context}: schema validation exceeded the ${fixtureLimits.schemaReferenceDepth}-reference depth bound`,
    );
    assert.equal(
      state.activeReferences.has(schema.$ref),
      false,
      `${context}: schema reference cycle includes ${schema.$ref}`,
    );
    const validated = state.validatedReferences.get(schema.$ref)?.get(value);
    if (validated !== undefined && state.referenceDepth <= validated.depth) {
      if (validated.error !== undefined) throw validated.error;
      return;
    }
    const definition = root.$defs?.[schema.$ref.split("/").at(-1)];
    assert.notEqual(definition, undefined, `${context}: unresolved schema reference ${schema.$ref}`);
    const activeReferences = new Set(state.activeReferences);
    activeReferences.add(schema.$ref);
    const validatedValues = state.validatedReferences.get(schema.$ref) ?? new Map();
    try {
      validateSchema(value, definition, root, context, {
        ...descend(),
        referenceDepth: state.referenceDepth + 1,
        activeReferences,
      });
      validatedValues.set(value, { depth: state.referenceDepth });
      state.validatedReferences.set(schema.$ref, validatedValues);
    } catch (error) {
      if (!/schema (?:reference cycle|validation exceeded)|(?:unresolved|unsupported) schema reference/.test(error.message)) {
        validatedValues.set(value, { depth: state.referenceDepth, error });
        state.validatedReferences.set(schema.$ref, validatedValues);
      }
      throw error;
    }
    return;
  }
  if (schema.oneOf !== undefined) {
    const matches = schema.oneOf.filter((candidate) => {
      try {
        validateSchema(value, candidate, root, context, descend());
        return true;
      } catch (error) {
        if (/schema (?:reference cycle|validation exceeded)|(?:unresolved|unsupported) schema reference/.test(error.message)) {
          throw error;
        }
        return false;
      }
    });
    assert.equal(matches.length, 1, `${context}: value does not match exactly one published schema branch`);
    return;
  }
  if (schema.const !== undefined) assert.deepEqual(value, schema.const, `${context}: constant value does not match schema`);
  if (schema.enum !== undefined) assert.ok(schema.enum.includes(value), `${context}: value is outside the schema enum`);
  if (schema.type === "object") {
    assert.equal(value !== null && typeof value === "object" && !Array.isArray(value), true, `${context}: expected schema object`);
    for (const field of schema.required ?? []) assert.ok(Object.hasOwn(value, field), `${context}: missing schema field ${field}`);
    if (schema.additionalProperties === false) {
      for (const field of Object.keys(value)) assert.ok(Object.hasOwn(schema.properties ?? {}, field), `${context}: unexpected schema field ${field}`);
    }
    for (const [field, fieldValue] of Object.entries(value)) {
      if (schema.properties?.[field] !== undefined) {
        validateSchema(fieldValue, schema.properties[field], root, `${context}.${field}`, descend());
      }
    }
  } else if (schema.type === "array") {
    assert.ok(Array.isArray(value), `${context}: expected schema array`);
    if (schema.minItems !== undefined) assert.ok(value.length >= schema.minItems, `${context}: array is shorter than schema minimum`);
    if (schema.maxItems !== undefined) assert.ok(value.length <= schema.maxItems, `${context}: array is longer than schema maximum`);
    for (const [index, item] of value.entries()) {
      validateSchema(item, schema.items, root, `${context}[${index}]`, descend());
    }
  } else if (schema.type === "string") {
    assert.equal(typeof value, "string", `${context}: expected schema string`);
    if (schema.minLength !== undefined) assert.ok([...value].length >= schema.minLength, `${context}: string is shorter than schema minimum`);
    if (schema.maxLength !== undefined) assert.ok([...value].length <= schema.maxLength, `${context}: string is longer than schema maximum`);
  } else if (schema.type === "boolean") {
    assert.equal(typeof value, "boolean", `${context}: expected schema boolean`);
  } else if (schema.type === "integer") {
    assert.equal(Number.isInteger(value), true, `${context}: expected schema integer`);
    if (schema.minimum !== undefined) assert.ok(value >= schema.minimum, `${context}: integer is below schema minimum`);
    if (schema.maximum !== undefined) assert.ok(value <= schema.maximum, `${context}: integer is above schema maximum`);
  }
}

function loadToolSchemas(repositoryRoot) {
  const directory = join(repositoryRoot, "crates", "veln-mcp", "schemas", "mcp", "v1");
  const load = (name) => JSON.parse(readFileSync(join(directory, `${name}.json`), "utf8"));
  return {
    searchInput: load("search-docs-input"),
    searchResult: load("search-docs-result"),
    readInput: load("read-doc-input"),
    readResult: load("read-doc-result"),
  };
}

function validateSearchResult(result, context) {
  assert.equal(result.scope, "language", `${context}: result scope must remain language`);
  assert.ok(Array.isArray(result.results), `${context}: search results must be an array`);
  assert.ok(result.results.length <= fixtureLimits.searchResults, `${context}: search result count exceeds the fixture limit`);
  for (const candidate of result.results) {
    assert.match(candidate.uri ?? "", snapshotTopicUri, `${context}: search URI must use a canonical snapshot digest`);
    for (const field of ["title", "summary", "excerpt"]) {
      assert.equal(typeof candidate[field], "string", `${context}: search result ${field} is required`);
    }
    for (const field of ["prefix_truncated", "suffix_truncated"]) {
      assert.equal(typeof candidate[field], "boolean", `${context}: search result ${field} is required`);
    }
  }
  return result.results;
}

function validateReadResult(result, selectedUri, context) {
  assert.equal(result.uri, selectedUri, `${context}: read result changed the selected URI`);
  for (const field of ["name", "title", "text"]) {
    assert.equal(typeof result[field], "string", `${context}: read result ${field} is required`);
  }
  if (result.description !== undefined) {
    assert.equal(typeof result.description, "string", `${context}: read result description must be text`);
  }
  assert.equal(result.mimeType, markdownMimeType, `${context}: read result mimeType is invalid`);
}

function validateToolEnvelope(value, schema, expectedIsError, context) {
  assertExactKeys(value, ["content", "structuredContent", "isError"], `${context}: MCP tool envelope`);
  assert.equal(value.isError, expectedIsError, `${context}: MCP tool envelope has the wrong error state`);
  assert.ok(Array.isArray(value.content), `${context}: MCP tool content must be an array`);
  assert.equal(value.content.length, 1, `${context}: MCP tool content must contain one text item`);
  assertExactKeys(value.content[0], ["type", "text"], `${context}: MCP tool text content`);
  assert.equal(value.content[0].type, "text", `${context}: MCP tool content must be text`);
  assert.equal(typeof value.content[0].text, "string", `${context}: MCP tool text is required`);
  assert.deepEqual(
    JSON.parse(value.content[0].text),
    value.structuredContent,
    `${context}: MCP tool text must encode structuredContent`,
  );
  validateSchema(value.structuredContent, schema, schema, `${context}: structuredContent`);
  return value.structuredContent;
}

function validateFailure(answer, operation, artifactUri, previousResult, context) {
  assert.deepEqual(answer.failure, { operation, artifact_uri: artifactUri }, `${context}: failure must identify the failed operation and artifact`);
  validateClaims(answer, undefined, undefined, context);
  assert.deepEqual(answer.retained_result, previousResult, `${context}: earlier result changed`);
}

function validateLanguageTurn(turn, previousResult, contract, schemas, published, snapshots, context) {
  const { events } = turn;
  assert.ok(turn.request.text.length <= fixtureLimits.requestCharacters, `${context}: request exceeds the fixture text limit`);
  for (const event of events) {
    assert.ok(["call", "result", "answer"].includes(event.type), `${context}: language route contains forbidden fallback event ${event.type}`);
  }
  const calls = events.filter((event) => event.type === "call");
  assert.ok(events.length <= fixtureLimits.eventsPerTurn, `${context}: turn exceeds the event limit`);
  assert.ok(calls.length <= contract.language.maximum_calls, `${context}: language route exceeded its call bound`);
  assert.equal(events[0]?.type, "call", `${context}: language route must start with a call`);
  assertExactKeys(events[0], ["type", "tool", "arguments"], `${context}: search call event`);
  assert.equal(events[0]?.tool, contract.language.search_tool, `${context}: language route must search first`);
  validateSchema(events[0]?.arguments, schemas.searchInput, schemas.searchInput, `${context}: search_docs input`);
  assert.equal(events[0]?.arguments?.scope, contract.language.search_scope, `${context}: search scope must be language`);
  assertExactKeys(turn.expected, ["search_arguments", "answer_claims"], `${context}: language expectation`);
  const selection = expectedSelection(turn.request.text, "language", context);
  assert.deepEqual(turn.expected.search_arguments, selection.searchArguments, `${context}: fixture expectation does not follow the request`);
  assert.deepEqual(
    events[0]?.arguments,
    selection.searchArguments,
    `${context}: search request must match request-selected evidence`,
  );
  assert.equal(events[1]?.type, "result", `${context}: search result must follow search call`);
  assert.equal(events[1]?.tool, contract.language.search_tool, `${context}: expected recorded search result`);
  const answer = finalAnswer(events, context);
  const searchResult = events[1];
  assert.equal(Object.hasOwn(searchResult, "error") !== Object.hasOwn(searchResult, "value"), true, `${context}: result event must contain exactly one of error or value`);
  assertExactKeys(searchResult, searchResult.error === undefined
    ? ["type", "tool", "value"]
    : ["type", "tool", "error"], `${context}: search result event`);
  assert.ok(Array.isArray(turn.expected.answer_claims), `${context}: expected answer claims must be an array`);
  if (answer.status !== "answered") {
    assert.deepEqual(turn.expected.answer_claims, [], `${context}: bounded outcome must not expect language claims`);
  }

  if (searchResult.error !== undefined) {
    assert.equal(events.length, 3, `${context}: unavailable search must stop without retry or read`);
    assertExactKeys(searchResult.error, ["code"], `${context}: search transport error`);
    assert.equal(typeof searchResult.error.code, "string", `${context}: search error code is required`);
    assert.ok(searchResult.error.code.length > 0, `${context}: search error code must not be empty`);
    assert.equal(answer.status, "search_unavailable", `${context}: wrong unavailable-search status`);
    assertExactKeys(answer, ["type", "status", "claims", "source_uris", "failure", "retained_result"], `${context}: search failure answer`);
    validateFailure(answer, "search_docs", null, previousResult, context);
    return previousResult;
  }

  const searchStructured = validateToolEnvelope(
    searchResult.value,
    schemas.searchResult,
    false,
    `${context}: search_docs result`,
  );
  const results = validateSearchResult(searchStructured, context);
  const resultDigests = new Set(results.map((result) => snapshotDigest(result.uri, context)));
  assert.ok(resultDigests.size <= 1, `${context}: search results must belong to one snapshot`);
  const resultDigest = resultDigests.values().next().value ?? published.digest;
  const snapshotEvidence = resultDigest === published.digest ? undefined : snapshots.get(resultDigest);
  assert.ok(resultDigest === published.digest || snapshotEvidence,
    `${context}: search results have no checked snapshot evidence`);
  const searchEvidence = resultDigest === published.digest
    ? published
    : {
      digest: resultDigest,
      catalog: materializeSnapshotCatalog(published, snapshotEvidence, `${context}: checked snapshot`),
      caseFoldMappings: published.caseFoldMappings,
    };
  assert.deepEqual(
    searchStructured,
    expectedPublishedSearch(events[0].arguments, searchEvidence),
    `${context}: recorded search_docs result differs from the checked snapshot artifact`,
  );
  if (selection.topic === undefined) {
    assert.equal(results.length, 0, `${context}: request-selected topic absence must not replay a match`);
  } else if (results.length > 0 && resultDigest === published.digest) {
    assert.ok(results.some((result) => result.uri.endsWith(`/topic/${selection.topic}`)), `${context}: search results do not contain the request-selected topic`);
  }
  if (results.length === 0) {
    assert.equal(events.length, 3, `${context}: no match must stop without retry or read`);
    assert.equal(answer.status, "no_match", `${context}: wrong no-match status`);
    assertExactKeys(answer, ["type", "status", "claims", "source_uris", "message"], `${context}: no-match answer`);
    assert.equal(
      answer.message,
      "The published Veln language reference has no matching topic.",
      `${context}: no-match answer must only report the published-topic absence`,
    );
    validateClaims(answer, undefined, undefined, context);
    return previousResult;
  }

  assert.equal(events.length, 5, `${context}: matching route must have one search and one read`);
  assert.equal(events[2]?.type, "call", `${context}: topic read must follow search result`);
  assertExactKeys(events[2], ["type", "tool", "arguments"], `${context}: read call event`);
  assert.equal(events[2]?.tool, contract.language.read_tool, `${context}: matching route must use read_doc`);
  validateSchema(events[2]?.arguments, schemas.readInput, schemas.readInput, `${context}: read_doc input`);
  const selectedUri = events[2]?.arguments?.uri;
  assert.ok(results.some((result) => result.uri === selectedUri), `${context}: read_doc URI must exactly match a search result`);
  assert.equal(selectedUri, results[0].uri, `${context}: read_doc must deterministically select the first search result`);
  assert.equal(events[3]?.type, "result", `${context}: topic result must follow read call`);
  assert.equal(events[3]?.tool, contract.language.read_tool, `${context}: expected recorded read result`);
  const readResult = events[3];
  assert.equal(Object.hasOwn(readResult, "error") !== Object.hasOwn(readResult, "value"), true, `${context}: result event must contain exactly one of error or value`);
  assertExactKeys(readResult, readResult.error === undefined
    ? ["type", "tool", "value"]
    : ["type", "tool", "error"], `${context}: read result event`);
  if (resultDigest === published.digest) {
    assert.ok(selectedUri.endsWith(`/topic/${selection.topic}`), `${context}: read_doc did not select the request-selected topic`);
  }

  if (readResult.error !== undefined) {
    assertExactKeys(readResult.error, ["code"], `${context}: read transport error`);
    assert.equal(typeof readResult.error.code, "string", `${context}: read error code is required`);
    assert.ok(readResult.error.code.length > 0, `${context}: read error code must not be empty`);
    assert.notEqual(
      readResult.error.code,
      "resource_not_found",
      `${context}: resource_not_found must use the checked stale-snapshot result path`,
    );
    assert.equal(answer.status, "topic_unavailable", `${context}: wrong unreadable-topic status`);
    assertExactKeys(answer, ["type", "status", "claims", "source_uris", "failure", "retained_result"], `${context}: topic failure answer`);
    validateFailure(answer, "read_doc", selectedUri, previousResult, context);
    return previousResult;
  }
  if (readResult.value?.isError === true) {
    const failure = validateToolEnvelope(
      readResult.value,
      schemas.readResult,
      true,
      `${context}: read_doc failure result`,
    );
    assert.equal(failure.code, "resource_not_found", `${context}: unsupported read_doc error`);
    assert.equal(failure.details?.uri, selectedUri, `${context}: stale error must identify the selected URI`);
    assert.equal(failure.text, undefined, `${context}: failed read must not contain partial document text`);
    const staleDigest = snapshotDigest(selectedUri, context);
    assert.notEqual(
      staleDigest,
      published.digest,
      `${context}: stale snapshot URI must differ from the checked published snapshot digest`,
    );
    assert.ok(snapshots.has(staleDigest), `${context}: stale snapshot has no checked catalog evidence`);
    assert.equal(answer.status, "stale_snapshot", `${context}: wrong stale-snapshot status`);
    assertExactKeys(answer, ["type", "status", "claims", "source_uris", "failure", "retained_result"], `${context}: stale-snapshot answer`);
    validateFailure(answer, "read_doc", selectedUri, previousResult, context);
    return previousResult;
  }

  const readStructured = validateToolEnvelope(
    readResult.value,
    schemas.readResult,
    false,
    `${context}: read_doc result`,
  );
  validateReadResult(readStructured, selectedUri, context);
  assert.ok(
    Buffer.byteLength(readStructured.text, "utf8") <= fixtureLimits.resourceTextBytes,
    `${context}: selected resource exceeds the published byte limit`,
  );
  assert.equal(
    snapshotDigest(selectedUri, context),
    published.digest,
    `${context}: successful read must use the checked published snapshot digest`,
  );
  const topicId = selectedUri.slice(selectedUri.lastIndexOf("/") + 1);
  const topic = published.catalog.topics.find((candidate) => candidate.id === topicId);
  assert.ok(topic, `${context}: selected topic is absent from the checked language-reference artifact`);
  assert.deepEqual(readStructured, {
    uri: selectedUri,
    name: topic.id,
    title: topic.title,
    description: topic.summary,
    mimeType: markdownMimeType,
    text: renderTopic(topic, published.digest),
  }, `${context}: recorded read_doc result differs from the checked language-reference artifact`);
  assert.equal(answer.status, "answered", `${context}: wrong successful status`);
  assertExactKeys(answer, ["type", "status", "claims", "source_uris"], `${context}: successful answer`);
  validateClaims(answer, selectedUri, readStructured.text, context);
  assert.deepEqual(answer.claims, turn.expected.answer_claims, `${context}: answer claims must match the scenario expectation`);
  return retainedResult(answer);
}

function checkedRepositoryPath(repositoryRoot, path, context) {
  assert.equal(typeof path, "string", `${context}: repository read path is required`);
  assert.ok(!isAbsolute(path) && !path.includes("\\"), `${context}: repository path must be relative and portable`);
  assert.equal(posix.normalize(path), path, `${context}: repository path must be normalized`);
  assert.ok(path.startsWith("docs/"), `${context}: repository authority must stay under docs/`);
  const absolute = resolve(repositoryRoot, path);
  const docsRoot = realpathSync(resolve(repositoryRoot, "docs"));
  const fromDocs = relative(docsRoot, absolute);
  assert.ok(fromDocs !== ".." && !fromDocs.startsWith(`..${posix.sep}`), `${context}: repository authority escaped docs/`);
  assert.ok(existsSync(absolute) && statSync(absolute).isFile(), `${context}: repository authority does not exist: ${path}`);
  const realFromDocs = relative(docsRoot, realpathSync(absolute));
  assert.ok(realFromDocs !== ".." && !realFromDocs.startsWith(`..${posix.sep}`), `${context}: repository authority escaped docs/`);
  return absolute;
}

export function linkedDocumentationPaths(sourcePath, repositoryRoot) {
  const absolute = resolve(repositoryRoot, sourcePath);
  assert.ok(
    statSync(absolute).size <= fixtureLimits.repositoryDocumentBytes,
    `${sourcePath}: repository document exceeds the byte limit`,
  );
  const source = navigationalMarkdown(readFileSync(absolute, "utf8"));
  const links = [];
  const openLabels = [];
  for (let cursor = 0; cursor < source.length; cursor += 1) {
    if (source[cursor] === "[" && !isBackslashEscaped(source, cursor)) {
      openLabels.push({
        image: cursor > 0 && source[cursor - 1] === "!" && !isBackslashEscaped(source, cursor - 1),
        containsLink: false,
      });
      continue;
    }
    if (source[cursor] !== "]" || isBackslashEscaped(source, cursor)) continue;
    const label = openLabels.pop();
    if (source[cursor + 1] !== "(" || label === undefined) continue;
    const destination = parseInlineLinkDestination(source, cursor + 2);
    if (destination === undefined) continue;
    const target = destination.target.split("#", 1)[0];
    cursor = destination.end;
    if (label.image) continue;
    if (openLabels.length > 0) openLabels[openLabels.length - 1].containsLink = true;
    if (label.containsLink) continue;
    if (target.length === 0 || target.includes("(") || target.includes("[")) continue;
    if (/^[a-z][a-z0-9+.-]*:/i.test(target)) continue;
    const joined = posix.normalize(posix.join(posix.dirname(sourcePath), target));
    if (joined.startsWith("docs/")) links.push(joined);
  }
  return links;
}

function parseInlineLinkDestination(source, start) {
  let cursor = start;
  while (source[cursor] === " " || source[cursor] === "\t" || source[cursor] === "\n") cursor += 1;
  const destinationStart = cursor;
  let target;
  if (source[cursor] === "<") {
    cursor += 1;
    const targetStart = cursor;
    while (cursor < source.length && source[cursor] !== ">" && source[cursor] !== "\n") {
      if (source[cursor] === "<" && !isBackslashEscaped(source, cursor)) return undefined;
      cursor += 1;
    }
    if (source[cursor] !== ">") return undefined;
    target = source.slice(targetStart, cursor);
    cursor += 1;
  } else {
    let parentheses = 0;
    let escaped = false;
    while (cursor < source.length) {
      const character = source[cursor];
      if (!escaped) {
        if (character === "(") parentheses += 1;
        if (character === ")") {
          if (parentheses === 0) break;
          parentheses -= 1;
        }
        if (/\s/u.test(character)) break;
      }
      escaped = character === "\\" ? !escaped : false;
      cursor += 1;
    }
    if (parentheses !== 0) return undefined;
    target = source.slice(destinationStart, cursor);
  }
  const destinationEnd = cursor;
  while (source[cursor] === " " || source[cursor] === "\t" || source[cursor] === "\n") cursor += 1;
  const separated = cursor > destinationEnd;
  if (source[cursor] !== ")") {
    if (!separated) return undefined;
    const opener = source[cursor];
    const closer = opener === "(" ? ")" : opener === "\"" ? "\"" : opener === "'" ? "'" : undefined;
    if (closer === undefined) return undefined;
    cursor += 1;
    let escaped = false;
    while (cursor < source.length && (source[cursor] !== closer || escaped)) {
      if (source[cursor] === "\n") return undefined;
      escaped = source[cursor] === "\\" ? !escaped : false;
      cursor += 1;
    }
    if (source[cursor] !== closer) return undefined;
    cursor += 1;
    while (source[cursor] === " " || source[cursor] === "\t" || source[cursor] === "\n") cursor += 1;
  }
  if (source[cursor] !== ")") return undefined;
  return { target, end: cursor };
}

function isBackslashEscaped(source, index) {
  let backslashes = 0;
  for (let cursor = index - 1; cursor >= 0 && source[cursor] === "\\"; cursor -= 1) {
    backslashes += 1;
  }
  return backslashes % 2 === 1;
}

function navigationalMarkdown(source) {
  const masked = source.split("");
  const mask = (start, end) => {
    for (let index = start; index < end; index += 1) {
      if (masked[index] !== "\n" && masked[index] !== "\r") masked[index] = " ";
    }
  };
  const findVisible = (token, start) => {
    let index = source.indexOf(token, start);
    while (index !== -1) {
      let visible = true;
      for (let offset = 0; offset < token.length; offset += 1) {
        if (masked[index + offset] !== token[offset]) {
          visible = false;
          break;
        }
      }
      if (visible) return index;
      index = source.indexOf(token, index + 1);
    }
    return -1;
  };
  const monotonicVisibleFinder = (token) => {
    let candidate;
    let searchCursor = 0;
    return (start) => {
      if (candidate === -1) return -1;
      if (candidate !== undefined && candidate >= start) {
        let visible = true;
        for (let offset = 0; offset < token.length; offset += 1) {
          if (masked[candidate + offset] !== token[offset]) {
            visible = false;
            break;
          }
        }
        if (visible) return candidate;
        searchCursor = candidate + 1;
      }
      searchCursor = Math.max(searchCursor, start);
      candidate = findVisible(token, searchCursor);
      searchCursor = candidate === -1 ? source.length : candidate;
      return candidate;
    };
  };

  let htmlBlock;
  let htmlLineStart = 0;
  while (htmlLineStart < source.length) {
    const newline = source.indexOf("\n", htmlLineStart);
    const lineEnd = newline === -1 ? source.length : newline + 1;
    const line = source.slice(htmlLineStart, lineEnd).replace(/[\r\n]+$/, "");
    if (htmlBlock?.untilBlank === true) {
      if (/^\s*$/u.test(line)) htmlBlock = undefined;
      else mask(htmlLineStart, lineEnd);
    } else if (htmlBlock?.closing !== undefined) {
      mask(htmlLineStart, lineEnd);
      if (htmlBlock.closing.test(line)) htmlBlock = undefined;
    } else {
      const rawTag = /^ {0,3}<(script|pre|style|textarea)(?=[\t\r\n />])/iu.exec(line);
      if (rawTag !== null) {
        const closing = new RegExp(`</${rawTag[1]}\\s*>`, "iu");
        mask(htmlLineStart, lineEnd);
        if (!closing.test(line)) htmlBlock = { closing };
      } else if (/^ {0,3}<\/?[A-Za-z][^>]*>/u.test(line)) {
        mask(htmlLineStart, lineEnd);
        htmlBlock = { untilBlank: true };
      }
    }
    htmlLineStart = lineEnd;
  }

  let fence;
  let lineStart = 0;
  while (lineStart < source.length) {
    const newline = source.indexOf("\n", lineStart);
    const lineEnd = newline === -1 ? source.length : newline + 1;
    const line = source.slice(lineStart, lineEnd).replace(/[\r\n]+$/, "");
    if (fence === undefined) {
      const opening = /^ {0,3}(`{3,}|~{3,})/.exec(line);
      if (opening !== null) {
        fence = { character: opening[1][0], length: opening[1].length };
        mask(lineStart, lineEnd);
      } else if (/^(?: {4}|\t)/.test(line)) {
        mask(lineStart, lineEnd);
      }
    } else {
      const closing = new RegExp(`^ {0,3}\\${fence.character}{${fence.length},}[ \\t]*$`);
      mask(lineStart, lineEnd);
      if (closing.test(line)) fence = undefined;
    }
    lineStart = lineEnd;
  }

  const findCommentStart = monotonicVisibleFinder("<!--");
  const tickRuns = [];
  for (let index = 0; index < source.length;) {
    if (masked[index] !== "`") {
      index += 1;
      continue;
    }
    const start = index;
    while (masked[index] === "`") index += 1;
    tickRuns.push({ start, end: index, nextMatching: undefined });
  }
  const nextRunByLength = new Map();
  for (let index = tickRuns.length - 1; index >= 0; index -= 1) {
    const run = tickRuns[index];
    const length = run.end - run.start;
    run.nextMatching = nextRunByLength.get(length);
    nextRunByLength.set(length, run);
  }

  let tickIndex = 0;
  let cursor = 0;
  while (cursor < source.length) {
    const commentStart = findCommentStart(cursor);
    while (tickIndex < tickRuns.length && (
      tickRuns[tickIndex].start < cursor
      || masked[tickRuns[tickIndex].start] !== "`"
    )) {
      tickIndex += 1;
    }
    const tick = tickRuns[tickIndex];
    const tickStart = tick?.start ?? -1;
    const start = commentStart === -1
      ? tickStart
      : tickStart === -1 ? commentStart : Math.min(commentStart, tickStart);
    if (start === -1) break;
    if (start === commentStart) {
      const marker = findVisible("-->", start + 4);
      const end = marker === -1 ? source.length : marker + 3;
      mask(start, end);
      cursor = end;
      continue;
    }
    tickIndex += 1;
    if (tick.nextMatching === undefined) {
      cursor = tick.end;
      continue;
    }
    const end = tick.nextMatching.end;
    mask(start, end);
    cursor = end;
  }
  return masked.join("");
}

export function shortestDocumentationRoute(
  entry,
  authority,
  repositoryRoot,
  maximumReads,
  maximumDiscoveryDocuments = fixtureLimits.repositoryDiscoveryDocuments,
) {
  assert.ok(
    Number.isSafeInteger(maximumDiscoveryDocuments) && maximumDiscoveryDocuments > 0,
    "repository route discovery bound must be a positive integer",
  );
  const entryIdentity = realpathSync(checkedRepositoryPath(
    repositoryRoot,
    entry,
    "repository route entry",
  ));
  const authorityIdentity = realpathSync(checkedRepositoryPath(
    repositoryRoot,
    authority,
    "repository route authority",
  ));
  const queue = [{ paths: [entry], identity: entryIdentity }];
  const visited = new Set([entryIdentity]);
  let queueIndex = 0;
  while (queueIndex < queue.length) {
    const { paths, identity } = queue[queueIndex];
    queueIndex += 1;
    const current = paths.at(-1);
    if (identity === authorityIdentity) return paths;
    if (paths.length >= maximumReads) continue;
    for (const linked of linkedDocumentationPaths(current, repositoryRoot)) {
      const linkedIdentity = realpathSync(checkedRepositoryPath(
        repositoryRoot,
        linked,
        "repository route discovery",
      ));
      if (visited.has(linkedIdentity)) continue;
      assert.ok(
        visited.size < maximumDiscoveryDocuments,
        `repository route discovery exceeded the ${maximumDiscoveryDocuments}-document bound`,
      );
      visited.add(linkedIdentity);
      if (linkedIdentity === authorityIdentity) return [...paths, linked];
      queue.push({ paths: [...paths, linked], identity: linkedIdentity });
    }
  }
  assert.fail(`repository authority is not reachable within the read bound: ${authority}`);
}

export function currentRepositoryAuthority(path, context = "repository authority") {
  const match = readFileSync(path, "utf8").match(/^---\n([\s\S]*?)\n---/);
  const role = match?.[1].match(/^role:\s*(\S+)\s*$/m)?.[1];
  const status = match?.[1].match(/^status:\s*(\S+)\s*$/m)?.[1];
  assert.equal(
    status,
    undefined,
    `${context}: terminal repository authority must be current, not lifecycle status ${status}`,
  );
  return role;
}

function validateRepositoryTurn(turn, previousResult, requirement, contract, repositoryRoot, context) {
  const { events } = turn;
  assert.ok(turn.request.text.length <= fixtureLimits.requestCharacters, `${context}: request exceeds the fixture text limit`);
  for (const event of events) {
    assert.ok(["read", "answer"].includes(event.type), `${context}: repository route contains published-reference or fallback event ${event.type}`);
  }
  const reads = events.filter((event) => event.type === "read");
  assert.ok(events.length <= fixtureLimits.eventsPerTurn, `${context}: turn exceeds the event limit`);
  const selection = expectedSelection(turn.request.text, "repository", context);
  assert.equal(requirement.authority, selection.authority, `${context}: acceptance label does not match request-selected repository authority`);
  assert.equal(reads[0]?.path, contract.repository.entry, `${context}: repository route must start at docs/README.md`);
  assert.equal(new Set(reads.map((read) => read.path)).size, reads.length, `${context}: repository route repeated a path`);
  assert.ok(reads.length <= contract.repository.maximum_reads, `${context}: repository route exceeded its read bound`);
  for (const [index, read] of reads.entries()) {
    assertExactKeys(read, ["type", "path", "value"], `${context}: repository read`);
    checkedRepositoryPath(repositoryRoot, read.path, context);
    const terminal = index === reads.length - 1;
    if (!terminal || requirement.authority === undefined) {
      assertExactKeys(read.value, ["route"], `${context}: repository routing result`);
      assert.equal(
        typeof read.value.route === "string" || (terminal && read.value.route === null),
        true,
        `${context}: repository route must be a path or an explicit terminal null`,
      );
    } else if (requirement.readyOnly) {
      assertExactKeys(read.value, ["authority", "selection"], `${context}: repository proposal result`);
      assert.equal(read.value.authority, requirement.authority, `${context}: terminal read named the wrong repository authority`);
      assert.equal(read.value.selection, "ready-only", `${context}: proposal selection must be Ready-only`);
    } else {
      assertExactKeys(read.value, ["authority"], `${context}: repository authority result`);
      assert.equal(read.value.authority, requirement.authority, `${context}: terminal read named the wrong repository authority`);
    }
    if (index > 0) {
      assert.equal(read.path, reads[index - 1].value?.route, `${context}: selected docs route was not followed`);
      assert.ok(linkedDocumentationPaths(reads[index - 1].path, repositoryRoot).includes(read.path), `${context}: repository routing page does not select ${read.path}`);
    }
  }
  const expectedPaths = requirement.authority === undefined
    ? selection.paths
    : shortestDocumentationRoute(
      contract.repository.entry,
      requirement.authority,
      repositoryRoot,
      contract.repository.maximum_reads,
    );
  assert.deepEqual(reads.map((read) => read.path), expectedPaths,
    `${context}: repository route must use the smallest task-appropriate documentation path`);
  const answer = finalAnswer(events, context);
  validateClaims(answer, undefined, undefined, context);

  if (requirement.authority === undefined) {
    assert.ok(reads.length > 1, `${context}: unknown topic must follow the fallback documentation route`);
    assert.equal(reads.at(-1).value?.route, null, `${context}: exhausted route must be explicit`);
    assert.equal(answer.status, "repository_no_route", `${context}: wrong unknown-route status`);
    assertExactKeys(answer, ["type", "status", "claims", "source_uris", "message"], `${context}: repository no-route answer`);
    assert.match(answer.message ?? "", /no repository documentation route/i, `${context}: missing repository route was not reported`);
    return previousResult;
  }

  assert.equal(reads.at(-1)?.path, requirement.authority, `${context}: acceptance row selected the wrong repository authority`);
  assert.equal(answer.status, "repository_routed", `${context}: wrong repository status`);
  assertExactKeys(answer, ["type", "status", "claims", "source_uris", "repository_authority"], `${context}: repository answer`);
  assert.equal(answer.repository_authority, requirement.authority, `${context}: answer named the wrong authority`);
  const role = currentRepositoryAuthority(
    checkedRepositoryPath(repositoryRoot, requirement.authority, context),
    context,
  );
  if (requirement.authority === "docs/proposals/README.md") {
    assert.equal(role, "routing", `${context}: proposal request must use the proposal catalog route`);
  } else {
    assert.ok(
      ["specification", "proposal", "reference"].includes(role),
      `${context}: terminal repository authority must be current specification, proposal, or reference`,
    );
  }
  return previousResult;
}

export function validateScenarioDocument(document, options = {}) {
  const contract = loadSkillContract(options);
  const repositoryRoot = options.repositoryRoot ?? defaultRepositoryRoot;
  const schemas = loadToolSchemas(repositoryRoot);
  const published = loadPublishedLanguageReference(repositoryRoot);
  const snapshots = loadSnapshotEvidence(repositoryRoot, published);
  for (const path of contract.maintenance) {
    checkedRepositoryPath(repositoryRoot, path, "veln-language maintenance contract");
  }
  assert.equal(document.schema_version, 1, "unsupported veln-language scenario schema");
  assert.ok(Array.isArray(document.request_selection), "request-selection evidence must be an array");
  assert.ok(
    document.request_selection.length <= fixtureLimits.requestSelectionCases,
    "request-selection evidence exceeds the case limit",
  );
  const requestSelectionOracle = loadRequestSelectionOracle();
  const requestSelectionIds = new Set();
  const requestSelectionTexts = new Set();
  const semanticCoverage = new Set();
  for (const selection of document.request_selection) {
    assertExactKeys(selection, ["id", "action", "subject", "text", "route"], "request-selection case");
    assert.equal(typeof selection.id, "string", "request-selection case id must be a string");
    assert.ok(selection.id.length > 0, "request-selection case id must not be empty");
    assert.equal(requestSelectionIds.has(selection.id), false, `duplicate request-selection case ${selection.id}`);
    requestSelectionIds.add(selection.id);
    assert.ok(["language", "repository"].includes(selection.route), `${selection.id}: invalid expected route`);
    assert.ok(selection.text.length <= fixtureLimits.requestCharacters, `${selection.id}: request exceeds the fixture text limit`);
    assert.equal(requestSelectionTexts.has(selection.text), false, `${selection.id}: duplicate request-selection text`);
    requestSelectionTexts.add(selection.text);
  }
  assert.deepEqual(
    [...requestSelectionIds].sort(),
    [...requestSelectionOracle.keys()].sort(),
    "update request-selection IDs to exactly match the canonical corpus; complete membership prevents incomplete or fabricated routing evidence",
  );
  for (const selection of document.request_selection) {
    const expected = requestSelectionOracle.get(selection.id);
    assert.deepEqual(
      requestSelectionRecord(selection),
      expected,
      `${selection.id}: text and semantic classifications differ from the independent corpus oracle`,
    );
    assert.equal(
      routeRequestSemantics({ action: expected.action, subject: expected.subject }, selection.id),
      selection.route,
      `${selection.id}: request semantics selected the wrong route`,
    );
    semanticCoverage.add(`${expected.action}:${expected.subject}:${selection.route}`);
  }
  assert.deepEqual(
    semanticCoverage,
    new Set([
      "information:language_behavior:language",
      "information:implementation:repository",
      "information:repository_material:repository",
      "repository_action:implementation:repository",
      "repository_action:language_behavior:repository",
      "repository_action:repository_material:repository",
    ]),
    "request-selection evidence must cover every action-and-subject routing class",
  );
  const requestSelectionByText = new Map(
    document.request_selection.map((selection) => [selection.text, selection]),
  );
  for (const [text, route] of [
    ["Please assess the Veln parser.", "repository"],
    ["Audit the Veln lexer.", "repository"],
    ["Assess how effects are handled in Veln.", "repository"],
    ["Validate how Veln schemas are parsed.", "repository"],
    ["Alter how effects are handled in Veln.", "repository"],
    ["How are effects handled in Veln?", "language"],
    ["Explain how effects are handled in Veln.", "language"],
    ["What does validate mean in Veln schemas?", "language"],
    ["How does Veln alter effects?", "language"],
    ["Where is the Veln parser implemented?", "repository"],
  ]) {
    assert.equal(
      requestSelectionByText.get(text)?.route,
      route,
      `request-selection evidence must include the semantic contrast ${JSON.stringify(text)}`,
    );
  }
  assert.ok(Array.isArray(document.scenarios), "scenarios must be an array");
  assert.ok(document.scenarios.length <= fixtureLimits.scenarios, "scenario document exceeds the scenario limit");
  const coverage = new Set(document.scenarios.map((scenario) => scenario.covers));
  assert.deepEqual(coverage, new Set(acceptance.keys()), "scenario coverage does not match the acceptance model");
  assert.equal(coverage.size, document.scenarios.length, "scenario coverage entries must be unique");

  for (const scenario of document.scenarios) {
    const requirement = acceptance.get(scenario.covers);
    assert.ok(requirement, `${scenario.id}: unknown acceptance row`);
    assert.ok(Array.isArray(scenario.turns) && scenario.turns.length > 0, `${scenario.id}: turns are required`);
    assert.ok(scenario.turns.length <= fixtureLimits.turnsPerScenario, `${scenario.id}: scenario exceeds the turn limit`);
    assert.equal(scenario.turns.at(-1).events.at(-1).status, requirement.finalStatus, `${scenario.id}: acceptance row has the wrong final outcome`);
    let lastSuccessfulResult;
    for (const [index, turn] of scenario.turns.entries()) {
      const context = `${scenario.id} turn ${index + 1}`;
      const selection = requestSelectionByText.get(turn.request?.text);
      assert.ok(selection, `${context}: request has no independent semantic annotation`);
      const expected = requestSelectionOracle.get(selection.id);
      assert.equal(expected.text, turn.request.text, `${context}: recorded request differs from the independent corpus oracle`);
      const route = routeRequestSemantics({ action: expected.action, subject: expected.subject }, context);
      assert.equal(route, selection.route, `${context}: reviewed request semantics selected a route inconsistent with its corpus row`);
      assert.equal(route, requirement.route, `${context}: reviewed request semantics selected the wrong route for ${scenario.covers}`);
      lastSuccessfulResult = route === "language"
        ? validateLanguageTurn(turn, lastSuccessfulResult, contract, schemas, published, snapshots, context)
        : validateRepositoryTurn(turn, lastSuccessfulResult, requirement, contract, repositoryRoot, context);
    }
    if (requirement.failure) {
      assert.equal(scenario.turns.length, 2, `${scenario.id}: failure scenario must establish one earlier success`);
      const expected = retainedResult(scenario.turns[0].events.at(-1));
      assert.deepEqual(lastSuccessfulResult, expected, `${scenario.id}: failed turn replaced the earlier result`);
    }
  }
  return document.scenarios.length;
}

export function readScenarioDocument(path) {
  assert.ok(statSync(path).size <= fixtureLimits.fixtureBytes, "scenario fixture exceeds the byte limit");
  const bytes = readFileSync(path);
  const document = JSON.parse(bytes.toString("utf8"));
  if (document.recordings === undefined) return document;
  assertExactKeys(document, ["schema_version", "recordings", "request_selection", "scenarios"], "scenario document");
  assertExactKeys(document.recordings, ["schemas-search", "schemas-read"], "scenario recordings");
  assert.ok(Array.isArray(document.scenarios), "scenarios must be an array");
  assert.ok(Array.isArray(document.request_selection), "request-selection evidence must be an array");
  assert.ok(
    document.request_selection.length <= fixtureLimits.requestSelectionCases,
    "request-selection evidence exceeds the case limit",
  );
  assert.ok(document.scenarios.length <= fixtureLimits.scenarios, "scenario document exceeds the scenario limit");
  const referencedEvents = [];
  for (const scenario of document.scenarios) {
    assertExactKeys(scenario, ["id", "covers", "turns"], "scenario");
    assert.ok(Array.isArray(scenario.turns) && scenario.turns.length > 0, `${scenario.id}: turns are required`);
    assert.ok(scenario.turns.length <= fixtureLimits.turnsPerScenario, `${scenario.id}: scenario exceeds the turn limit`);
    for (const turn of scenario.turns) {
      const turnKeys = Object.hasOwn(turn, "expected")
        ? ["request", "expected", "events"]
        : ["request", "events"];
      assertExactKeys(turn, turnKeys, `${scenario.id}: turn`);
      assert.ok(Array.isArray(turn.events), `${scenario.id}: events are required`);
      assert.ok(turn.events.length <= fixtureLimits.eventsPerTurn, `${scenario.id}: turn exceeds the event limit`);
      for (const event of turn.events) {
        if (event.value_ref === undefined) continue;
        assert.ok(Object.hasOwn(document.recordings, event.value_ref), `unknown scenario recording ${event.value_ref}`);
        assertExactKeys(event, ["type", "tool", "value_ref"], "recorded result reference");
        referencedEvents.push(event);
      }
    }
  }
  for (const event of referencedEvents) {
    event.value = document.recordings[event.value_ref];
    delete event.value_ref;
  }
  delete document.recordings;
  return document;
}

if (process.argv[1] === scriptPath) {
  const fixturePath = join(dirname(scriptPath), "fixtures", "veln-language", "scenarios.json");
  const count = validateScenarioDocument(readScenarioDocument(fixturePath));
  console.log(`validated ${count} veln-language scenarios against the canonical skill`);
}
