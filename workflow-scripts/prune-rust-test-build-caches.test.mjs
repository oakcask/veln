import assert from "node:assert/strict";
import test from "node:test";

import {
  pruneRustTestBuildCaches,
  selectObsoleteCaches,
} from "./prune-rust-test-build-caches.mjs";

const prefix = "rust-test-build-Linux-";
const currentKey = `${prefix}current-lock-current-revision`;

test("selects only older generations with the same build prefix", () => {
  assert.deepEqual(
    selectObsoleteCaches(
      [
        { id: 1, key: currentKey },
        { id: 2, key: `${prefix}current-lock-previous-revision` },
        { id: 4, key: `${prefix}previous-lock-previous-revision` },
        { id: 3, key: "cargo-dependencies-Linux-lock" },
      ],
      { prefix, currentKey },
    ),
    [
      { id: 2, key: `${prefix}current-lock-previous-revision` },
      { id: 4, key: `${prefix}previous-lock-previous-revision` },
    ],
  );
});

test("rejects malformed cache records before deletion", () => {
  assert.throws(
    () =>
      selectObsoleteCaches([{ id: "2", key: `${prefix}previous-lock` }], {
        prefix,
        currentKey,
      }),
    /every cache needs an integer id and string key/u,
  );
});

test("lists main caches and deletes every obsolete generation", () => {
  const calls = [];
  const notices = [];
  const deleted = pruneRustTestBuildCaches({
    prefix,
    currentKey,
    ref: "refs/heads/main",
    repository: "example/veln",
    runCommand(args) {
      calls.push(args);
      if (args[1] === "list") {
        return {
          status: 0,
          stdout: JSON.stringify([
            { id: 1, key: currentKey },
            { id: 2, key: `${prefix}current-lock-previous-revision` },
            { id: 3, key: `${prefix}previous-lock-previous-revision` },
          ]),
        };
      }
      return { status: 0, stdout: "" };
    },
    notice: (message) => notices.push(message),
  });

  assert.deepEqual(calls, [
    [
      "cache",
      "list",
      "--repo",
      "example/veln",
      "--key",
      prefix,
      "--ref",
      "refs/heads/main",
      "--limit",
      "100",
      "--json",
      "id,key",
    ],
    ["cache", "delete", "2", "--repo", "example/veln"],
    ["cache", "delete", "3", "--repo", "example/veln"],
  ]);
  assert.deepEqual(deleted, [2, 3]);
  assert.deepEqual(notices, [
    `Pruned 2 obsolete main Rust test build caches; ${currentKey} remains the rolling baseline.`,
  ]);
});

test("fails with repair guidance when caches cannot be listed", () => {
  assert.throws(
    () =>
      pruneRustTestBuildCaches({
        prefix,
        currentKey,
        ref: "refs/heads/main",
        repository: "example/veln",
        runCommand: () => ({ status: 1, stdout: "" }),
      }),
    /Inspect GitHub cache permissions and rerun the seed job/u,
  );
});

test("fails with repair guidance when an obsolete cache cannot be deleted", () => {
  let callCount = 0;
  assert.throws(
    () =>
      pruneRustTestBuildCaches({
        prefix,
        currentKey,
        ref: "refs/heads/main",
        repository: "example/veln",
        runCommand() {
          callCount += 1;
          if (callCount === 1) {
            return {
              status: 0,
              stdout: JSON.stringify([{ id: 2, key: `${prefix}previous-lock` }]),
            };
          }
          return { status: 1, stdout: "" };
        },
      }),
    /Delete obsolete cache 2 and rerun the seed job/u,
  );
});
