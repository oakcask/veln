import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { Worker } from "node:worker_threads";

import {
  currentRepositoryAuthority,
  expectedPublishedSearch,
  loadCaseFoldMappings,
  loadPublishedLanguageReference,
  loadSnapshotEvidence,
  readScenarioDocument,
  selectRequestRoute,
  shortestDocumentationRoute,
  validateScenarioDocument,
} from "./check-veln-language-skill.mjs";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = join(scriptDirectory, "..");
const fixturePath = join(scriptDirectory, "fixtures", "veln-language", "scenarios.json");
const stressWorkerPath = new URL("./check-veln-language-skill.stress-worker.mjs", import.meta.url);
const candidateOptions = {};
const options = candidateOptions;

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

function runStressTarget(target, data, timeout = 2_000) {
  return new Promise((resolvePromise, rejectPromise) => {
    const worker = new Worker(stressWorkerPath, { workerData: { target, data } });
    let settled = false;
    const finish = (callback, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      callback(value);
    };
    const timer = setTimeout(() => {
      void worker.terminate();
      finish(rejectPromise, new Error(`stress target ${target} exceeded ${timeout} ms`));
    }, timeout);
    worker.once("message", (message) => {
      if (message.ok) finish(resolvePromise, message.value);
      else finish(rejectPromise, Object.assign(new Error(message.message), { name: message.name }));
    });
    worker.once("error", (error) => finish(rejectPromise, error));
    worker.once("exit", (code) => {
      if (code !== 0) finish(rejectPromise, new Error(`stress target ${target} exited with code ${code}`));
    });
  });
}

function digestCatalog(catalog) {
  const bytes = Buffer.from(`${JSON.stringify(catalog)}\n`);
  const length = Buffer.alloc(8);
  length.writeBigUInt64BE(BigInt(bytes.length));
  return createHash("sha256")
    .update(Buffer.from("veln-language-reference/v1\0"))
    .update(length)
    .update(bytes)
    .digest("hex");
}

function writeSnapshotEvidence(root, snapshots) {
  const directory = join(root, "workflow-scripts", "fixtures", "veln-language");
  mkdirSync(directory, { recursive: true });
  writeFileSync(
    join(directory, "snapshot-catalogs.json"),
    JSON.stringify({ schema_version: 1, snapshots }),
  );
}

test("canonical veln-language skill replays every acceptance scenario", () => {
  assert.equal(validateScenarioDocument(fixture()), 12);
});

test("request selection derives every corpus route from raw request text", () => {
  const document = fixture();
  for (const entry of document.request_selection) {
    assert.equal(selectRequestRoute(entry.text, candidateOptions), entry.route, entry.id);
  }
});

test("raw-text selector distinguishes semantic routing contrasts", () => {
  assert.equal(
    selectRequestRoute("Please assess the Veln parser.", candidateOptions),
    "repository",
  );
  assert.equal(
    selectRequestRoute("Audit the Veln lexer.", candidateOptions),
    "repository",
  );
  assert.equal(
    selectRequestRoute("Assess how effects are handled in Veln.", candidateOptions),
    "repository",
  );
  assert.equal(
    selectRequestRoute("How are effects handled in Veln?", candidateOptions),
    "language",
  );
  assert.equal(
    selectRequestRoute("Explain how effects are handled in Veln.", candidateOptions),
    "language",
  );
});

test("rejects invalid request text and contradictory corpus routes", () => {
  assert.throws(() => selectRequestRoute({}, candidateOptions), /request text must be a string/);
  assert.throws(() => selectRequestRoute("", candidateOptions), /request text must not be empty/);
  const document = fixture();
  document.request_selection.find((entry) => entry.id === "effects-passive-information").route = "repository";
  assert.throws(() => validateScenarioDocument(document, candidateOptions), /raw request text selected the wrong route/);
});

test("requires semantic evidence for every replayed request", () => {
  const document = fixture();
  scenario(document, "language-match").turns[0].request.text =
    "What can I test in Veln schema fields?";
  assert.throws(() => validateScenarioDocument(document, candidateOptions), /no independent semantic annotation/);
});

test("checks every corpus row against independent text and semantic labels", () => {
  const document = fixture();
  for (const [index, selection] of document.request_selection.entries()) {
    for (const [field, value] of [
      ["text", `${selection.text} changed`],
      ["action", selection.action === "information" ? "repository_action" : "information"],
      ["subject", selection.subject === "implementation" ? "language_behavior" : "implementation"],
    ]) {
      const mutated = structuredClone(document);
      mutated.request_selection[index][field] = value;
      if (field !== "text") {
        const semantics = mutated.request_selection[index];
        semantics.route = semantics.action === "repository_action"
          || semantics.subject !== "language_behavior"
          ? "repository"
          : "language";
      }
      assert.throws(
        () => validateScenarioDocument(mutated, candidateOptions),
        /raw request text selected the wrong route|differ from the independent corpus oracle/,
        `${selection.id} ${field}`,
      );
    }
  }
});

test("rejects missing and unknown request-selection corpus IDs", () => {
  const missing = fixture();
  missing.request_selection = missing.request_selection.filter((entry) => entry.id !== "add-action");
  assert.throws(
    () => validateScenarioDocument(missing, candidateOptions),
    /update request-selection IDs to exactly match the canonical corpus/,
  );

  const unknown = fixture();
  unknown.request_selection.push({
    id: "fabricated-language-row",
    action: "information",
    subject: "language_behavior",
    text: "Describe an invented Veln behavior.",
    route: "language",
  });
  assert.throws(
    () => validateScenarioDocument(unknown, candidateOptions),
    /update request-selection IDs to exactly match the canonical corpus/,
  );
});

test("rejects unsupported answer provenance and fallback", () => {
  const unsupported = fixture();
  scenario(unsupported, "language-match").turns[0].events.at(-1).claims = ["Model memory says otherwise."];
  assert.throws(() => validateScenarioDocument(unsupported, candidateOptions), /selected-resource evidence/);

  const fallback = fixture();
  scenario(fallback, "language-no-match").turns[0].events.at(-1).fallback = "proposal";
  assert.throws(() => validateScenarioDocument(fallback, candidateOptions), /fields must match/);
});

test("preserves an earlier successful result after bounded failure", () => {
  const document = fixture();
  scenario(document, "search-unavailable").turns[1].events.at(-1).retained_result.claims = [];
  assert.throws(() => validateScenarioDocument(document, candidateOptions), /earlier result changed/);
});

test("derives the smallest repository documentation route", () => {
  assert.deepEqual(
    shortestDocumentationRoute("docs/README.md", "docs/specification/mcp.md", repositoryRoot, 3),
    ["docs/README.md", "docs/specification/mcp.md"],
  );
});

test("terminates a nonresponsive stress worker", async () => {
  await assert.rejects(runStressTarget("nontermination", {}, 100), /exceeded 100 ms/);
});

test("normalizes the largest accepted internal whitespace run with linear scaling", async () => {
  const result = await runStressTarget("normalization-scaling", {
    sizes: [65_536, 131_072, 262_144],
    repetitions: 12,
  });
  assert.deepEqual(result.lengths, [65_538, 131_074, 262_146]);
  assert.ok(result.milliseconds[1] <= result.milliseconds[0] * 3.5 + 2, result.milliseconds);
  assert.ok(result.milliseconds[2] <= result.milliseconds[1] * 3.5 + 2, result.milliseconds);
});

test("preserves Unicode whitespace trimming semantics", async () => {
  const result = await runStressTarget("normalization-values", {
    values: ["\u0085 schema \u3000", "a   b", "\u2003\u2003"],
  });
  assert.deepEqual(result, ["schema", "a   b", ""]);
});

test("rejects a published catalog beyond its byte limit before reading it", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-catalog-bytes-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const generated = join(root, "tools", "veln-repo-language-reference", "generated");
  mkdirSync(generated, { recursive: true });
  writeFileSync(join(generated, "language-reference-catalog-v1.sha256"), `${"0".repeat(64)}\n`);
  writeFileSync(join(generated, "language-reference-catalog-v1.json"), " ".repeat(2_000_001));
  assert.throws(() => loadPublishedLanguageReference(root), /catalog exceeds the byte limit/);
});

test("rejects a published catalog beyond its topic limit", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-catalog-topics-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const generated = join(root, "tools", "veln-repo-language-reference", "generated");
  mkdirSync(generated, { recursive: true });
  const catalog = { topics: Array.from({ length: 513 }, (_, id) => ({ id })) };
  writeFileSync(join(generated, "language-reference-catalog-v1.json"), `${JSON.stringify(catalog)}\n`);
  writeFileSync(join(generated, "language-reference-catalog-v1.sha256"), `${digestCatalog(catalog)}\n`);
  assert.throws(() => loadPublishedLanguageReference(root), /catalog exceeds the topic limit/);
});

test("rejects snapshot, override, and cross-product work beyond explicit limits", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-snapshot-limits-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const published = { digest: "0".repeat(64), catalog: { topics: [{ id: "topic" }] } };
  const snapshot = {
    base_digest: published.digest,
    digest: "1".repeat(64),
    topic_overrides: [],
  };

  writeSnapshotEvidence(root, Array.from({ length: 33 }, () => snapshot));
  assert.throws(() => loadSnapshotEvidence(root, published), /snapshot limit/);

  writeSnapshotEvidence(root, [{ ...snapshot, topic_overrides: Array.from({ length: 65 }, () => ({})) }]);
  assert.throws(() => loadSnapshotEvidence(root, published), /overrides exceed the limit/);

  writeSnapshotEvidence(root, Array.from({ length: 32 }, () => snapshot));
  const widePublished = {
    ...published,
    catalog: { topics: Array.from({ length: 513 }, (_, id) => ({ id: `topic-${id}` })) },
  };
  assert.throws(() => loadSnapshotEvidence(root, widePublished), /snapshot-topic work limit/);
});

test("bounds snapshot reconstruction at the largest accepted dimensions", async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-snapshot-scale-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const catalog = {
    topics: Array.from({ length: 512 }, (_, index) => ({
      id: `topic-${index}`,
      title: `Topic ${index}`,
      summary: `Summary ${index}`,
      keywords: ["topic"],
      body: [`Body ${index}`],
    })),
  };
  const published = { digest: digestCatalog(catalog), catalog };
  const snapshots = Array.from({ length: 32 }, (_, index) => {
    const topic_overrides = [{
      topic_id: "topic-0",
      id: `archived-${index}`,
      title: `Archived ${index}`,
      summary: `Summary ${index}`,
      keywords: ["archived"],
      body: [`Body ${index}`],
    }];
    const archived = structuredClone(catalog);
    archived.topics[0] = { ...archived.topics[0], ...topic_overrides[0] };
    delete archived.topics[0].topic_id;
    archived.topics.sort((left, right) => Buffer.compare(Buffer.from(left.id), Buffer.from(right.id)));
    return { base_digest: published.digest, digest: digestCatalog(archived), topic_overrides };
  });
  writeSnapshotEvidence(root, snapshots);
  const result = await runStressTarget("snapshot-evidence", { root, published });
  assert.equal(result.size, 32);
  assert.ok(result.hasCatalog.every((value) => value === false));
});

test("bounds scenario fixture bytes before expansion", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-fixture-limit-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const path = join(root, "oversized.json");
  writeFileSync(path, `{${" ".repeat(999_999)}}`);
  assert.throws(() => readScenarioDocument(path), /scenario fixture exceeds the byte limit/);
});

test("deduplicates wide repository-route aliases", async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-route-aliases-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs", "shared"), { recursive: true });
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  const entryLinks = [];
  const cycleLinks = ["[authority](../authority.md)"];
  for (let index = 0; index < 1_000; index += 1) {
    entryLinks.push(`[alias ${index}](alias-${index}/route.md)`);
    cycleLinks.push(`[cycle ${index}](cycle-${index}.md)`);
    symlinkSync("route.md", join(root, "docs", "shared", `cycle-${index}.md`));
    symlinkSync("shared", join(root, "docs", `alias-${index}`));
  }
  writeFileSync(join(root, "docs", "README.md"), entryLinks.join("\n"));
  writeFileSync(join(root, "docs", "shared", "route.md"), cycleLinks.join("\n"));
  assert.deepEqual(
    await runStressTarget("shortest-route", {
      entry: "docs/README.md",
      authority: "docs/authority.md",
      root,
      maximumReads: 3,
    }),
    ["docs/README.md", "docs/alias-0/route.md", "docs/authority.md"],
  );
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

function syntheticPublished(topic) {
  return {
    digest: "1".repeat(64),
    caseFoldMappings: loadCaseFoldMappings(),
    catalog: { topics: [topic] },
  };
}

function syntheticTopic(overrides = {}) {
  return {
    id: "unicode-search",
    title: "Unicode Search",
    summary: "No matching summary text.",
    keywords: [],
    body: ["No matching body text."],
    ...overrides,
  };
}

test("replays NFC and full default case folding without compatibility normalization", () => {
  const folded = expectedPublishedSearch(
    { query: "STRASSE", scope: "language" },
    syntheticPublished(syntheticTopic({ title: "Straße" })),
  );
  assert.equal(folded.results.length, 1);
  assert.equal(folded.results[0].excerpt, "Straße");

  const canonical = expectedPublishedSearch(
    { query: "Café", scope: "language" },
    syntheticPublished(syntheticTopic({ title: "Cafe\u0301" })),
  );
  assert.equal(canonical.results.length, 1);

  const compatibility = expectedPublishedSearch(
    { query: "1 schema", scope: "language" },
    syntheticPublished(syntheticTopic({ title: "① schema" })),
  );
  assert.deepEqual(compatibility.results, []);
});

test("replays a long excerpt from the first matching source scalar", () => {
  const body = `${"😀".repeat(180)}needle${"z".repeat(180)}`;
  const result = expectedPublishedSearch(
    { query: "needle", scope: "language" },
    syntheticPublished(syntheticTopic({ body: [body] })),
  ).results[0];
  assert.equal([...result.excerpt].length, 160);
  assert.equal(result.excerpt.startsWith("needle"), true);
  assert.equal(result.prefix_truncated, true);
  assert.equal(result.suffix_truncated, true);
});

test("rejects a search query unrelated to the scenario expectation", () => {
  const document = fixture();
  matchingTurn(document).events[0].arguments.query = "Veln effects";
  assert.throws(() => validateScenarioDocument(document, options), /search request must match request-selected evidence/);
});

test("accepts a deterministic selection for a multi-topic language question", () => {
  const document = fixture();
  matchingTurn(document).request.text = "How do Veln schemas and contracts interact?";
  assert.match(matchingTurn(document).request.text, /\bschemas\b.*\bcontracts\b/u);
  assert.equal(validateScenarioDocument(document, options), 12);
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

test("rejects a fallback field on a language search call", () => {
  const document = fixture();
  matchingTurn(document).events[0].fallback = "model-memory";
  assert.throws(() => validateScenarioDocument(document, options), /search call event: fields must match the closed shape/);
});

test("rejects an unknown field on a language read call", () => {
  const document = fixture();
  matchingTurn(document).events[2].retry = false;
  assert.throws(() => validateScenarioDocument(document, options), /read call event: fields must match the closed shape/);
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

test("rejects resource_not_found as a generic unreadable-topic transport failure", () => {
  const document = fixture();
  scenario(document, "topic-unreadable").turns[1].events[3].error.code = "resource_not_found";
  assert.throws(
    () => validateScenarioDocument(document, options),
    /resource_not_found must use the checked stale-snapshot result path/,
  );
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
    uri: "veln-doc:///language/snapshot/0ad0e0df939b4fbb64748e6f182838c804799919de624ec063b038df1b31c350/topic/fabricated-modules",
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

test("rejects closed and superseded terminal repository authorities", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-authority-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const authority = join(root, "authority.md");
  for (const status of ["closed", "superseded"]) {
    writeFileSync(authority, `---\nrole: reference\nauthority: supporting\nstatus: ${status}\n---\n`);
    assert.throws(
      () => currentRepositoryAuthority(authority),
      new RegExp(`terminal repository authority must be current, not lifecycle status ${status}`),
    );
  }
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

test("rejects repository routes that appear only in non-navigational Markdown", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-route-examples-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  writeFileSync(join(root, "docs", "README.md"), [
    "```markdown",
    "[fenced example](authority.md)",
    "```",
    "",
    "~~~markdown",
    "[tilde-fenced example](authority.md)",
    "~~~",
    "",
    "`[inline example](authority.md)`",
    "",
    "    [indented-code example](authority.md)",
    "",
    "\\[escaped-link example](authority.md)",
    "",
    "<!-- [comment example](authority.md) -->",
    "",
    "![image](authority.md)",
  ].join("\n"));

  assert.throws(
    () => shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 2),
    /repository authority is not reachable/,
  );
});

test("terminates a nonresponsive stress worker at the external time bound", async () => {
  await assert.rejects(
    runStressTarget("nontermination", {}, 100),
    /exceeded 100 ms/,
  );
});

test("deduplicates wide alias cycles by resolved documentation identity", async (context) => {
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
    await runStressTarget("shortest-route", {
      entry: "docs/README.md", authority: "docs/authority.md", root, maximumReads: 3,
    }),
    ["docs/README.md", "docs/alias-0/route.md", "docs/authority.md"],
  );
});

test("bounds discovery across wide distinct documentation graphs", async (context) => {
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

  await assert.rejects(
    runStressTarget("shortest-route", {
      entry: "docs/README.md", authority: "docs/authority.md", root, maximumReads: 3, discoveryBound: 16,
    }),
    /exceeded the 16-document bound/,
  );
});

test("returns an authority found before unrelated wide siblings exceed the discovery bound", async (context) => {
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
    await runStressTarget("shortest-route", {
      entry: "docs/README.md", authority: "docs/authority.md", root, maximumReads: 3, discoveryBound: 2,
    }),
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

test("discovers links in linear progress on malformed adjacent-size input", async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-links-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "README.md"), "[".repeat(262_144));
  assert.deepEqual(await runStressTarget("linked-paths", { path: "docs/README.md", root }), []);
});

test("retains snapshot overrides without bilinear catalog copies", async (context) => {
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
  const digestBytes = (bytes) => {
    const length = Buffer.alloc(8);
    length.writeBigUInt64BE(BigInt(bytes.length));
    return createHash("sha256")
      .update(Buffer.from("veln-language-reference/v1\0"))
      .update(length)
      .update(bytes)
      .digest("hex");
  };
  const digest = (value) => digestBytes(Buffer.from(`${JSON.stringify(value)}\n`));
  assert.notEqual(
    digest(catalog),
    digestBytes(Buffer.from(JSON.stringify(catalog))),
    "canonical catalog digest must include the terminal LF",
  );
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

  const loaded = await runStressTarget("snapshot-evidence", { root, published });
  assert.equal(loaded.cloneCalls, 0);
  assert.equal(loaded.size, snapshots.length);
  assert.ok(loaded.keys.every((keys) => (
    JSON.stringify(keys) === JSON.stringify(["base_digest", "digest", "topic_overrides"])
  )));
  assert.ok(loaded.hasCatalog.every((value) => value === false));
  assert.ok(loaded.serializedLength < JSON.stringify(catalog).length * 2);
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
    request_selection: [],
    scenarios: Array.from({ length: 12 }, (_, index) => ({
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
  assert.equal(values.length, 120);
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
      request_selection: [],
      scenarios: Array.from({ length: 12 }, scenario),
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
