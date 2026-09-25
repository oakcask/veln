#!/usr/bin/env node

import assert from "node:assert/strict";
import { existsSync, readFileSync, realpathSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, posix, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const defaultRepositoryRoot = dirname(dirname(scriptPath));
const defaultSkillPath = join(defaultRepositoryRoot, ".agents", "skills", "veln-language", "SKILL.md");

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
  ["repository-change", {
    route: "repository",
    finalStatus: "repository_routed",
    authority: "docs/specification/source-surface.md",
  }],
  ["repository-proposal", {
    route: "repository",
    finalStatus: "repository_routed",
    authority: "docs/proposals/README.md",
  }],
  ["repository-unknown", { route: "repository", finalStatus: "repository_no_route" }],
]);

const snapshotTopicUri = /^veln-doc:\/\/\/language\/snapshot\/[0-9a-f]{64}\/topic\/[a-z0-9]+(?:-[a-z0-9]+)*$/;
const markdownMimeType = "text/markdown; charset=utf-8";
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

function parseSkillContract(skillText) {
  const frontmatter = skillText.match(/^---\n([\s\S]*?)\n---/);
  assert.match(frontmatter?.[1] ?? "", /^name:\s*veln-language\s*$/m, "canonical skill name must be veln-language");
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
  assert.equal(contract.schema_version, 1, "unsupported veln-language skill contract");
  assert.deepEqual(contract.request_selection, {
    repository_when: "The request asks to inspect or change the Veln repository, its implementation, or its proposal state.",
    language_otherwise: true,
  }, "veln-language request-selection contract is inconsistent");
  assert.deepEqual(contract.language, {
    search_tool: "search_docs",
    search_scope: "language",
    read_tool: "read_doc",
    maximum_calls: 2,
    call_order: ["search_docs", "read_doc"],
    read_exact_search_result_uri: true,
    fallback: "forbidden",
    answer_source: "selected_resource_uri",
    report_selected_uri: true,
    no_match: "Report that the published Veln language reference has no matching topic. Do not use proposal text or model memory.",
  }, "veln-language skill language contract is inconsistent");
  assert.equal(contract.repository?.entry, "docs/README.md", "skill must start repository tasks at docs/README.md");
  assert.equal(contract.repository?.follow_selected_links, true, "skill must follow repository documentation links");
  assert.equal(contract.repository?.maximum_reads, 3, "skill must bound repository documentation reads");
  assert.equal(contract.repository?.repeat_paths, "forbidden", "skill must reject repository documentation cycles");
  assert.equal(contract.repository?.published_reference_is_authority, false, "skill must reject published-reference repository authority");
  assert.deepEqual(contract.repository?.explicit_targets, [
    "repository",
    "codebase",
    "source code",
    "proposal state",
  ], "skill must define explicit repository targets");
  assert.deepEqual(contract.repository?.intent_verbs, [
    "add",
    "change",
    "debug",
    "fix",
    "implement",
    "inspect",
    "modify",
    "refactor",
    "remove",
    "review",
    "select",
    "test",
    "update",
  ], "skill must define repository inspection and change intents");
  assert.deepEqual(contract.repository?.language_complements, [
    "how",
    "what",
    "when",
    "where",
    "whether",
    "why",
  ], "skill must preserve language questions phrased with an inspection verb");
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

function routeRequest(text, contract, context) {
  assert.equal(typeof text, "string", `${context}: request text is required`);
  assert.ok(text.trim().length > 0, `${context}: request text must not be empty`);
  const lower = text.toLocaleLowerCase("en-US");
  const containsTerm = (term) => {
    const escaped = term.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return new RegExp(`\\b${escaped}\\b`, "u").test(lower);
  };
  const repositoryPath = contract.repository.path_prefixes.some((prefix) => lower.includes(prefix));
  const explicitTarget = contract.repository.explicit_targets.some(containsTerm);
  const intent = contract.repository.intent_verbs.find((verb) => new RegExp(
    `^(?:please\\s+|can you\\s+|could you\\s+|would you\\s+)?${verb}\\b`,
    "u",
  ).test(lower.trim()));
  const languageComplement = intent !== undefined && contract.repository.language_complements.some((term) => new RegExp(
    `^(?:please\\s+|can you\\s+|could you\\s+|would you\\s+)?${intent}\\s+${term}\\b`,
    "u",
  ).test(lower.trim()));
  const repository = repositoryPath || explicitTarget || (intent !== undefined && !languageComplement);
  return repository ? "repository" : "language";
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
  assert.deepEqual(answer.source_uris, [expectedSource], `${context}: answer must report only the exact selected URI`);
  const evidence = resourceText
    .split(/(?<=[.!?])(?:\s+|$)|\n+/u)
    .map((statement) => statement.trim())
    .filter(Boolean);
  for (const claim of answer.claims) {
    assert.equal(typeof claim, "string", `${context}: every claim must be text`);
    assert.ok(claim.trim().length > 0, `${context}: claims must not be empty`);
    assert.ok(evidence.includes(claim), `${context}: claim must match unambiguous selected-resource evidence`);
  }
}

function validateSchema(value, schema, root, context) {
  if (schema.$ref !== undefined) {
    assert.match(schema.$ref, /^#\/\$defs\/[A-Za-z0-9_-]+$/, `${context}: unsupported schema reference`);
    return validateSchema(value, root.$defs[schema.$ref.split("/").at(-1)], root, context);
  }
  if (schema.oneOf !== undefined) {
    const matches = schema.oneOf.filter((candidate) => {
      try {
        validateSchema(value, candidate, root, context);
        return true;
      } catch {
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
      if (schema.properties?.[field] !== undefined) validateSchema(fieldValue, schema.properties[field], root, `${context}.${field}`);
    }
  } else if (schema.type === "array") {
    assert.ok(Array.isArray(value), `${context}: expected schema array`);
    for (const [index, item] of value.entries()) validateSchema(item, schema.items, root, `${context}[${index}]`);
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

function validateLanguageTurn(turn, previousResult, contract, schemas, context) {
  const { events } = turn;
  for (const event of events) {
    assert.ok(["call", "result", "answer"].includes(event.type), `${context}: language route contains forbidden fallback event ${event.type}`);
  }
  const calls = events.filter((event) => event.type === "call");
  assert.ok(calls.length <= contract.language.maximum_calls, `${context}: language route exceeded its call bound`);
  assert.equal(events[0]?.type, "call", `${context}: language route must start with a call`);
  assert.equal(events[0]?.tool, contract.language.search_tool, `${context}: language route must search first`);
  validateSchema(events[0]?.arguments, schemas.searchInput, schemas.searchInput, `${context}: search_docs input`);
  assert.equal(events[0]?.arguments?.scope, contract.language.search_scope, `${context}: search scope must be language`);
  assertExactKeys(turn.expected, ["search_arguments", "answer_claims"], `${context}: language expectation`);
  assert.deepEqual(
    events[0]?.arguments,
    turn.expected.search_arguments,
    `${context}: search request must match the scenario expectation`,
  );
  assert.equal(events[1]?.type, "result", `${context}: search result must follow search call`);
  assert.equal(events[1]?.tool, contract.language.search_tool, `${context}: expected recorded search result`);
  const answer = finalAnswer(events, context);
  const searchResult = events[1];
  assert.ok(Array.isArray(turn.expected.answer_claims), `${context}: expected answer claims must be an array`);
  if (answer.status !== "answered") {
    assert.deepEqual(turn.expected.answer_claims, [], `${context}: bounded outcome must not expect language claims`);
  }

  if (searchResult.error !== undefined) {
    assert.equal(events.length, 3, `${context}: unavailable search must stop without retry or read`);
    assert.equal(typeof searchResult.error.code, "string", `${context}: search error code is required`);
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
  assert.equal(events[2]?.tool, contract.language.read_tool, `${context}: matching route must use read_doc`);
  validateSchema(events[2]?.arguments, schemas.readInput, schemas.readInput, `${context}: read_doc input`);
  const selectedUri = events[2]?.arguments?.uri;
  assert.ok(results.some((result) => result.uri === selectedUri), `${context}: read_doc URI must exactly match a search result`);
  assert.equal(events[3]?.type, "result", `${context}: topic result must follow read call`);
  assert.equal(events[3]?.tool, contract.language.read_tool, `${context}: expected recorded read result`);
  const readResult = events[3];

  if (readResult.error !== undefined) {
    assert.equal(typeof readResult.error.code, "string", `${context}: read error code is required`);
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

function linkedDocumentationPaths(sourcePath, repositoryRoot) {
  const source = readFileSync(resolve(repositoryRoot, sourcePath), "utf8");
  const links = [];
  for (const match of source.matchAll(/\[[^\]]+\]\(([^)#]+)(?:#[^)]+)?\)/g)) {
    const target = match[1];
    if (/^[a-z][a-z0-9+.-]*:/i.test(target)) continue;
    const joined = posix.normalize(posix.join(posix.dirname(sourcePath), target));
    if (joined.startsWith("docs/")) links.push(joined);
  }
  return links;
}

function frontmatterRole(path) {
  const match = readFileSync(path, "utf8").match(/^---\n([\s\S]*?)\n---/);
  return match?.[1].match(/^role:\s*(\S+)\s*$/m)?.[1];
}

function validateRepositoryTurn(turn, previousResult, requirement, contract, repositoryRoot, context) {
  const { events } = turn;
  for (const event of events) {
    assert.ok(["read", "answer"].includes(event.type), `${context}: repository route contains published-reference or fallback event ${event.type}`);
  }
  const reads = events.filter((event) => event.type === "read");
  assert.equal(reads[0]?.path, contract.repository.entry, `${context}: repository route must start at docs/README.md`);
  assert.equal(new Set(reads.map((read) => read.path)).size, reads.length, `${context}: repository route repeated a path`);
  assert.ok(reads.length <= contract.repository.maximum_reads, `${context}: repository route exceeded its read bound`);
  for (const [index, read] of reads.entries()) {
    checkedRepositoryPath(repositoryRoot, read.path, context);
    if (index > 0) {
      assert.equal(read.path, reads[index - 1].value?.route, `${context}: selected docs route was not followed`);
      assert.ok(linkedDocumentationPaths(reads[index - 1].path, repositoryRoot).includes(read.path), `${context}: repository routing page does not select ${read.path}`);
    }
  }
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
  const role = frontmatterRole(checkedRepositoryPath(repositoryRoot, requirement.authority, context));
  if (requirement.authority.startsWith("docs/specification/")) {
    assert.equal(role, "specification", `${context}: implemented behavior must use specification authority`);
  } else {
    assert.equal(role, "routing", `${context}: proposal selection must use the proposal catalog route`);
    assert.equal(reads.at(-1).value?.selection, "ready-only", `${context}: proposal selection must be Ready-only`);
  }
  return previousResult;
}

export function validateScenarioDocument(document, options = {}) {
  const contract = loadSkillContract(options);
  const repositoryRoot = options.repositoryRoot ?? defaultRepositoryRoot;
  const schemas = loadToolSchemas(repositoryRoot);
  for (const path of contract.maintenance) {
    checkedRepositoryPath(repositoryRoot, path, "veln-language maintenance contract");
  }
  assert.equal(document.schema_version, 1, "unsupported veln-language scenario schema");
  assert.ok(Array.isArray(document.scenarios), "scenarios must be an array");
  const coverage = new Set(document.scenarios.map((scenario) => scenario.covers));
  assert.deepEqual(coverage, new Set(acceptance.keys()), "scenario coverage does not match the acceptance model");
  assert.equal(coverage.size, document.scenarios.length, "scenario coverage entries must be unique");

  for (const scenario of document.scenarios) {
    const requirement = acceptance.get(scenario.covers);
    assert.ok(requirement, `${scenario.id}: unknown acceptance row`);
    assert.ok(Array.isArray(scenario.turns) && scenario.turns.length > 0, `${scenario.id}: turns are required`);
    assert.equal(scenario.turns.at(-1).events.at(-1).status, requirement.finalStatus, `${scenario.id}: acceptance row has the wrong final outcome`);
    let lastSuccessfulResult;
    for (const [index, turn] of scenario.turns.entries()) {
      const context = `${scenario.id} turn ${index + 1}`;
      const route = routeRequest(turn.request?.text, contract, context);
      assert.equal(route, requirement.route, `${context}: request text selected the wrong route for ${scenario.covers}`);
      lastSuccessfulResult = route === "language"
        ? validateLanguageTurn(turn, lastSuccessfulResult, contract, schemas, context)
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
  return JSON.parse(readFileSync(path, "utf8"));
}

if (process.argv[1] === scriptPath) {
  const fixturePath = join(dirname(scriptPath), "fixtures", "veln-language", "scenarios.json");
  const count = validateScenarioDocument(readScenarioDocument(fixturePath));
  console.log(`validated ${count} veln-language scenarios against the canonical skill`);
}
