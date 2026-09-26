import { parentPort, workerData } from "node:worker_threads";

import {
  linkedDocumentationPaths,
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
  throw new Error(`unknown stress target: ${target}`);
}

try {
  parentPort.postMessage({ ok: true, value: runTarget(workerData.target, workerData.data) });
} catch (error) {
  parentPort.postMessage({ ok: false, name: error.name, message: error.message });
}
