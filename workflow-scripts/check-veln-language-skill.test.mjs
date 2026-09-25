import assert from "node:assert/strict";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  readScenarioDocument,
  validateScenarioDocument,
} from "./check-veln-language-skill.mjs";

const fixturePath = join(
  dirname(fileURLToPath(import.meta.url)),
  "fixtures",
  "veln-language",
  "scenarios.json",
);

function fixture() {
  return readScenarioDocument(fixturePath);
}

function scenario(document, coverage) {
  return document.scenarios.find((candidate) => candidate.covers === coverage);
}

test("replays every veln-language acceptance scenario", () => {
  assert.equal(validateScenarioDocument(fixture()), 8);
});

test("rejects a language search without explicit language scope", () => {
  const document = fixture();
  delete scenario(document, "language-match").turns[0].events[0].arguments.scope;
  assert.throws(() => validateScenarioDocument(document), /search scope must be language/);
});

test("rejects a read before language search", () => {
  const document = fixture();
  scenario(document, "language-match").turns[0].events[0].tool = "read_doc";
  assert.throws(() => validateScenarioDocument(document), /language route must search first/);
});

test("rejects a topic URI not returned by search", () => {
  const document = fixture();
  scenario(document, "language-match").turns[0].events[2].arguments.uri =
    "veln-doc:///language/snapshot/synthesized/topic/schemas";
  assert.throws(() => validateScenarioDocument(document), /must exactly match a search result/);
});

test("rejects a claim absent from the selected topic", () => {
  const document = fixture();
  scenario(document, "language-match").turns[0].events[4].claims[0] =
    "Schemas implicitly generate network clients.";
  assert.throws(() => validateScenarioDocument(document), /claim must be supported/);
});

test("rejects repository or model fallback after no match", () => {
  const document = fixture();
  const events = scenario(document, "language-no-match").turns[0].events;
  events.splice(2, 0, { type: "read", path: "docs/proposals/example.md" });
  assert.throws(() => validateScenarioDocument(document), /forbidden fallback event read/);
});

test("rejects mutation of the earlier result after failure", () => {
  const document = fixture();
  scenario(document, "search-unavailable").turns[1].events[2].retained_result.claims[0] =
    "Changed after failure.";
  assert.throws(() => validateScenarioDocument(document), /earlier result changed/);
});

test("rejects a repository task that skips docs README", () => {
  const document = fixture();
  scenario(document, "repository-current").turns[0].events[0].path =
    "docs/specification/mcp.md";
  assert.throws(() => validateScenarioDocument(document), /must start at docs\/README.md/);
});
