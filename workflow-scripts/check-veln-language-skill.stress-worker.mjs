import { parentPort, workerData } from "node:worker_threads";

import {
  linkedDocumentationPaths,
  loadSnapshotEvidence,
  shortestDocumentationRoute,
} from "./check-veln-language-skill.mjs";

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
  throw new Error(`unknown stress target: ${target}`);
}

try {
  parentPort.postMessage({ ok: true, value: runTarget(workerData.target, workerData.data) });
} catch (error) {
  parentPort.postMessage({ ok: false, name: error.name, message: error.message });
}
