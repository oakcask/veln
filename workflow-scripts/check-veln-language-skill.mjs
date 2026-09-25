#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const requiredCoverage = new Set([
  "language-match",
  "language-no-match",
  "search-unavailable",
  "topic-unreadable",
  "stale-snapshot-uri",
  "repository-current",
  "repository-proposal",
  "repository-unknown",
]);
const failureCoverage = new Set([
  "search-unavailable",
  "topic-unreadable",
  "stale-snapshot-uri",
]);

function finalAnswer(events, context) {
  assert.equal(events.at(-1)?.type, "answer", `${context}: answer must be the final event`);
  assert.equal(
    events.filter((event) => event.type === "answer").length,
    1,
    `${context}: expected one answer`,
  );
  return events.at(-1);
}

function retainedResult(answer) {
  return {
    status: answer.status,
    claims: structuredClone(answer.claims),
    source_uris: structuredClone(answer.source_uris),
  };
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
  assert.deepEqual(
    answer.source_uris,
    [expectedSource],
    `${context}: answer must report only the exact selected URI`,
  );
  for (const claim of answer.claims) {
    assert.ok(
      resourceText.includes(claim),
      `${context}: claim must be supported by the selected resource text`,
    );
  }
}

function validateLanguageTurn(turn, previousResult, context) {
  const { events } = turn;
  for (const event of events) {
    assert.ok(
      ["call", "result", "answer"].includes(event.type),
      `${context}: language route contains forbidden fallback event ${event.type}`,
    );
  }
  const calls = events.filter((event) => event.type === "call");
  assert.ok(calls.length <= 2, `${context}: language route exceeded its call bound`);
  assert.equal(events[0]?.type, "call", `${context}: language route must start with a call`);
  assert.equal(events[0]?.tool, "search_docs", `${context}: language route must search first`);
  assert.equal(events[0]?.arguments?.scope, "language", `${context}: search scope must be language`);
  assert.equal(events[1]?.type, "result", `${context}: search result must follow search call`);
  assert.equal(events[1]?.tool, "search_docs", `${context}: expected recorded search result`);
  const answer = finalAnswer(events, context);
  const searchResult = events[1];

  if (searchResult.error !== undefined) {
    assert.equal(events.length, 3, `${context}: unavailable search must stop without retry or read`);
    assert.equal(answer.status, "search_unavailable", `${context}: wrong unavailable-search status`);
    assert.match(answer.message ?? "", /search.*unavailable/i, `${context}: search failure was not reported`);
    validateClaims(answer, undefined, undefined, context);
    assert.deepEqual(answer.retained_result, previousResult, `${context}: earlier result changed`);
    return previousResult;
  }

  assert.equal(searchResult.value?.scope, "language", `${context}: result scope must remain language`);
  const results = searchResult.value?.results;
  assert.ok(Array.isArray(results), `${context}: search results must be an array`);
  if (results.length === 0) {
    assert.equal(events.length, 3, `${context}: no match must stop without retry or read`);
    assert.equal(answer.status, "no_match", `${context}: wrong no-match status`);
    assert.match(answer.message ?? "", /published.*no matching topic/i, `${context}: published-topic absence was not reported`);
    validateClaims(answer, undefined, undefined, context);
    return previousResult;
  }

  assert.equal(events.length, 5, `${context}: matching route must have one search and one read`);
  assert.equal(events[2]?.type, "call", `${context}: topic read must follow search result`);
  assert.equal(events[2]?.tool, "read_doc", `${context}: matching route must use read_doc`);
  const selectedUri = events[2]?.arguments?.uri;
  assert.ok(
    results.some((result) => result.uri === selectedUri),
    `${context}: read_doc URI must exactly match a search result`,
  );
  assert.equal(events[3]?.type, "result", `${context}: topic result must follow read call`);
  assert.equal(events[3]?.tool, "read_doc", `${context}: expected recorded read result`);
  const readResult = events[3];

  if (readResult.error !== undefined) {
    assert.equal(answer.status, "topic_unavailable", `${context}: wrong unreadable-topic status`);
    assert.match(answer.message ?? "", /topic.*unavailable/i, `${context}: unreadable topic was not reported`);
    validateClaims(answer, undefined, undefined, context);
    assert.deepEqual(answer.retained_result, previousResult, `${context}: earlier result changed`);
    return previousResult;
  }
  if (readResult.value?.isError === true) {
    assert.equal(
      readResult.value.structuredContent?.code,
      "resource_not_found",
      `${context}: unsupported read_doc error`,
    );
    assert.equal(
      readResult.value.structuredContent?.details?.uri,
      selectedUri,
      `${context}: stale error must identify the selected URI`,
    );
    assert.equal(answer.status, "stale_snapshot", `${context}: wrong stale-snapshot status`);
    assert.match(answer.message ?? "", /snapshot.*stale/i, `${context}: stale snapshot was not reported`);
    validateClaims(answer, undefined, undefined, context);
    assert.deepEqual(answer.retained_result, previousResult, `${context}: earlier result changed`);
    return previousResult;
  }

  assert.equal(readResult.value?.uri, selectedUri, `${context}: read result changed the selected URI`);
  assert.equal(answer.status, "answered", `${context}: wrong successful status`);
  validateClaims(answer, selectedUri, readResult.value?.text ?? "", context);
  return retainedResult(answer);
}

function validateRepositoryTurn(turn, previousResult, coverage, context) {
  const { events } = turn;
  for (const event of events) {
    assert.ok(
      ["read", "answer"].includes(event.type),
      `${context}: repository route contains published-reference or fallback event ${event.type}`,
    );
  }
  const reads = events.filter((event) => event.type === "read");
  assert.equal(reads[0]?.path, "docs/README.md", `${context}: repository route must start at docs/README.md`);
  for (const read of reads) {
    assert.ok(read.path.startsWith("docs/"), `${context}: repository authority must stay under docs/`);
  }
  const answer = finalAnswer(events, context);
  validateClaims(answer, undefined, undefined, context);

  if (coverage === "repository-unknown") {
    assert.equal(reads.length, 1, `${context}: unknown route must stop at docs/README.md`);
    assert.equal(reads[0].value?.route, null, `${context}: unknown route must be explicit`);
    assert.equal(answer.status, "repository_no_route", `${context}: wrong unknown-route status`);
    assert.match(answer.message ?? "", /no repository documentation route/i, `${context}: missing repository route was not reported`);
  } else {
    assert.equal(reads.length, 2, `${context}: repository route must read one selected authority`);
    assert.equal(reads[1].path, reads[0].value?.route, `${context}: selected docs route was not followed`);
    assert.equal(answer.status, "repository_routed", `${context}: wrong repository status`);
    assert.equal(answer.repository_authority, reads[1].path, `${context}: answer named the wrong authority`);
  }
  if (coverage === "repository-current") {
    assert.ok(reads[1].path.startsWith("docs/specification/"), `${context}: current behavior must use specification authority`);
  }
  if (coverage === "repository-proposal") {
    assert.equal(reads[1].path, "docs/proposals/README.md", `${context}: proposal selection must use the proposal catalog`);
    assert.equal(reads[1].value?.selection, "ready-only", `${context}: proposal selection must be Ready-only`);
  }
  return previousResult;
}

export function validateScenarioDocument(document) {
  assert.equal(document.schema_version, 1, "unsupported veln-language scenario schema");
  assert.ok(Array.isArray(document.scenarios), "scenarios must be an array");
  const coverage = new Set(document.scenarios.map((scenario) => scenario.covers));
  assert.deepEqual(coverage, requiredCoverage, "scenario coverage does not match the acceptance model");
  assert.equal(coverage.size, document.scenarios.length, "scenario coverage entries must be unique");

  for (const scenario of document.scenarios) {
    assert.ok(Array.isArray(scenario.turns) && scenario.turns.length > 0, `${scenario.id}: turns are required`);
    let lastSuccessfulResult = scenario.initial_result === undefined
      ? undefined
      : structuredClone(scenario.initial_result);
    const initialSnapshot = structuredClone(lastSuccessfulResult);
    for (const [index, turn] of scenario.turns.entries()) {
      const context = `${scenario.id} turn ${index + 1}`;
      if (turn.request?.route === "language") {
        lastSuccessfulResult = validateLanguageTurn(turn, lastSuccessfulResult, context);
      } else if (turn.request?.route === "repository") {
        lastSuccessfulResult = validateRepositoryTurn(turn, lastSuccessfulResult, scenario.covers, context);
      } else {
        assert.fail(`${context}: request route must be language or repository`);
      }
    }
    if (failureCoverage.has(scenario.covers)) {
      assert.ok(initialSnapshot !== undefined || scenario.turns.length > 1, `${scenario.id}: failure requires an earlier result`);
      const expected = initialSnapshot ?? retainedResult(scenario.turns[0].events.at(-1));
      assert.deepEqual(lastSuccessfulResult, expected, `${scenario.id}: failed turn replaced the earlier result`);
    }
  }
  return document.scenarios.length;
}

export function readScenarioDocument(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

const scriptPath = fileURLToPath(import.meta.url);
if (process.argv[1] === scriptPath) {
  const fixturePath = join(dirname(scriptPath), "fixtures", "veln-language", "scenarios.json");
  const count = validateScenarioDocument(readScenarioDocument(fixturePath));
  console.log(`validated ${count} veln-language scenarios`);
}
