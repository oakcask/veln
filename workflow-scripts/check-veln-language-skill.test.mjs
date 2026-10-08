import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdtempSync,
  mkdirSync,
  openSync,
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
  classifyReadFailure,
  currentRepositoryAuthority,
  expectedPublishedSearch,
  expectedSnapshotRead,
  linkedDocumentationPaths,
  loadCaseFoldMappings,
  loadPublishedLanguageReference,
  loadSnapshotEvidence,
  loadToolSchemas,
  readScenarioDocument,
  selectRequestRoute,
  shortestDocumentationRoute,
  validateSchema,
  validateScenarioDocument,
} from "./check-veln-language-skill.mjs";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = join(scriptDirectory, "..");
const fixturePath = join(scriptDirectory, "fixtures", "veln-language", "scenarios.json");
const stressWorkerPath = new URL("./check-veln-language-skill.stress-worker.mjs", import.meta.url);
const candidateSkillPath = process.env.VELN_LANGUAGE_SKILL_PATH;
const candidateOptions = candidateSkillPath === undefined ? {} : { skillPath: candidateSkillPath };
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

function staleTurn(document) {
  return scenario(document, "stale-snapshot-uri").turns[1];
}

function staleEvents(document) {
  const events = staleTurn(document).events;
  return {
    listResult: events.find((event) => event.type === "result" && event.tool === "list_language_topics"),
    transition: events.find((event) => event.type === "server_transition"),
    readCall: events.find((event) => event.type === "call" && event.tool === "read_doc"),
    readResult: events.find((event) => event.type === "result" && event.tool === "read_doc"),
    answer: events.at(-1),
  };
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
    const timer = setTimeout(async () => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      try {
        await worker.terminate();
      } catch (error) {
        rejectPromise(new AggregateError(
          [error],
          `stress target ${target} exceeded ${timeout} ms and worker termination failed`,
        ));
        return;
      }
      rejectPromise(new Error(`stress target ${target} exceeded ${timeout} ms`));
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
  assert.equal(validateScenarioDocument(fixture()), 17);
});

test("the reviewed semantic topic selections identify published topics", () => {
  const published = loadPublishedLanguageReference(repositoryRoot);
  const ids = new Set(published.catalog.topics.map((topic) => topic.id));
  for (const id of ["schemas", "effects-handlers", "modules-imports-packages"]) {
    assert.ok(ids.has(id), `${id} must be a published topic`);
  }
});

test("accepts a successful read from the archived snapshot retained by discovery", () => {
  const published = loadPublishedLanguageReference(repositoryRoot);
  const snapshots = loadSnapshotEvidence(repositoryRoot, published);
  const archivedDigest = [...snapshots.keys()][0];
  const uri = `veln-doc:///language/snapshot/${archivedDigest}/topic/modules-legacy-layout`;
  const read = expectedSnapshotRead(uri, published, snapshots);

  assert.equal(read.uri, uri);
  assert.equal(read.name, "modules-legacy-layout");
  assert.match(read.text, /The archived snapshot described a module layout/);
  assert.match(read.text, new RegExp(archivedDigest, "u"));
});

test("lists all topics before reporting that unrelated vocabulary has no relevant topic", () => {
  const document = fixture();
  const turn = scenario(document, "language-unrelated-vocabulary").turns[1];
  assert.ok([...turn.request.text].length > 256);
  assert.deepEqual(turn.events.map((event) => event.type), ["call", "result", "answer"]);
  assert.equal(turn.events[0].tool, "list_language_topics");
  assert.equal(turn.events[2].status, "no_match");
  assert.equal(validateScenarioDocument(document, options), 17);
});

test("rejects fallback data on an unrelated-topic answer", () => {
  const document = fixture();
  scenario(document, "language-unrelated-vocabulary").turns[1].events[2].fallback = "model-memory";
  assert.throws(
    () => validateScenarioDocument(document, options),
    /no-match answer: fields must match the closed shape/,
  );
});

test("bounds local schema reference traversal", () => {
  const reference = (name) => ({ $ref: `#/$defs/${name}` });
  const accepted = { $defs: {}, ...reference("level-0") };
  for (let index = 0; index < 64; index += 1) {
    accepted.$defs[`level-${index}`] = index === 63
      ? { type: "object" }
      : reference(`level-${index + 1}`);
  }
  assert.doesNotThrow(() => validateSchema({}, accepted, accepted, "bounded schema"));

  const tooDeep = structuredClone(accepted);
  tooDeep.$defs["level-63"] = reference("level-64");
  tooDeep.$defs["level-64"] = { type: "object" };
  assert.throws(
    () => validateSchema({}, tooDeep, tooDeep, "deep schema"),
    /schema validation exceeded the 64-reference depth bound/,
  );

  const cyclic = { $defs: { loop: reference("loop") }, ...reference("loop") };
  assert.throws(
    () => validateSchema({}, cyclic, cyclic, "cyclic schema"),
    /schema reference cycle includes #\/\$defs\/loop/,
  );
});

test("rejects over-depth schema references in either JSON property order", () => {
  const schema = {
    type: "object",
    required: ["shallow", "deep"],
    additionalProperties: false,
    properties: {
      shallow: { $ref: "#/$defs/shared" },
      deep: { $ref: "#/$defs/level-0" },
    },
    $defs: {
      shared: { $ref: "#/$defs/overflow" },
      overflow: { type: "string" },
    },
  };
  for (let index = 0; index < 63; index += 1) {
    schema.$defs[`level-${index}`] = index === 62
      ? { $ref: "#/$defs/shared" }
      : { $ref: `#/$defs/level-${index + 1}` };
  }

  for (const value of [
    JSON.parse('{"shallow":"same","deep":"same"}'),
    JSON.parse('{"deep":"same","shallow":"same"}'),
  ]) {
    assert.throws(
      () => validateSchema(value, schema, schema, "ordered schema"),
      /schema validation exceeded the 64-reference depth bound/,
    );
  }
});

test("rejects a branching schema reference cycle within the external time bound", async () => {
  await assert.rejects(
    runStressTarget("schema-branching-cycle", {}, 3_000),
    /schema reference cycle includes #\/\$defs\/loop/,
  );
});

test("validates a branching schema DAG within the external time bound", async () => {
  assert.equal(await runStressTarget("schema-branching-dag", { levels: 30 }, 3_000), 30);
});

test("rejects a failing branching schema DAG within the external time bound", async () => {
  await assert.rejects(
    runStressTarget("schema-failing-branching-dag", { levels: 30 }, 3_000),
    /value does not match exactly one published schema branch/,
  );
});

test("validates a shared-child inline schema DAG within the external time bound", async () => {
  assert.equal(await runStressTarget("schema-inline-branching-dag", { levels: 30 }, 3_000), 30);
});

test("bounds distinct inline schema children within the external time bound", async () => {
  await assert.rejects(
    runStressTarget("schema-inline-distinct-children", { children: 16_384 }, 3_000),
    /schema validation exceeded the 16384-operation work bound/,
  );
  await assert.rejects(
    runStressTarget("schema-inline-distinct-children", { children: 16_385 }, 3_000),
    /exceed the 16384-child fixture bound/,
  );
});

test("rejects deeply nested inline schemas within the external time bound", async () => {
  for (const kind of ["array", "object"]) {
    await assert.rejects(
      runStressTarget("schema-inline-depth", { kind, levels: 2_500 }, 3_000),
      /schema validation exceeded the 128-traversal depth bound/,
    );
  }
});

test("request selection applies reviewed semantics for every raw corpus request", () => {
  const document = fixture();
  for (const entry of document.request_selection) {
    assert.equal(
      selectRequestRoute({ action: entry.action, subject: entry.subject }, candidateOptions),
      entry.route,
      entry.id,
    );
  }
});

test("reviewed raw requests distinguish semantic routing contrasts", () => {
  const cases = new Map(fixture().request_selection.map((entry) => [entry.text, entry]));
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
  ]) {
    const entry = cases.get(text);
    assert.ok(entry, text);
    assert.equal(
      selectRequestRoute({ action: entry.action, subject: entry.subject }, candidateOptions),
      route,
      text,
    );
  }
});

test("rejects invalid request semantics and contradictory corpus routes", () => {
  assert.throws(() => selectRequestRoute({ action: "information" }, candidateOptions), /fields must match/);
  assert.throws(
    () => selectRequestRoute({ action: "guess", subject: "implementation" }, candidateOptions),
    /unknown requested action class/,
  );
  const document = fixture();
  document.request_selection.find((entry) => entry.id === "effects-passive-information").subject = "implementation";
  assert.throws(
    () => validateScenarioDocument(document, candidateOptions),
    /replayed semantics selected the wrong observable route/,
  );
});

test("requires semantic evidence for every replayed request", () => {
  const document = fixture();
  scenario(document, "language-match").turns[0].request.text =
    "What can I test in Veln schema fields?";
  assert.throws(() => validateScenarioDocument(document, candidateOptions), /no independent semantic annotation/);
});

test("checks every corpus row against independent text and observable routes", () => {
  const document = fixture();
  for (const [index, selection] of document.request_selection.entries()) {
    for (const [field, value] of [
      ["text", `${selection.text} changed`],
      ["route", selection.route === "language" ? "repository" : "language"],
    ]) {
      const mutated = structuredClone(document);
      mutated.request_selection[index][field] = value;
      assert.throws(
        () => validateScenarioDocument(mutated, candidateOptions),
        /differs from the independent corpus oracle/,
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
  scenario(document, "listing-unavailable").turns[1].events.at(-1).retained_result.claims = [];
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

test("scales published search excerpts near the accepted field and query limits", async () => {
  const result = await runStressTarget("published-search-scaling", {
    sizes: [900_000, 1_800_000],
    tokenCounts: [250, 500],
  }, 10_000);
  assert.deepEqual(result.excerptLengths, [160, 160]);
  assert.ok(result.milliseconds[1] <= result.milliseconds[0] * 3.25 + 20, result.milliseconds);
});

test("preserves Unicode whitespace trimming semantics", async () => {
  const result = await runStressTarget("normalization-values", {
    values: ["\u0085 schema \u3000", "a   b", "\u2003\u2003"],
  });
  assert.deepEqual(result, ["schema", "a   b", ""]);
});

test("bounds candidate skill bytes at the accepted boundary", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-skill-bytes-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const path = join(root, "SKILL.md");
  const canonical = readFileSync(join(repositoryRoot, ".agents", "skills", "veln-language", "SKILL.md"), "utf8");
  const contractEnd = canonical.lastIndexOf("\n}\n```\n<!-- veln-language-contract:end -->");
  assert.notEqual(contractEnd, -1);
  const padding = 16_384 - Buffer.byteLength(canonical, "utf8");
  assert.ok(padding >= 0, "canonical skill already exceeds its byte limit");
  const accepted = `${canonical.slice(0, contractEnd)}${" ".repeat(padding)}${canonical.slice(contractEnd)}`;
  assert.equal(Buffer.byteLength(accepted, "utf8"), 16_384);
  writeFileSync(path, accepted);
  assert.equal(
    selectRequestRoute({ action: "information", subject: "language_behavior" }, { skillPath: path }),
    "language",
  );

  writeFileSync(path, `${accepted} `);
  assert.throws(
    () => selectRequestRoute({ action: "information", subject: "language_behavior" }, { skillPath: path }),
    /veln-language skill exceeds the byte limit/,
  );
  assert.throws(
    () => selectRequestRoute(
      { action: "information", subject: "language_behavior" },
      { skillText: "x".repeat(16_385) },
    ),
    /veln-language skill exceeds the byte limit/,
  );
});

test("bounds case-folding bytes at the accepted boundary", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-case-fold-bytes-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const directory = join(root, "crates", "veln-project", "testdata");
  mkdirSync(directory, { recursive: true });
  const path = join(directory, "case_folding_17_c_f.txt");
  const accepted = "0041;0061\n#".padEnd(32_768, "x");
  writeFileSync(path, accepted);
  assert.equal(loadCaseFoldMappings(root).get("A"), "a");
  writeFileSync(path, `${accepted}x`);
  assert.throws(() => loadCaseFoldMappings(root), /case-folding data exceeds the byte limit/);
});

test("bounds the catalog digest sidecar at its canonical byte length", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-digest-bytes-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const generated = join(root, "tools", "veln-repo-language-reference", "generated");
  const caseFoldDirectory = join(root, "crates", "veln-project", "testdata");
  mkdirSync(generated, { recursive: true });
  mkdirSync(caseFoldDirectory, { recursive: true });
  const catalog = { topics: [] };
  writeFileSync(join(generated, "language-reference-catalog-v1.json"), `${JSON.stringify(catalog)}\n`);
  const sidecarPath = join(generated, "language-reference-catalog-v1.sha256");
  const sidecar = `${digestCatalog(catalog)}\n`;
  assert.equal(Buffer.byteLength(sidecar), 65);
  writeFileSync(sidecarPath, sidecar);
  writeFileSync(join(caseFoldDirectory, "case_folding_17_c_f.txt"), "0041;0061\n");
  assert.equal(loadPublishedLanguageReference(root).digest, digestCatalog(catalog));
  writeFileSync(sidecarPath, `${sidecar}x`);
  assert.throws(
    () => loadPublishedLanguageReference(root),
    /digest sidecar exceeds the byte limit/,
  );
});

test("bounds each MCP schema at the accepted byte boundary", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-schema-bytes-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const directory = join(root, "crates", "veln-mcp", "schemas", "mcp", "v1");
  mkdirSync(directory, { recursive: true });
  const names = [
    "list-language-topics-input",
    "list-language-topics-result",
    "search-docs-input",
    "search-docs-result",
    "read-doc-input",
    "read-doc-result",
  ];
  const accepted = "{}".padEnd(16_384, " ");
  for (const name of names) writeFileSync(join(directory, `${name}.json`), accepted);
  assert.deepEqual(loadToolSchemas(root), {
    listInput: {},
    listResult: {},
    searchInput: {},
    searchResult: {},
    readInput: {},
    readResult: {},
  });
  for (const name of names) {
    writeFileSync(join(directory, `${name}.json`), `${accepted} `);
    assert.throws(() => loadToolSchemas(root), new RegExp(`${name} MCP schema exceeds the byte limit`));
    writeFileSync(join(directory, `${name}.json`), accepted);
  }
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

test("rejects arguments on language-topic discovery", () => {
  const document = fixture();
  matchingTurn(document).events[0].arguments.scope = "language";
  assert.throws(() => validateScenarioDocument(document, options), /unexpected schema field scope/);
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

test("maps a long decomposed canonical match into the returned excerpt", () => {
  const title = `${"x".repeat(180)}Cafe\u0301`;
  const results = expectedPublishedSearch(
    { query: "Café", scope: "language" },
    syntheticPublished(syntheticTopic({ title })),
  ).results;
  assert.equal(results.length, 1);
  assert.equal([...results[0].excerpt].length, 160);
  assert.equal(results[0].excerpt.endsWith("Cafe\u0301"), true);
  assert.equal(results[0].prefix_truncated, true);
  assert.equal(results[0].suffix_truncated, false);
});

test("rejects search arguments on language-topic discovery", () => {
  const document = fixture();
  matchingTurn(document).events[0].arguments.query = "Veln effects";
  assert.throws(() => validateScenarioDocument(document, options), /unexpected schema field query/);
});

test("selects a listed topic for a non-English request without forwarding query text", () => {
  const document = fixture();
  const turn = scenario(document, "language-topic-selection").turns[0];
  assert.equal(turn.request.text, "Veln のスキーマはどのように値をエンコードしますか？");
  assert.deepEqual(turn.events[0].arguments, {});
  assert.deepEqual(turn.expected.selected_topic_ids, ["schemas"]);
  assert.match(turn.events[2].arguments.uri, /\/topic\/schemas$/u);
  assert.equal(validateScenarioDocument(document, options), 17);
});

test("rejects a fixture topic selection that differs from the reviewed semantic selection", () => {
  const document = fixture();
  const turn = scenario(document, "language-topic-selection").turns[0];
  turn.expected.selected_topic_ids = ["contracts"];
  turn.events[2].arguments.uri = turn.events[2].arguments.uri.replace(/schemas$/u, "contracts");
  assert.throws(() => validateScenarioDocument(document, options), /reviewed semantic topic selection/);
});

test("rejects a listed but semantically unselected topic", () => {
  const document = fixture();
  matchingTurn(document).events[2].arguments.uri = structured(matchingTurn(document).events[1]).topics
    .find((topic) => topic.uri.endsWith("/contracts")).uri;
  assert.throws(() => validateScenarioDocument(document, options), /exactly match the selected listed topic/);
});

test("gives an explicit repository path precedence over a competing MCP subject", () => {
  const document = fixture();
  scenario(document, "repository-current").turns[0].request.text =
    "Inspect MCP documentation search in docs/specification/types.md.";
  assert.throws(() => validateScenarioDocument(document, options), /acceptance label does not match request-selected repository/);
});

test("requires the deep explicit path to use the direct two-read selector", () => {
  const document = fixture();
  const turn = scenario(document, "repository-explicit-path").turns[0];
  assert.deepEqual(turn.events.filter((event) => event.type === "read").map((event) => event.path), [
    "docs/README.md",
    "docs/specification/language-reference-catalog.md",
  ]);
  delete turn.events[0].value.selection;
  assert.throws(() => validateScenarioDocument(document, options), /fields must match the closed shape/);
});

test("rejects a read before language-topic discovery", () => {
  const document = fixture();
  matchingTurn(document).events[0].tool = "read_doc";
  assert.throws(() => validateScenarioDocument(document, options), /language route must list topics first/);
});

test("rejects a fallback field on a language-topic discovery call", () => {
  const document = fixture();
  matchingTurn(document).events[0].fallback = "model-memory";
  assert.throws(() => validateScenarioDocument(document, options), /topic-list call event: fields must match the closed shape/);
});

test("rejects an unknown field on a language read call", () => {
  const document = fixture();
  matchingTurn(document).events[2].retry = false;
  assert.throws(() => validateScenarioDocument(document, options), /read call event: fields must match the closed shape/);
});

test("rejects a noncanonical snapshot URI", () => {
  const document = fixture();
  const event = matchingTurn(document).events[1];
  structured(event).topics[0].uri = "veln-doc:///language/snapshot/placeholder/topic/schemas";
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /canonical snapshot digest/);
});

test("rejects an incomplete listed topic", () => {
  const document = fixture();
  const event = matchingTurn(document).events[1];
  delete structured(event).topics[0].summary;
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /missing schema field summary/);
});

test("rejects listed metadata that drifts from the checked language-reference artifact", () => {
  const document = fixture();
  const event = matchingTurn(document).events[1];
  structured(event).topics[0].summary = "A shortened recording.";
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /topics must be in URI order|differs from the checked snapshot artifact/);
});

test("rejects incomplete read metadata", () => {
  const document = fixture();
  const event = matchingTurn(document).events[3];
  delete structured(event).mimeType;
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /does not match exactly one published schema branch/);
});

test("rejects a listed topic outside the published schema", () => {
  const document = fixture();
  const event = matchingTurn(document).events[1];
  structured(event).topics[0].fallback = true;
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /unexpected schema field fallback/);
});

test("rejects a topic URI not returned by discovery", () => {
  const document = fixture();
  matchingTurn(document).events[2].arguments.uri =
    "veln-doc:///language/snapshot/0000000000000000000000000000000000000000000000000000000000000000/topic/schemas";
  assert.throws(() => validateScenarioDocument(document, options), /exactly match the selected listed topic/);
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
  assert.throws(() => validateScenarioDocument(document, options), /differs from the checked listed snapshot artifact/);
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
  assert.throws(() => validateScenarioDocument(document, options), /must only report the topic absence/);
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
  assert.throws(() => validateScenarioDocument(document, options), /result kind disagrees with the MCP error state/);
});

test("rejects a topic-list result with both error and value", () => {
  const document = fixture();
  scenario(document, "listing-unavailable").turns[1].events[1].value =
    structured(matchingTurn(document).events[1]);
  assert.throws(() => validateScenarioDocument(document, options), /exactly one of error or value/);
});

test("replays a malformed topic-list value as the generic bounded outcome", () => {
  const document = fixture();
  const turn = scenario(document, "listing-failed").turns[1];
  assert.equal(turn.events[1].kind, "malformed_result");
  assert.deepEqual(turn.events[1].value, { content: "not-an-array" });
  assert.equal(validateScenarioDocument(document, options), 17);
});

test("rejects a valid topic-list envelope labeled as malformed", () => {
  const document = fixture();
  const malformed = scenario(document, "listing-failed").turns[1].events[1];
  malformed.value = matchingTurn(document).events[1].value;
  assert.throws(
    () => validateScenarioDocument(document, options),
    /malformed_result value is a valid MCP tool envelope/,
  );
});

test("rejects a valid topic-list tool-error envelope labeled as malformed", () => {
  const document = fixture();
  const malformed = scenario(document, "listing-failed").turns[1].events[1];
  malformed.value = {
    content: [{ type: "text", text: '{"code":"timeout"}' }],
    structuredContent: { code: "timeout" },
    isError: true,
  };
  assert.throws(
    () => validateScenarioDocument(document, options),
    /malformed_result value is a valid MCP tool envelope/,
  );
});

test("routes an unmatched topic-list transport failure to the generic bounded outcome", () => {
  const document = fixture();
  const turn = scenario(document, "listing-failed").turns[1];
  turn.events[1] = { type: "result", tool: "list_language_topics", kind: "transport_error", error: { code: "timeout" } };
  assert.equal(validateScenarioDocument(document, options), 17);
});

test("rejects a read result with both error and value", () => {
  const document = fixture();
  scenario(document, "topic-unreadable").turns[1].events[3].value =
    matchingTurn(document).events[3].value;
  assert.throws(() => validateScenarioDocument(document, options), /exactly one of error or value/);
});

test("routes an unmatched read transport failure to the generic bounded outcome", () => {
  const document = fixture();
  const turn = scenario(document, "topic-read-failed").turns[1];
  turn.events[3] = { type: "result", tool: "read_doc", kind: "transport_error", error: { code: "invalid_arguments" } };
  assert.equal(validateScenarioDocument(document, options), 17);
});

test("replays a malformed read value as the generic bounded outcome", () => {
  const document = fixture();
  const turn = scenario(document, "topic-read-failed").turns[1];
  assert.equal(turn.events[3].kind, "malformed_result");
  assert.deepEqual(turn.events[3].value, { structuredContent: null, isError: false });
  assert.equal(validateScenarioDocument(document, options), 17);
});

test("rejects a valid read envelope labeled as malformed", () => {
  const document = fixture();
  const malformed = scenario(document, "topic-read-failed").turns[1].events[3];
  malformed.value = matchingTurn(document).events[3].value;
  assert.throws(
    () => validateScenarioDocument(document, options),
    /malformed_result value is a valid MCP tool envelope/,
  );
});

test("rejects a valid read tool-error envelope labeled as malformed", () => {
  const document = fixture();
  const malformed = scenario(document, "topic-read-failed").turns[1].events[3];
  malformed.value = {
    content: [{ type: "text", text: '{"code":"timeout"}' }],
    structuredContent: { code: "timeout" },
    isError: true,
  };
  assert.throws(
    () => validateScenarioDocument(document, options),
    /malformed_result value is a valid MCP tool envelope/,
  );
});

test("dispatches resource_not_found by result kind as well as code", () => {
  const document = fixture();
  const turn = scenario(document, "topic-read-failed").turns[1];
  turn.events[3] = { type: "result", tool: "read_doc", kind: "transport_error", error: { code: "resource_not_found" } };
  assert.equal(validateScenarioDocument(document, options), 17);
});

test("keeps named and generic read failures bounded after server replacement", () => {
  const document = fixture();
  const events = staleEvents(document);
  assert.equal(events.transition.transition, "replace_process");
  const selectedUri = events.readCall.arguments.uri;
  const contract = {
    failure: {
      topic_unavailable: { result: { kind: "transport_error", code: "transport_unavailable" } },
      stale_snapshot: {
        result: { kind: "tool_error", code: "resource_not_found", selected_uri_must_match: true },
      },
    },
  };
  assert.equal(
    classifyReadFailure("transport_error", "transport_unavailable", undefined, selectedUri, contract),
    "topic_unavailable",
  );
  assert.equal(
    classifyReadFailure("malformed_result", "invalid_response", undefined, selectedUri, contract),
    "topic_read_failed",
  );
  assert.equal(
    classifyReadFailure("tool_error", "timeout", selectedUri, selectedUri, contract),
    "topic_read_failed",
  );
  assert.equal(
    classifyReadFailure("tool_error", "resource_not_found", `${selectedUri}-different`, selectedUri, contract),
    "topic_read_failed",
  );
});

test("rejects a selected resource beyond the published byte limit", () => {
  const document = fixture();
  const event = matchingTurn(document).events[3];
  structured(event).text = "Sentence. ".repeat(30_000);
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /published byte limit/);
});

test("rejects read text that drifts from the checked listed snapshot artifact", () => {
  const document = fixture();
  const turn = matchingTurn(document);
  const statement = "A replacement recording.";
  structured(turn.events[3]).text = statement;
  syncEnvelope(turn.events[3]);
  turn.expected.answer_claims = [statement];
  turn.events[4].claims = [statement];
  assert.throws(() => validateScenarioDocument(document, options), /differs from the checked listed snapshot artifact/);
});

test("rejects incomplete failure provenance", () => {
  const document = fixture();
  delete scenario(document, "topic-unreadable").turns[1].events[4].failure.artifact_uri;
  assert.throws(() => validateScenarioDocument(document, options), /failed operation and artifact/);
});

test("rejects a stale result without the schema-required message", () => {
  const document = fixture();
  const event = staleEvents(document).readResult;
  delete event.value.structuredContent.message;
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /value does not match exactly one published schema branch/);
});

test("rejects a stale-snapshot row that uses the current published digest", () => {
  const document = fixture();
  const events = staleEvents(document);
  const staleUri = events.readCall.arguments.uri;
  const currentUri = staleUri.replace(
    /snapshot\/[0-9a-f]{64}\//u,
    "snapshot/8e9bbfb545aab06e6549331872f47399b52ec4f3e686fd7efc6c0fba5d0ef9c2/",
  );
  for (const topic of structured(events.listResult).topics) {
    topic.uri = topic.uri.replace(/snapshot\/[0-9a-f]{64}\//u,
      "snapshot/8e9bbfb545aab06e6549331872f47399b52ec4f3e686fd7efc6c0fba5d0ef9c2/");
  }
  syncEnvelope(events.listResult);
  events.transition.before.language_snapshot_digest = events.transition.after.language_snapshot_digest;
  events.readCall.arguments.uri = currentUri;
  structured(events.readResult).details.uri = currentUri;
  syncEnvelope(events.readResult);
  events.answer.failure.artifact_uri = currentUri;
  assert.throws(
    () => validateScenarioDocument(document, options),
    /differs from the checked snapshot artifact|stale snapshot URI must differ/,
  );
});

test("rejects mutation of an unselected stale listed topic", () => {
  const document = fixture();
  const event = staleEvents(document).listResult;
  structured(event).topics[1].summary = "Fabricated historical summary.";
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /topics must be in URI order|differs from the checked snapshot artifact/);
});

test("rejects a fabricated unselected stale listed topic", () => {
  const document = fixture();
  const event = staleEvents(document).listResult;
  const snapshotDigest = structured(event).topics[0].uri.match(
    /snapshot\/([0-9a-f]{64})\//u,
  )[1];
  structured(event).topics.push({
    uri: `veln-doc:///language/snapshot/${snapshotDigest}/topic/fabricated-modules`,
    title: "Fabricated Modules",
    summary: "This topic has no snapshot evidence.",
  });
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /topics must be in URI order|differs from the checked snapshot artifact/);
});

test("rejects stale topic listings from mixed snapshots", () => {
  const document = fixture();
  const event = staleEvents(document).listResult;
  structured(event).topics[1].uri = structured(event).topics[1].uri.replace(
    /snapshot\/[0-9a-f]{64}\//u,
    "snapshot/f11c4d28cf8dd18f23d9a112e7f0fee721225c1980baa94a673fb9090b06e175/",
  );
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /topics must be in URI order|must belong to one snapshot/);
});

test("rejects stale topic listings without snapshot evidence", () => {
  const document = fixture();
  const event = staleEvents(document).listResult;
  for (const topic of structured(event).topics) {
    topic.uri = topic.uri.replace(
      /snapshot\/[0-9a-f]{64}\//u,
      "snapshot/1111111111111111111111111111111111111111111111111111111111111111/",
    );
  }
  syncEnvelope(event);
  assert.throws(() => validateScenarioDocument(document, options), /no checked snapshot evidence/);
});

test("rejects stale-snapshot evidence without a server-process replacement", () => {
  const document = fixture();
  const turn = staleTurn(document);
  turn.events.splice(turn.events.indexOf(staleEvents(document).transition), 1);
  assert.throws(
    () => validateScenarioDocument(document, options),
    /stale snapshot requires a recorded server-process replacement/,
  );
});

test("rejects a server replacement that keeps the same process identity", () => {
  const document = fixture();
  const transition = staleEvents(document).transition;
  transition.after.server_instance = transition.before.server_instance;
  assert.throws(
    () => validateScenarioDocument(document, options),
    /server replacement must identify distinct process instances/,
  );
});

test("rejects a server replacement whose previous state did not produce search results", () => {
  const document = fixture();
  staleEvents(document).transition.before.language_snapshot_digest =
    staleEvents(document).transition.after.language_snapshot_digest;
  assert.throws(
    () => validateScenarioDocument(document, options),
    /previous server must retain the snapshot that produced the listing/,
  );
});

test("rejects a server replacement whose new state is not the checked published snapshot", () => {
  const document = fixture();
  staleEvents(document).transition.after.language_snapshot_digest =
    "1111111111111111111111111111111111111111111111111111111111111111";
  assert.throws(
    () => validateScenarioDocument(document, options),
    /replacement server must use the checked published snapshot/,
  );
});

test("rejects a misplaced server replacement after the stale read", () => {
  const document = fixture();
  const turn = staleTurn(document);
  const transitionIndex = turn.events.indexOf(staleEvents(document).transition);
  const [transition] = turn.events.splice(transitionIndex, 1);
  turn.events.splice(turn.events.length - 1, 0, transition);
  assert.throws(
    () => validateScenarioDocument(document, options),
    /matching route must have one listing and one read/,
  );
});

test("rejects mutation of the earlier result after failure", () => {
  const document = fixture();
  scenario(document, "listing-unavailable").turns[1].events[2].retained_result.claims[0] = "Changed after failure.";
  assert.throws(() => validateScenarioDocument(document, options), /earlier result changed/);
});

test("rejects removal from the complete earlier result after failure", () => {
  const document = fixture();
  delete scenario(document, "listing-unavailable").turns[1].events[2].retained_result.source_uris;
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

test("accepts a terminal repository authority at the document byte limit", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-authority-boundary-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const authority = join(root, "authority.md");
  const frontmatter = "---\nrole: reference\nauthority: supporting\n---\n";
  writeFileSync(authority, frontmatter.padEnd(262_144, "x"));
  assert.equal(currentRepositoryAuthority(authority), "reference");
});

test("rejects an oversized directly linked terminal repository authority", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-authority-oversized-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "README.md"), "[authority](authority.md)\n");
  const authority = join(root, "docs", "authority.md");
  const frontmatter = "---\nrole: reference\nauthority: supporting\n---\n";
  writeFileSync(authority, frontmatter.padEnd(262_145, "x"));

  assert.deepEqual(
    shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 3),
    ["docs/README.md", "docs/authority.md"],
  );
  assert.throws(
    () => currentRepositoryAuthority(authority),
    /repository document exceeds the byte limit/,
  );
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
    ">     [block-quoted indented-code example](authority.md)",
    "",
    "> >     [nested block-quoted indented-code example](authority.md)",
    "",
    "\\[escaped-link example](authority.md)",
    "",
    "<!-- [comment example](authority.md) -->",
    "",
    "<pre>[raw HTML example](authority.md)</pre>",
    "",
    "![image](authority.md)",
  ].join("\n"));

  assert.throws(
    () => shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 2),
    /repository authority is not reachable/,
  );
});

test("ignores links in CommonMark closed HTML blocks and resumes navigation afterward", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-closed-html-blocks-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "fake.md"), "# Fake authority\n");
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  writeFileSync(join(root, "docs", "README.md"), [
    "<!--",
    "[comment](fake.md)",
    "-->",
    "<?processing",
    "[instruction](fake.md)",
    "?>",
    "<!DECLARATION",
    "[declaration](fake.md)",
    ">",
    "<![CDATA[",
    "[cdata](fake.md)",
    "]]>",
    "[authority](authority.md)",
  ].join("\n"));

  assert.deepEqual(
    linkedDocumentationPaths("docs/README.md", root),
    ["docs/authority.md"],
  );
});

test("does not let HTML-looking fenced text hide a following route", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-fenced-html-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "fake.md"), "# Fake authority\n");
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  writeFileSync(join(root, "docs", "README.md"), [
    "```markdown",
    "<section>",
    "[fake](fake.md)",
    "```",
    "[authority](authority.md)",
  ].join("\n"));

  assert.deepEqual(
    linkedDocumentationPaths("docs/README.md", root),
    ["docs/authority.md"],
  );
});

test("discovers full, collapsed, and shortcut reference links", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-reference-links-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  for (const name of ["full.md", "collapsed.md", "shortcut.md", "inline.md"]) {
    writeFileSync(join(root, "docs", name), `# ${name}\n`);
  }
  writeFileSync(join(root, "docs", "README.md"), [
    "[full route][full]",
    "[collapsed][]",
    "[shortcut]",
    "[inline](inline.md)",
    "",
    "[full]: full.md",
    "[collapsed]: collapsed.md",
    "[shortcut]: shortcut.md",
    "[inline]: shortcut.md",
  ].join("\n"));

  assert.deepEqual(
    linkedDocumentationPaths("docs/README.md", root),
    ["docs/full.md", "docs/collapsed.md", "docs/shortcut.md", "docs/inline.md"],
  );
});

test("ignores links in block-quoted tilde fences and resumes navigation afterward", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-block-quote-fence-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "fake.md"), "# Fake authority\n");
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  writeFileSync(join(root, "docs", "README.md"), [
    "> ~~~md",
    "> [fake](fake.md)",
    "> ~~~",
    "",
    "> ~~~md",
    "> [also fake](fake.md)",
    "[authority](authority.md)",
  ].join("\n"));

  assert.deepEqual(
    linkedDocumentationPaths("docs/README.md", root),
    ["docs/authority.md"],
  );
});

test("discovers a repository route with a titled Markdown link", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-titled-link-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  for (const title of ["Authority", "((("]) {
    writeFileSync(join(root, "docs", "README.md"), `[authority](authority.md "${title}")\n`);
    assert.deepEqual(
      shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 2),
      ["docs/README.md", "docs/authority.md"],
    );
  }
});

test("discovers inline repository routes with balanced parenthesized destinations", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-parenthesized-links-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs", "route(one(two))"), { recursive: true });
  writeFileSync(join(root, "docs", "route(one(two))", "authority.md"), "# Authority\n");
  writeFileSync(join(root, "docs", "README.md"), [
    "[bare](route(one(two))/authority.md)",
    "[angle](<route(one(two))/authority.md>)",
  ].join("\n"));

  assert.deepEqual(
    linkedDocumentationPaths("docs/README.md", root),
    [
      "docs/route(one(two))/authority.md",
      "docs/route(one(two))/authority.md",
    ],
  );
});

test("accepts one line ending between inline-link components", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-link-line-ending-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs", "route(one)"), { recursive: true });
  writeFileSync(join(root, "docs", "route(one)", "authority.md"), "# Authority\n");

  for (const lineEnding of ["\n", "\r\n"]) {
    writeFileSync(join(root, "docs", "README.md"), [
      "[authority](",
      "route(one)/authority.md",
      "\"Authority\"",
      ")",
    ].join(lineEnding));
    assert.deepEqual(
      shortestDocumentationRoute(
        "docs/README.md",
        "docs/route(one)/authority.md",
        root,
        2,
      ),
      ["docs/README.md", "docs/route(one)/authority.md"],
      JSON.stringify(lineEnding),
    );
  }
});

test("rejects blank lines around inline-link destinations", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-link-blank-lines-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");

  for (const source of [
    "[authority](\n\nauthority.md)",
    "[authority](authority.md\n\n)",
    "[authority](authority.md\n\n\"Authority\")",
  ]) {
    writeFileSync(join(root, "docs", "README.md"), source);
    assert.deepEqual(linkedDocumentationPaths("docs/README.md", root), [], source);
  }
});

test("does not combine an unmatched link label with a later image destination", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-link-atoms-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  writeFileSync(join(root, "docs", "README.md"), "[unmatched label\n![image](authority.md)\n");

  assert.throws(
    () => shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 2),
    /repository authority is not reachable/,
  );
});

test("does not treat an outer nested Markdown link as navigation", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-nested-link-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "inner.md"), "# Inner\n");
  writeFileSync(join(root, "docs", "outer.md"), "# Outer\n");
  writeFileSync(join(root, "docs", "README.md"), "[[inner](inner.md)](outer.md)\n");

  assert.deepEqual(
    shortestDocumentationRoute("docs/README.md", "docs/inner.md", root, 2),
    ["docs/README.md", "docs/inner.md"],
  );
  assert.throws(
    () => shortestDocumentationRoute("docs/README.md", "docs/outer.md", root, 2),
    /repository authority is not reachable/,
  );
});

test("finds a valid link after repeated malformed destinations", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-link-after-malformed-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  writeFileSync(
    join(root, "docs", "README.md"),
    `${"[](".repeat(8)} [authority](authority.md)\n`,
  );

  assert.deepEqual(
    shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 2),
    ["docs/README.md", "docs/authority.md"],
  );
});

test("retains a link whose label contains an image", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-image-label-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  writeFileSync(join(root, "docs", "README.md"), "[![icon](icon.png)](authority.md)\n");

  assert.deepEqual(
    shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 2),
    ["docs/README.md", "docs/authority.md"],
  );
});

test("does not close inline code with a different-length backtick run", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-inline-delimiters-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "authority.md"), "# Authority\n");
  writeFileSync(join(root, "docs", "README.md"), "x `[authority](authority.md)``\n");

  assert.deepEqual(
    shortestDocumentationRoute("docs/README.md", "docs/authority.md", root, 2),
    ["docs/README.md", "docs/authority.md"],
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
    value: { route: null },
  });
  assert.throws(() => validateScenarioDocument(document, options), /repeated a path/);
});

test("rejects a repository route beyond the read bound", () => {
  const document = fixture();
  const turn = scenario(document, "repository-unknown").turns[0];
  turn.events[1].value.route = "docs/specification/README.md";
  turn.events.splice(2, 0, {
    type: "read",
    path: "docs/specification/README.md",
    value: { route: "docs/specification/topic-map.md" },
  }, {
    type: "read",
    path: "docs/specification/topic-map.md",
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

test("parses nested brackets with adjacent-size linear scaling", async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-links-nested-brackets-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  const path = join(root, "docs", "README.md");
  const sizes = [131_072, 262_144];
  const result = await runStressTarget("nested-brackets", {
    path,
    repositoryPath: "docs/README.md",
    root,
    sizes,
  }, 1_000);
  assert.deepEqual(result.bytes, sizes);
  assert.deepEqual(result.linkCounts, [0, 0]);
  assert.ok(
    result.milliseconds[1] <= result.milliseconds[0] * 3.5 + 20,
    `nested bracket scaling regressed: ${result.milliseconds.join(" ms, ")} ms`,
  );
});

test("bounds repeated malformed destinations through the accepted size", async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-links-malformed-destinations-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  const path = join(root, "docs", "README.md");
  const sizes = [131_072, 262_144];
  const result = await runStressTarget("repeated-malformed-destinations", {
    path,
    repositoryPath: "docs/README.md",
    root,
    sizes,
  }, 1_000);
  assert.deepEqual(result.bytes, sizes);
  assert.deepEqual(result.linkCounts, [0, 0]);
  assert.ok(
    result.milliseconds[1] <= result.milliseconds[0] * 3.5 + 20,
    `malformed destination scaling regressed: ${result.milliseconds.join(" ms, ")} ms`,
  );
});

test("parses open labels followed by malformed destinations with adjacent-size linear scaling", async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-links-open-label-malformed-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  const path = join(root, "docs", "README.md");
  const sizes = [32_768, 65_536];
  const result = await runStressTarget("open-label-malformed-destinations", {
    path,
    repositoryPath: "docs/README.md",
    root,
    sizes,
  }, 1_000);
  assert.deepEqual(result.bytes, sizes);
  assert.deepEqual(result.linkCounts, [0, 0]);
  assert.ok(
    result.milliseconds[1] <= result.milliseconds[0] * 3.5 + 20,
    `open-label malformed destination scaling regressed: ${result.milliseconds.join(" ms, ")} ms`,
  );
});

test("parses escaped inline-link destinations with adjacent-size scaling", async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-links-escaped-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  const path = join(root, "docs", "README.md");
  const result = await runStressTarget("escaped-destination", {
    path,
    repositoryPath: "docs/README.md",
    root,
    sizes: [131_072, 262_144],
  }, 1_000);
  assert.deepEqual(result.bytes, [131_072, 262_144]);
  assert.deepEqual(result.linkCounts, [1, 1]);
  assert.ok(
    result.milliseconds[1] <= result.milliseconds[0] * 3.5 + 20,
    `escaped destination scaling regressed: ${result.milliseconds.join(" ms, ")} ms`,
  );
});

test("masks mixed inline code and fenced comments at adjacent accepted sizes", async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-links-masking-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  const sourceForSize = (size) => {
    const visible = "`x` ".repeat(Math.floor(size / 8));
    const fence = "\n```\n";
    const suffix = "\n```\n";
    const markers = "<!--x-->".repeat(Math.floor((size - visible.length - fence.length - suffix.length) / 8));
    return `${visible}${fence}${markers}${suffix}`;
  };
  for (const size of [131_072, 262_144]) {
    writeFileSync(join(root, "docs", "README.md"), sourceForSize(size));
    assert.deepEqual(
      await runStressTarget("linked-paths", { path: "docs/README.md", root }),
      [],
      `${size} bytes`,
    );
  }
});

test("parses descending unmatched inline-code runs with linear scaling", async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-links-ticks-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  const path = join(root, "docs", "README.md");
  const result = await runStressTarget("descending-unmatched-ticks", {
    counts: [510, 722],
    path,
    repositoryPath: "docs/README.md",
    root,
    sizes: [131_072, 262_144],
    repetitions: 5,
  }, 1_000);
  assert.deepEqual(result.bytes, [131_072, 262_144]);
  assert.deepEqual(result.runCounts, [510, 722]);
  assert.ok(
    result.milliseconds[1] <= result.milliseconds[0] * 3.5 + 20,
    `descending unmatched tick scaling regressed: ${result.milliseconds.join(" ms, ")} ms`,
  );
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

test("rejects a scenario symlink to a non-regular file without reading it", {
  skip: process.platform !== "linux",
}, async (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-fixture-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const path = join(root, "non-regular.json");
  symlinkSync("/dev/zero", path);
  await assert.rejects(
    runStressTarget("scenario-document", { path }),
    /scenario fixture must be a regular file/,
  );
});

test("rejects a scenario FIFO without waiting for a writer", {
  skip: process.platform !== "linux",
}, (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-fixture-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const path = join(root, "scenario.json");
  const created = spawnSync("mkfifo", [path], { encoding: "utf8" });
  assert.equal(created.status, 0, created.stderr);
  const moduleUrl = new URL("./check-veln-language-skill.mjs", import.meta.url).href;
  const source = `import { readScenarioDocument } from ${JSON.stringify(moduleUrl)}; readScenarioDocument(${JSON.stringify(path)});`;
  const read = spawnSync(process.execPath, ["--input-type=module", "--eval", source], {
    encoding: "utf8",
    timeout: 2_000,
  });
  assert.notEqual(read.error?.code, "ETIMEDOUT", "scenario FIFO read exceeded the external time bound");
  assert.equal(read.status, 1, read.stderr);
  assert.match(read.stderr, /scenario fixture must be a regular file/);
});

test("rejects a repository document symlink swapped outside docs", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-repository-read-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "docs"));
  writeFileSync(join(root, "docs", "inside.md"), "[inside](authority.md)\n");
  writeFileSync(join(root, "outside.md"), "[outside](docs/authority.md)\n");
  const path = join(root, "docs", "route.md");
  symlinkSync("inside.md", path);
  assert.throws(
    () => linkedDocumentationPaths("docs/route.md", root, (candidate, flags) => {
      rmSync(candidate);
      symlinkSync("../outside.md", candidate);
      return openSync(candidate, flags);
    }),
    /repository document escaped/,
  );
});

test("shares recordings at the largest accepted reference boundary", (context) => {
  const root = mkdtempSync(join(tmpdir(), "veln-language-fixture-"));
  context.after(() => rmSync(root, { recursive: true, force: true }));
  const referenceEvent = () => ({
    type: "result",
    tool: "list_language_topics",
    kind: "success",
    value_ref: "language-topics-list",
  });
  const document = {
    schema_version: 1,
    recordings: {
      "language-topics-list": { content: "" },
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
  document.recordings["language-topics-list"].content = "x".repeat(1_000_000 - emptyBytes);
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
    tool: "list_language_topics",
    kind: "success",
    value_ref: "language-topics-list",
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
        "language-topics-list": { content: "x".repeat(100_000) },
        "schemas-read": {},
      },
      request_selection: [],
      scenarios: Array.from({ length: 17 }, scenario),
    };
    if (fixtureCase.name === "turns") document.scenarios[0].turns.push(turn());
    if (fixtureCase.name === "events") {
      document.scenarios[0].turns[0].events = Array.from({ length: 9 }, referenceEvent);
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
