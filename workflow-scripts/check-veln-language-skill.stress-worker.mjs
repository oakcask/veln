import { parentPort, workerData } from "node:worker_threads";

import { writeFileSync } from "node:fs";

import {
  expectedPublishedSearch,
  linkedDocumentationPaths,
  loadCaseFoldMappings,
  loadSnapshotEvidence,
  normalizeSearchText,
  shortestDocumentationRoute,
  validateSchema,
} from "./check-veln-language-skill.mjs";

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

function runTarget(target, data) {
  if (target === "nontermination") {
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0);
  }
  if (target === "shortest-route") {
    return shortestDocumentationRoute(
      data.entry,
      data.authority,
      data.root,
      data.maximumReads,
      data.discoveryBound,
    );
  }
  if (target === "linked-paths") {
    return linkedDocumentationPaths(data.path, data.root);
  }
  if (target === "descending-unmatched-ticks") {
    const milliseconds = [];
    const bytes = [];
    const descendingRuns = Array.from(
      { length: data.count },
      (_, index) => "`".repeat(data.count - index),
    ).join(" ");
    for (const size of data.sizes) {
      const prefix = `x ${descendingRuns}`;
      const source = prefix.padEnd(size, "x");
      writeFileSync(data.path, source);
      const start = performance.now();
      linkedDocumentationPaths(data.repositoryPath, data.root);
      milliseconds.push(performance.now() - start);
      bytes.push(Buffer.byteLength(source));
    }
    return { bytes, milliseconds };
  }
  if (target === "escaped-destination") {
    const milliseconds = [];
    const bytes = [];
    const linkCounts = [];
    for (const size of data.sizes) {
      const prefix = "[route](";
      const suffix = "authority.md)";
      const source = `${prefix}${"\\".repeat(size - prefix.length - suffix.length)}${suffix}`;
      writeFileSync(data.path, source);
      const start = performance.now();
      const links = linkedDocumentationPaths(data.repositoryPath, data.root);
      milliseconds.push(performance.now() - start);
      bytes.push(Buffer.byteLength(source));
      linkCounts.push(links.length);
    }
    return { bytes, linkCounts, milliseconds };
  }
  if (target === "repeated-malformed-destinations") {
    const bytes = [];
    const linkCounts = [];
    const milliseconds = [];
    for (const size of data.sizes) {
      const source = "[](".repeat(Math.floor(size / 3)).padEnd(size, "x");
      writeFileSync(data.path, source);
      const start = performance.now();
      const links = linkedDocumentationPaths(data.repositoryPath, data.root);
      milliseconds.push(performance.now() - start);
      bytes.push(Buffer.byteLength(source));
      linkCounts.push(links.length);
    }
    return { bytes, linkCounts, milliseconds };
  }
  if (target === "schema-branching-cycle") {
    const reference = { $ref: "#/$defs/loop" };
    const schema = {
      $defs: { loop: { oneOf: [reference, reference] } },
      ...reference,
    };
    validateSchema({}, schema, schema, "branching cyclic schema");
  }
  if (target === "schema-branching-dag") {
    const schema = { $defs: {}, $ref: "#/$defs/level-0" };
    let value = {};
    for (let index = data.levels - 1; index >= 0; index -= 1) {
      const reference = {
        $ref: index === data.levels - 1
          ? "#/$defs/terminal"
          : `#/$defs/level-${index + 1}`,
      };
      const branch = (choice) => ({
        type: "object",
        properties: { next: reference, choice: { const: choice } },
        required: ["next", "choice"],
        additionalProperties: false,
      });
      schema.$defs[`level-${index}`] = {
        oneOf: [branch("left"), branch("right")],
      };
      value = { next: value, choice: "left" };
    }
    schema.$defs.terminal = { type: "object", additionalProperties: false };
    validateSchema(value, schema, schema, "branching schema DAG");
    return data.levels;
  }
  if (target === "schema-failing-branching-dag") {
    const schema = { $defs: {}, $ref: "#/$defs/level-0" };
    let value = {};
    for (let index = data.levels - 1; index >= 0; index -= 1) {
      const reference = {
        $ref: index === data.levels - 1
          ? "#/$defs/terminal"
          : `#/$defs/level-${index + 1}`,
      };
      const branch = (choice) => ({
        type: "object",
        properties: { next: reference, choice: { const: choice } },
        required: ["next", "choice"],
        additionalProperties: false,
      });
      schema.$defs[`level-${index}`] = {
        oneOf: [branch("left"), branch("right")],
      };
      value = { next: value, choice: "left" };
    }
    schema.$defs.terminal = { const: "unreachable" };
    validateSchema(value, schema, schema, "failing branching schema DAG");
  }
  if (target === "schema-inline-depth") {
    let schema = { type: "string" };
    let value = "leaf";
    for (let index = 0; index < data.levels; index += 1) {
      if (data.kind === "array") {
        schema = { type: "array", items: schema };
        value = [value];
      } else {
        schema = {
          type: "object",
          properties: { next: schema },
          required: ["next"],
          additionalProperties: false,
        };
        value = { next: value };
      }
    }
    validateSchema(value, schema, schema, `deep inline ${data.kind} schema`);
  }
  if (target === "snapshot-evidence") {
    const originalStructuredClone = globalThis.structuredClone;
    let cloneCalls = 0;
    globalThis.structuredClone = (...arguments_) => {
      cloneCalls += 1;
      return originalStructuredClone(...arguments_);
    };
    try {
      const loaded = loadSnapshotEvidence(data.root, data.published);
      return {
        cloneCalls,
        size: loaded.size,
        keys: [...loaded.values()].map((snapshot) => Object.keys(snapshot).sort()),
        hasCatalog: [...loaded.values()].map((snapshot) => Object.hasOwn(snapshot, "catalog")),
        serializedLength: JSON.stringify([...loaded.values()]).length,
      };
    } finally {
      globalThis.structuredClone = originalStructuredClone;
    }
  }
  if (target === "normalization-values") {
    return data.values.map((value) => normalizeSearchText(value, new Map()));
  }
  if (target === "normalization-scaling") {
    const milliseconds = [];
    const lengths = [];
    for (const size of data.sizes) {
      const value = `a${" ".repeat(size)}b`;
      normalizeSearchText(value, new Map());
      const samples = [];
      for (let sample = 0; sample < 5; sample += 1) {
        const start = performance.now();
        for (let repetition = 0; repetition < data.repetitions; repetition += 1) {
          lengths.push(normalizeSearchText(value, new Map()).length);
        }
        samples.push(performance.now() - start);
      }
      milliseconds.push(median(samples));
    }
    return {
      lengths: data.sizes.map((_, index) => lengths[(index + 1) * data.repetitions * 5 - 1]),
      milliseconds,
    };
  }
  if (target === "published-search-scaling") {
    const caseFoldMappings = loadCaseFoldMappings();
    const milliseconds = [];
    const excerptLengths = [];
    for (let index = 0; index < data.sizes.length; index += 1) {
      const body = `a${"x".repeat(data.sizes[index])}`;
      const query = `${"a ".repeat(data.tokenCounts[index] - 1)}a`;
      const published = {
        digest: "1".repeat(64),
        caseFoldMappings,
        catalog: {
          topics: [{ id: "topic", title: "Topic", summary: "None", keywords: [], body: [body] }],
        },
      };
      expectedPublishedSearch({ query, scope: "language" }, published);
      const samples = [];
      let result;
      for (let sample = 0; sample < 3; sample += 1) {
        const start = performance.now();
        result = expectedPublishedSearch({ query, scope: "language" }, published);
        samples.push(performance.now() - start);
      }
      milliseconds.push(median(samples));
      excerptLengths.push([...result.results[0].excerpt].length);
    }
    return { excerptLengths, milliseconds };
  }
  throw new Error(`unknown stress target: ${target}`);
}

try {
  parentPort.postMessage({ ok: true, value: runTarget(workerData.target, workerData.data) });
} catch (error) {
  parentPort.postMessage({ ok: false, name: error.name, message: error.message });
}
