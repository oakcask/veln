import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  linkedDocumentationPaths,
  loadSnapshotEvidence,
  readScenarioDocument,
  shortestDocumentationRoute,
  validateScenarioDocument,
} from "./check-veln-language-skill.mjs";

const fixturePath = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "veln-language", "scenarios.json");
const skillText = `---
name: veln-language
description: Use for Veln language questions and for inspecting or changing the Veln repository through its documentation authority.
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
    "selection": "first_search_result",
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
    "authority_selection": "smallest_current_linked_authority",
    "no_route": "Stop and report that no repository documentation route covers the request.",
    "explicit_targets": ["repository", "codebase", "source code", "proposal state"],
    "intent_verbs": ["add", "change", "debug", "fix", "implement", "inspect", "modify", "refactor", "remove", "review", "select", "test", "update"],
    "intent_selection": "An inspection or change intent selects repository work regardless of its position unless the request asks how, what, when, where, whether, or why the Veln language behaves, or explicitly asks about language semantics.",
    "language_complements": ["how", "what", "when", "where", "whether", "why"],
    "location_question_endings": ["defined", "handled", "implemented", "located"],
    "location_question_forms": ["where", "tell me where", "show me where"],
    "path_prefixes": [".agents/", ".github/", "crates/", "docs/", "editors/", "examples/", "scripts/", "tools/", "workflow-scripts/"]
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

function structured(event) {
  return event.value.structuredContent;
}

function syncEnvelope(event) {
  event.value.content[0].text = JSON.stringify(event.value.structuredContent);
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

test("rejects missing skill discovery metadata", () => {
  assert.throws(
    () => validateScenarioDocument(fixture(), { skillText: skillText.replace(/^description:.*\n/m, "") }),
    /one description/,
  );
});

test("rejects empty skill discovery metadata", () => {
  assert.throws(
    () => validateScenarioDocument(fixture(), { skillText: skillText.replace(/^description:.*$/m, "description:") }),
    /must not be empty/,
  );
});

test("rejects unrelated skill discovery metadata", () => {
  assert.throws(
    () => validateScenarioDocument(fixture(), { skillText: skillText.replace(/^description:.*$/m, "description: Format JSON files.") }),
    /select Veln language questions/,
  );
});

test("rejects oversized skill discovery metadata", () => {
  assert.throws(
    () => validateScenarioDocument(fixture(), { skillText: skillText.replace(/^description:.*$/m, `description: Use for Veln language questions and repository inspection. ${"x".repeat(300)}`) }),
    /discovery limit/,
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

test("keeps a language question containing change on the language route", () => {
  const document = fixture();
  matchingTurn(document).request.text = "How do Veln schemas change?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("keeps an auxiliary-led language behavior question on the language route", () => {
  const document = fixture();
  matchingTurn(document).request.text = "Does Veln update schemas automatically?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("keeps a language question containing inspect on the language route", () => {
  const document = fixture();
  matchingTurn(document).request.text = "Inspect how Veln schemas work.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes an interrogative repository inspection without a language subject", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "Review why the compiler crashes.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes an interrogative repository inspection with a Veln implementation subject", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "Review why the Veln compiler crashes.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("keeps a language topic review on the language route", () => {
  const document = fixture();
  const turn = scenario(document, "search-unavailable").turns[1];
  turn.request.text = "Review Veln effects semantics.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("keeps a language topic inspection on the language route", () => {
  const document = fixture();
  matchingTurn(document).request.text = "Inspect Veln schema semantics.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("keeps a language question about implementation on the language route", () => {
  const document = fixture();
  matchingTurn(document).request.text = "How does Veln implement schemas?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes an ordinary compiler change request without an incidental routing keyword", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "Modify the Veln compiler parser.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes a change request for an unlisted repository component", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "Fix the Veln lexer bug.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("canonical repository scenarios cover inspection and change requests", () => {
  const document = fixture();
  assert.match(
    scenario(document, "repository-current").turns[0].request.text,
    /^After reviewing the context, inspect\b/u,
  );
  assert.match(scenario(document, "repository-change").turns[0].request.text, /^Fix\b/u);
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes a polite repository change request", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "Could you please fix Veln parser recovery?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes a repository inspection intent that does not lead the request", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "I would like you to inspect the Veln parser implementation.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes a repository intent after an unrestricted context clause", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "After reviewing the context, inspect the Veln compiler parser.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes an interrogative repository location request", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "Where is Veln parser recovery implemented?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes an ordinary parser implementation question", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "How is the Veln parser implemented?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes a passive implementation question whose implementation term is not final", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "How is schema parsing implemented in the compiler?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("keeps a non-leading inspection request about language behavior on the language route", () => {
  const document = fixture();
  matchingTurn(document).request.text = "I would like you to inspect how Veln schemas work.";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("keeps a framed language-behavior question on the language route", () => {
  const document = fixture();
  matchingTurn(document).request.text = "Review this question: how do Veln schemas work?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("does not treat a mentioned intent verb as a repository request", () => {
  const document = fixture();
  matchingTurn(document).request.text = "What does inspect mean for Veln schemas?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes an indirect polite repository location request", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "Can you tell me where Veln parser recovery is implemented?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("keeps an indirect polite language question on the language route", () => {
  const document = fixture();
  matchingTurn(document).request.text = "Can you tell me how Veln schemas work?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("keeps a language-surface location question on the language route", () => {
  const document = fixture();
  matchingTurn(document).request.text = "Where are Veln schema fields located?";
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("routes a repository path request without a generic repository keyword", () => {
  const document = fixture();
  scenario(document, "repository-change").turns[0].request.text =
    "Change crates/veln-mcp/src/server.rs.";
  assert.equal(validateScenarioDocument(document, options), 9);
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

test("rejects a search query unrelated to the scenario expectation", () => {
  const document = fixture();
  matchingTurn(document).events[0].arguments.query = "Veln effects";
  assert.throws(() => validateScenarioDocument(document, options), /search request must match request-selected evidence/);
});

test("accepts a deterministic selection for a multi-topic language question", () => {
  const document = fixture();
  assert.match(matchingTurn(document).request.text, /\bschemas\b.*\bcontracts\b/u);
  assert.equal(validateScenarioDocument(document, options), 9);
});

test("rejects a non-first topic from a multi-result language search", () => {
  const document = fixture();
  matchingTurn(document).events[2].arguments.uri = structured(matchingTurn(document).events[1]).results[1].uri;
  assert.throws(() => validateScenarioDocument(document, options), /deterministically select the first search result/);
});

test("gives an explicit repository path precedence over a competing MCP subject", () => {
  const document = fixture();
  scenario(document, "repository-current").turns[0].request.text =
    "Inspect MCP documentation search in docs/specification/types.md.";
  assert.throws(() => validateScenarioDocument(document, options), /acceptance label does not match request-selected repository/);
});

test("rejects a read before language search", () => {
  const document = fixture();
  matchingTurn(document).events[0].tool = "read_doc";
  assert.throws(() => validateScenarioDocument(document, options), /language route must search first/);
});

test("rejects a noncanonical snapshot URI", () => {
  const document = fixture();
  const event = matchingTurn(document).events[1];
  structured(event).results[0].uri = "veln-doc:///language/snapshot/placeholder/topic/schemas";
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /canonical snapshot digest/);
});

test("rejects an incomplete search result", () => {
  const document = fixture();
  const event = matchingTurn(document).events[1];
  delete structured(event).results[0].excerpt;
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /missing schema field excerpt/);
});

test("rejects search metadata that drifts from the checked language-reference artifact", () => {
  const document = fixture();
  const event = matchingTurn(document).events[1];
  structured(event).results[0].summary = "A shortened recording.";
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /differs from the checked snapshot artifact/);
});

test("rejects incomplete read metadata", () => {
  const document = fixture();
  const event = matchingTurn(document).events[3];
  delete structured(event).mimeType;
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /does not match exactly one published schema branch/);
});

test("rejects a search result outside the published schema", () => {
  const document = fixture();
  const event = matchingTurn(document).events[1];
  structured(event).results[0].fallback = true;
  syncEnvelope(event);
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
  assert.throws(() => validateScenarioDocument(document, options), /unambiguous selected-resource evidence/);
});

test("rejects a claim that appears only inside a negated statement", () => {
  const document = fixture();
  const event = matchingTurn(document).events[3];
  structured(event).text =
    "The reference does not establish this claim: Schemas describe format-neutral and binary fields.";
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /differs from the checked language-reference artifact/);
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

test("rejects a model-memory claim appended to the no-match message", () => {
  const document = fixture();
  scenario(document, "language-no-match").turns[0].events[2].message +=
    " Veln uses a borrow checker inferred from model memory.";
  assert.throws(() => validateScenarioDocument(document, options), /must only report the published-topic absence/);
});

test("rejects a successful result without the published MCP envelope", () => {
  const document = fixture();
  delete matchingTurn(document).events[1].value.content;
  assert.throws(() => validateScenarioDocument(document, options), /MCP tool envelope/);
});

test("rejects MCP text content that differs from structuredContent", () => {
  const document = fixture();
  matchingTurn(document).events[1].value.content[0].text = '{"scope":"language","results":[]}';
  assert.throws(() => validateScenarioDocument(document, options), /must encode structuredContent/);
});

test("rejects a successful envelope marked as an error", () => {
  const document = fixture();
  matchingTurn(document).events[1].value.isError = true;
  assert.throws(() => validateScenarioDocument(document, options), /wrong error state/);
});

test("rejects a search result with both error and value", () => {
  const document = fixture();
  scenario(document, "search-unavailable").turns[1].events[1].value =
    structured(matchingTurn(document).events[1]);
  assert.throws(() => validateScenarioDocument(document, options), /exactly one of error or value/);
});

test("rejects a read result with both error and value", () => {
  const document = fixture();
  scenario(document, "topic-unreadable").turns[1].events[3].value =
    matchingTurn(document).events[3].value;
  assert.throws(() => validateScenarioDocument(document, options), /exactly one of error or value/);
});

test("rejects a selected resource beyond the published byte limit", () => {
  const document = fixture();
  const event = matchingTurn(document).events[3];
  structured(event).text = "Sentence. ".repeat(30_000);
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /published byte limit/);
});

test("rejects read text that drifts from the checked language-reference artifact", () => {
  const document = fixture();
  const turn = matchingTurn(document);
  const statement = "A replacement recording.";
  structured(turn.events[3]).text = statement;
  syncEnvelope(turn.events[3]);
  turn.expected.answer_claims = [statement];
  turn.events[4].claims = [statement];
  assert.throws(() => validateScenarioDocument(document, options), /differs from the checked language-reference artifact/);
});

test("rejects incomplete failure provenance", () => {
  const document = fixture();
  delete scenario(document, "topic-unreadable").turns[1].events[4].failure.artifact_uri;
  assert.throws(() => validateScenarioDocument(document, options), /failed operation and artifact/);
});

test("rejects a stale result without the schema-required message", () => {
  const document = fixture();
  const event = scenario(document, "stale-snapshot-uri").turns[1].events[3];
  delete event.value.structuredContent.message;
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /value does not match exactly one published schema branch/);
});

test("rejects a stale-snapshot row that uses the current published digest", () => {
  const document = fixture();
  const turn = scenario(document, "stale-snapshot-uri").turns[1];
  const staleUri = turn.events[2].arguments.uri;
  const currentUri = staleUri.replace(
    /snapshot\/[0-9a-f]{64}\//u,
    "snapshot/4fc5858d00e37d7e88faedcef4bb2c02175fa0dcac4c54a7caa395309d776ed9/",
  );
  structured(turn.events[1]).results[0].uri = currentUri;
  structured(turn.events[1]).results.splice(1);
  syncEnvelope(turn.events[1]);
  turn.events[2].arguments.uri = currentUri;
  structured(turn.events[3]).details.uri = currentUri;
  syncEnvelope(turn.events[3]);
  turn.events[4].failure.artifact_uri = currentUri;
  assert.throws(
    () => validateScenarioDocument(document, options),
    /differs from the checked snapshot artifact|stale snapshot URI must differ/,
  );
});

test("rejects mutation of an unselected stale search result", () => {
  const document = fixture();
  const event = scenario(document, "stale-snapshot-uri").turns[1].events[1];
  structured(event).results[1].summary = "Fabricated historical summary.";
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /differs from the checked snapshot artifact/);
});

test("rejects a fabricated unselected stale search result", () => {
  const document = fixture();
  const event = scenario(document, "stale-snapshot-uri").turns[1].events[1];
  structured(event).results.push({
    uri: "veln-doc:///language/snapshot/88ff7d6072458b22355d854d7d8ad464223c752c1e5f746a87ed725f2ab83359/topic/fabricated-modules",
    title: "Fabricated Modules",
    summary: "This topic has no snapshot evidence.",
    excerpt: "modules",
    prefix_truncated: false,
    suffix_truncated: false,
  });
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /differs from the checked snapshot artifact/);
});

test("rejects stale search results from mixed snapshots", () => {
  const document = fixture();
  const event = scenario(document, "stale-snapshot-uri").turns[1].events[1];
  structured(event).results[1].uri = structured(event).results[1].uri.replace(
    /snapshot\/[0-9a-f]{64}\//u,
    "snapshot/4fc5858d00e37d7e88faedcef4bb2c02175fa0dcac4c54a7caa395309d776ed9/",
  );
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /must belong to one snapshot/);
});

test("rejects stale search results without snapshot evidence", () => {
  const document = fixture();
  const event = scenario(document, "stale-snapshot-uri").turns[1].events[1];
  for (const result of structured(event).results) {
    result.uri = result.uri.replace(
      /snapshot\/[0-9a-f]{64}\//u,
      "snapshot/1111111111111111111111111111111111111111111111111111111111111111/",
    );
  }
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /no checked snapshot evidence/);
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

test("rejects fallback fields in repository read results", () => {
  const document = fixture();
  scenario(document, "repository-current").turns[0].events[0].value.fallback = "published-reference";
  assert.throws(() => validateScenarioDocument(document, options), /fields must match the closed shape/);
});

test("rejects published-reference authority in a terminal repository read", () => {
  const document = fixture();
  scenario(document, "repository-current").turns[0].events[1].value.authority = "published-language-reference";
  assert.throws(() => validateScenarioDocument(document, options), /terminal read named the wrong repository authority/);
});

test("rejects extra fields in a terminal repository read", () => {
  const document = fixture();
  scenario(document, "repository-current").turns[0].events[1].value.fallback = true;
  assert.throws(() => validateScenarioDocument(document, options), /fields must match the closed shape/);
});

test("rejects a longer repository route when a direct route exists", () => {
  const document = fixture();
  const current = scenario(document, "repository-current").turns[0];
  current.events[0].value.route = "docs/specification/README.md";
  current.events.splice(1, 0, {
    type: "read",
    path: "docs/specification/README.md",
    value: { route: "docs/specification/mcp.md" },
  });
  assert.throws(() => validateScenarioDocument(document, options), /smallest task-appropriate documentation path/);
});

test("derives the shortest repository route from current documentation links", () => {
  assert.deepEqual(
    shortestDocumentationRoute("docs/README.md", "docs/specification/mcp.md", join(dirname(fixturePath), "../../.."), 3),
    ["docs/README.md", "docs/specification/mcp.md"],
  );
});

test("deduplicates wide alias cycles by resolved documentation identity", { timeout: 1_000 }, (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-route-aliases-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs", "shared"), { recursive: true });
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");

  const width = 1_000;
  const entryLinks = [];
  const cycleLinks = ["[authority](../authority.md)"];
  for (let index = 0; index < width; index += 1) {
    entryLinks.push(`[alias ${index}](alias-${index}/route.md)`);
    cycleLinks.push(`[cycle ${index}](cycle-${index}.md)`);
    symlinkSync("route.md", join(root, "docs", "shared", `cycle-${index}.md`));
    symlinkSync("shared", join(root, "docs", `alias-${index}`));
  }
  writeFileSync(join(root, "docs", "README.md"), entryLinks.join("\n"));
  writeFileSync(join(root, "docs", "shared", "route.md"), cycleLinks.join("\n"));

  assert.deepEqual(
    shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 3),
    ["docs/README.md", "docs/alias-0/route.md", "docs/authority.md"],
  );
});

test("bounds discovery across wide distinct documentation graphs", { timeout: 1_000 }, (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-route-width-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");

  const links = [];
  for (let index = 0; index < 100; index += 1) {
    const name = `branch-${index}.md`;
    links.push(`[branch ${index}](${name})`);
    writeFileSync(join(root, "docs", name), "# Branch\n");
  }
  writeFileSync(join(root, "docs", "README.md"), links.join("\n"));

  assert.throws(
    () => shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 3, 16),
    /exceeded the 16-document bound/,
  );
});

test("returns an authority found before unrelated wide siblings exceed the discovery bound", { timeout: 1_000 }, (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-route-authority-first-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");

  const links = ["[authority](authority.md)"];
  for (let index = 0; index < 100; index += 1) {
    const name = `branch-${index}.md`;
    links.push(`[branch ${index}](${name})`);
    writeFileSync(join(root, "docs", name), "# Branch\n");
  }
  writeFileSync(join(root, "docs", "README.md"), links.join("\n"));

  assert.deepEqual(
    shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 3, 2),
    ["docs/README.md", "docs/authority.md"],
  );
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

test("discovers links in linear progress on malformed adjacent-size input", { timeout: 1_000 }, (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-links-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "README.md"), "[".repeat(262_144));
  assert.deepEqual(linkedDocumentationPaths("docs/README.md", root), []);
});

test("retains snapshot overrides without bilinear catalog copies", { timeout: 1_000 }, (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-snapshots-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const evidenceDirectory = join(root, "workflow-scripts", "fixtures", "veln-language");
  mkdirSync(evidenceDirectory, { recursive: true });
  const catalog = {
    topics: Array.from({ length: 128 }, (_, index) => ({
      id: `topic-${index}`,
      title: `Topic ${index}`,
      summary: `Summary ${index}`,
      keywords: ["topic"],
      body: [`Body ${index}`],
    })),
  };
  const digest = (value) => {
    const bytes = Buffer.from(JSON.stringify(value));
    const length = Buffer.alloc(8);
    length.writeBigUInt64BE(BigInt(bytes.length));
    return createHash("sha256")
      .update(Buffer.from("veln-language-reference/v1\0"))
      .update(length)
      .update(bytes)
      .digest("hex");
  };
  const published = { digest: digest(catalog), catalog };
  const snapshots = Array.from({ length: 32 }, (_, index) => {
    const topic_overrides = [{
      topic_id: "topic-0",
      id: `archived-topic-${index}`,
      title: `Archived Topic ${index}`,
      summary: `Archived summary ${index}`,
      keywords: ["archived"],
      body: [`Archived body ${index}`],
    }];
    const archived = structuredClone(catalog);
    archived.topics[0] = { ...archived.topics[0], ...topic_overrides[0] };
    delete archived.topics[0].topic_id;
    archived.topics.sort((left, right) => Buffer.compare(Buffer.from(left.id), Buffer.from(right.id)));
    return {
      base_digest: published.digest,
      digest: digest(archived),
      topic_overrides,
    };
  });
  writeFileSync(
    join(evidenceDirectory, "snapshot-catalogs.json"),
    JSON.stringify({ schema_version: 1, snapshots }),
  );

  const originalStructuredClone = globalThis.structuredClone;
  let cloneCalls = 0;
  globalThis.structuredClone = (...arguments_) => {
    cloneCalls += 1;
    return originalStructuredClone(...arguments_);
  };
  let loaded;
  try {
    loaded = loadSnapshotEvidence(root, published);
  } finally {
    globalThis.structuredClone = originalStructuredClone;
  }
  assert.equal(cloneCalls, 0);
  assert.equal(loaded.size, snapshots.length);
  for (const snapshot of loaded.values()) {
    assert.deepEqual(Object.keys(snapshot).sort(), ["base_digest", "digest", "topic_overrides"]);
    assert.equal(Object.hasOwn(snapshot, "catalog"), false);
  }
  assert.ok(JSON.stringify([...loaded.values()]).length < JSON.stringify(catalog).length * 2);
});

test("accepts a scenario file exactly at the byte limit", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-fixture-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const path = join(root, "limit.json");
  writeFileSync(path, `{${" ".repeat(999_998)}}`);
  assert.deepEqual(readScenarioDocument(path), {});
});

test("rejects a scenario file before reading beyond the byte limit", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-fixture-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const path = join(root, "oversized.json");
  writeFileSync(path, `{${" ".repeat(999_999)}}`);
  assert.throws(() => readScenarioDocument(path), /scenario fixture exceeds the byte limit/);
});

test("shares recordings at the largest accepted reference boundary", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-fixture-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const referenceEvent = () => ({
    type: "result",
    tool: "search_docs",
    value_ref: "schemas-search",
  });
  const document = {
    schema_version: 1,
    recordings: {
      "schemas-search": { content: "" },
      "schemas-read": {},
    },
    scenarios: Array.from({ length: 9 }, (_, index) => ({
      id: `reference-boundary-${index}`,
      covers: "language-match",
      turns: Array.from({ length: 2 }, () => ({
        request: { text: "How do Veln schemas work?" },
        events: Array.from({ length: 5 }, referenceEvent),
      })),
    })),
  };
  const emptyBytes = Buffer.byteLength(JSON.stringify(document));
  document.recordings["schemas-search"].content = "x".repeat(1_000_000 - emptyBytes);
  const serialized = JSON.stringify(document);
  assert.equal(Buffer.byteLength(serialized), 1_000_000);
  const path = join(root, "reference-boundary.json");
  writeFileSync(path, serialized);

  const originalStructuredClone = globalThis.structuredClone;
  let cloneCalls = 0;
  globalThis.structuredClone = (...arguments_) => {
    cloneCalls += 1;
    return originalStructuredClone(...arguments_);
  };
  let loaded;
  try {
    loaded = readScenarioDocument(path);
  } finally {
    globalThis.structuredClone = originalStructuredClone;
  }

  const values = loaded.scenarios.flatMap((scenario) => scenario.turns)
    .flatMap((turn) => turn.events)
    .map((event) => event.value);
  assert.equal(values.length, 90);
  assert.equal(cloneCalls, 0);
  assert.ok(values.every((value) => value === values[0]));
});

test("checks every recording reference bound before expanding recordings", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-fixture-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const referenceEvent = () => ({
    type: "result",
    tool: "search_docs",
    value_ref: "schemas-search",
  });
  const turn = () => ({
    request: { text: "How do Veln schemas work?" },
    events: [referenceEvent()],
  });
  const scenario = () => ({
    id: "reference-amplification",
    covers: "language-match",
    turns: [turn()],
  });
  const cases = [
    {
      name: "scenarios",
      message: /scenario document exceeds the scenario limit/,
      mutate: (document) => document.scenarios.push(scenario()),
    },
    {
      name: "turns",
      message: /scenario exceeds the turn limit/,
      mutate: (document) => document.scenarios[0].turns.push(turn()),
    },
    {
      name: "events",
      message: /turn exceeds the event limit/,
      mutate: (document) => document.scenarios[0].turns[0].events.push(referenceEvent()),
    },
  ];
  for (const fixtureCase of cases) {
    const document = {
      schema_version: 1,
      recordings: {
        "schemas-search": { content: "x".repeat(100_000) },
        "schemas-read": {},
      },
      scenarios: Array.from({ length: 9 }, scenario),
    };
    if (fixtureCase.name === "turns") document.scenarios[0].turns.push(turn());
    if (fixtureCase.name === "events") {
      document.scenarios[0].turns[0].events = Array.from({ length: 5 }, referenceEvent);
    }
    fixtureCase.mutate(document);
    const path = join(root, `${fixtureCase.name}.json`);
    writeFileSync(path, JSON.stringify(document));
    const originalStructuredClone = globalThis.structuredClone;
    let cloneCalls = 0;
    globalThis.structuredClone = (...arguments_) => {
      cloneCalls += 1;
      return originalStructuredClone(...arguments_);
    };
    try {
      assert.throws(() => readScenarioDocument(path), fixtureCase.message);
      assert.equal(cloneCalls, 0, `${fixtureCase.name} limit must precede recording expansion`);
    } finally {
      globalThis.structuredClone = originalStructuredClone;
    }
  }
});
