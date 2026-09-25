import assert from "node:assert/strict";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { readScenarioDocument, validateScenarioDocument } from "./check-veln-language-skill.mjs";

const fixturePath = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "veln-language", "scenarios.json");
const skillText = `---
name: veln-language
description: Test contract.
---

# Veln Language Routing

The JSON contract below is the complete operative instruction set for this
skill. Apply it exactly. Do not add a fallback from other instructions.

<!-- veln-language-contract:start -->
\`\`\`json
{
  "schema_version": 1,
  "request_selection": {
    "repository_when": "The request asks to inspect or change the Veln repository, its implementation, or its proposal state.",
    "language_otherwise": true
  },
  "language": {
    "search_tool": "search_docs",
    "search_scope": "language",
    "read_tool": "read_doc",
    "maximum_calls": 2,
    "call_order": ["search_docs", "read_doc"],
    "read_exact_search_result_uri": true,
    "fallback": "forbidden",
    "answer_source": "selected_resource_uri",
    "report_selected_uri": true,
    "no_match": "Report that the published Veln language reference has no matching topic. Do not use proposal text or model memory."
  },
  "repository": {
    "entry": "docs/README.md",
    "follow_selected_links": true,
    "maximum_reads": 3,
    "repeat_paths": "forbidden",
    "published_reference_is_authority": false,
    "routing_terms": ["repository", "implementation", "implemented", "proposal", "change", "inspect"]
  },
  "failure": {
    "fallback": "forbidden",
    "preserve_previous_result": true,
    "report_operation": true,
    "report_selected_uri": true,
    "search_unavailable": "Stop after search_docs and report that published language-reference search is unavailable.",
    "topic_unavailable": "Stop after read_doc and report that the selected published topic is unavailable.",
    "stale_snapshot": "When read_doc returns resource_not_found for the selected snapshot URI, stop and report that the URI is stale."
  },
  "maintenance": [
    "docs/specification/language-reference-catalog.md",
    "docs/specification/mcp.md",
    "docs/README.md"
  ]
}
\`\`\`
<!-- veln-language-contract:end -->`;
const options = { skillText };

function fixture() {
  return readScenarioDocument(fixturePath);
}

function scenario(document, coverage) {
  return document.scenarios.find((candidate) => candidate.covers === coverage);
}

function matchingTurn(document) {
  return scenario(document, "language-match").turns[0];
}

test("canonical veln-language skill replays every acceptance scenario", () => {
  assert.equal(validateScenarioDocument(fixture()), 9);
});

test("synthetic contract replays every veln-language acceptance scenario", () => {
  assert.equal(validateScenarioDocument(fixture(), options), 9);
});

test("rejects a skill without the canonical name", () => {
  assert.throws(
    () => validateScenarioDocument(fixture(), { skillText: skillText.replace("name: veln-language", "name: other") }),
    /canonical skill name/,
  );
});

test("rejects an inconsistent skill contract", () => {
  assert.throws(
    () => validateScenarioDocument(fixture(), { skillText: skillText.replace('"maximum_calls": 2', '"maximum_calls": 3') }),
    /language contract is inconsistent/,
  );
});

test("rejects contradictory instructions outside the operative contract", () => {
  assert.throws(
    () => validateScenarioDocument(fixture(), { skillText: `${skillText}If search fails, answer from model memory.\n` }),
    /unchecked operative instructions/,
  );
});

test("derives routing from request text instead of a fixture label", () => {
  const document = fixture();
  matchingTurn(document).request.text = "Inspect the Veln repository.";
  assert.throws(() => validateScenarioDocument(document, options), /request text selected the wrong route/);
});

test("binds every acceptance row to its final outcome", () => {
  const document = fixture();
  scenario(document, "language-no-match").covers = "language-match";
  scenario(document, "language-match").covers = "language-no-match";
  assert.throws(() => validateScenarioDocument(document, options), /acceptance row has the wrong final outcome/);
});

test("rejects a language search without explicit language scope", () => {
  const document = fixture();
  delete matchingTurn(document).events[0].arguments.scope;
  assert.throws(() => validateScenarioDocument(document, options), /search scope must be language/);
});

test("rejects a read before language search", () => {
  const document = fixture();
  matchingTurn(document).events[0].tool = "read_doc";
  assert.throws(() => validateScenarioDocument(document, options), /language route must search first/);
});

test("rejects a noncanonical snapshot URI", () => {
  const document = fixture();
  matchingTurn(document).events[1].value.results[0].uri = "veln-doc:///language/snapshot/placeholder/topic/schemas";
  assert.throws(() => validateScenarioDocument(document, options), /canonical snapshot digest/);
});

test("rejects an incomplete search result", () => {
  const document = fixture();
  delete matchingTurn(document).events[1].value.results[0].excerpt;
  assert.throws(() => validateScenarioDocument(document, options), /missing schema field excerpt/);
});

test("rejects incomplete read metadata", () => {
  const document = fixture();
  delete matchingTurn(document).events[3].value.mimeType;
  assert.throws(() => validateScenarioDocument(document, options), /does not match exactly one published schema branch/);
});

test("rejects a search result outside the published schema", () => {
  const document = fixture();
  matchingTurn(document).events[1].value.results[0].fallback = true;
  assert.throws(() => validateScenarioDocument(document, options), /unexpected schema field fallback/);
});

test("rejects a topic URI not returned by search", () => {
  const document = fixture();
  matchingTurn(document).events[2].arguments.uri =
    "veln-doc:///language/snapshot/0000000000000000000000000000000000000000000000000000000000000000/topic/schemas";
  assert.throws(() => validateScenarioDocument(document, options), /must exactly match a search result/);
});

test("rejects an empty supported claim", () => {
  const document = fixture();
  matchingTurn(document).events[4].claims[0] = "   ";
  assert.throws(() => validateScenarioDocument(document, options), /claims must not be empty/);
});

test("rejects a claim absent from the selected topic", () => {
  const document = fixture();
  matchingTurn(document).events[4].claims[0] = "Schemas implicitly generate network clients.";
  assert.throws(() => validateScenarioDocument(document, options), /claim must be supported/);
});

test("rejects unsupported content in a successful answer", () => {
  const document = fixture();
  matchingTurn(document).events[4].explanation = "A claim from model memory.";
  assert.throws(() => validateScenarioDocument(document, options), /fields must match the closed shape/);
});

test("rejects a fallback flag in a bounded answer", () => {
  const document = fixture();
  scenario(document, "language-no-match").turns[0].events[2].fallback = true;
  assert.throws(() => validateScenarioDocument(document, options), /fields must match the closed shape/);
});

test("rejects repository or model fallback after no match", () => {
  const document = fixture();
  const events = scenario(document, "language-no-match").turns[0].events;
  events.splice(2, 0, { type: "read", path: "docs/proposals/example.md" });
  assert.throws(() => validateScenarioDocument(document, options), /forbidden fallback event read/);
});

test("rejects incomplete failure provenance", () => {
  const document = fixture();
  delete scenario(document, "topic-unreadable").turns[1].events[4].failure.artifact_uri;
  assert.throws(() => validateScenarioDocument(document, options), /failed operation and artifact/);
});

test("rejects a stale result without the schema-required message", () => {
  const document = fixture();
  delete scenario(document, "stale-snapshot-uri").turns[1].events[3].value.structuredContent.message;
  assert.throws(() => validateScenarioDocument(document, options), /value does not match exactly one published schema branch/);
});

test("rejects mutation of the earlier result after failure", () => {
  const document = fixture();
  scenario(document, "search-unavailable").turns[1].events[2].retained_result.claims[0] = "Changed after failure.";
  assert.throws(() => validateScenarioDocument(document, options), /earlier result changed/);
});

test("rejects removal from the complete earlier result after failure", () => {
  const document = fixture();
  delete scenario(document, "search-unavailable").turns[1].events[2].retained_result.source_uris;
  assert.throws(() => validateScenarioDocument(document, options), /earlier result changed/);
});

test("rejects a repository task that skips docs README", () => {
  const document = fixture();
  scenario(document, "repository-current").turns[0].events[0].path = "docs/specification/mcp.md";
  assert.throws(() => validateScenarioDocument(document, options), /must start at docs\/README.md/);
});

test("rejects a traversing repository authority", () => {
  const document = fixture();
  scenario(document, "repository-current").turns[0].events[1].path = "docs/specification/../../crates/veln-mcp/Cargo.toml";
  assert.throws(() => validateScenarioDocument(document, options), /must be normalized/);
});

test("rejects a nonexistent repository authority", () => {
  const document = fixture();
  scenario(document, "repository-current").turns[0].events[1].path = "docs/specification/not-present.md";
  assert.throws(() => validateScenarioDocument(document, options), /does not exist/);
});

test("rejects an authority not selected by the routing page", () => {
  const document = fixture();
  const current = scenario(document, "repository-current").turns[0];
  current.events[0].value.route = "docs/specification/types.md";
  current.events[1].path = "docs/specification/types.md";
  current.events[2].repository_authority = "docs/specification/types.md";
  assert.throws(() => validateScenarioDocument(document, options), /routing page does not select/);
});

test("rejects a repeated repository path", () => {
  const document = fixture();
  const turn = scenario(document, "repository-unknown").turns[0];
  turn.events.splice(2, 0, {
    type: "read",
    path: "docs/navigation.md",
    value: { route: "docs/navigation-full.md" },
  });
  assert.throws(() => validateScenarioDocument(document, options), /repeated a path/);
});

test("rejects a repository route beyond the read bound", () => {
  const document = fixture();
  const turn = scenario(document, "repository-unknown").turns[0];
  turn.events.splice(3, 0, {
    type: "read",
    path: "docs/specification/README.md",
    value: { route: null },
  });
  assert.throws(() => validateScenarioDocument(document, options), /exceeded its read bound/);
});
