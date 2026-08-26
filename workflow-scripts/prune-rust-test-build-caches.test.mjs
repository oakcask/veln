import assert from "node:assert/strict";
import test from "node:test";

import {
  pruneRustTestBuildCaches,
  selectObsoleteCaches,
} from "./prune-rust-test-build-caches.mjs";

const prefix = "rust-test-build-Linux-";
const currentKey = `${prefix}current-lock-current-run`;

function createdAt(milliseconds) {
  return new Date(milliseconds).toISOString();
}

const older = {
  createdAt: createdAt(1_000),
  id: 1,
  key: `${prefix}previous-lock-previous-run`,
};
const current = {
  createdAt: createdAt(2_000),
  id: 2,
  key: currentKey,
};
const newer = {
  createdAt: createdAt(3_000),
  id: 3,
  key: `${prefix}current-lock-newer-run`,
};

test("selects matching caches older than the completed seed", () => {
  assert.deepEqual(
    selectObsoleteCaches(
      [
        newer,
        current,
        older,
        {
          createdAt: createdAt(1_000),
          id: 4,
          key: "cargo-dependencies-Linux-lock",
        },
      ],
      { prefix, currentKey },
    ),
    [older],
  );
});

test("preserves a newer cache from a concurrent main run", () => {
  assert.deepEqual(selectObsoleteCaches([newer, current], { prefix, currentKey }), []);
});

test("refuses deletion when the completed seed is not visible", () => {
  assert.throws(
    () => selectObsoleteCaches([older], { prefix, currentKey }),
    /deleting without the current baseline could remove a usable build cache/u,
  );
});

test("rejects malformed cache records before deletion", () => {
  assert.throws(
    () =>
      selectObsoleteCaches([{ createdAt: "invalid", id: "1", key: currentKey }], {
        prefix,
        currentKey,
      }),
    /every cache needs an integer id, string key, and valid creation time/u,
  );
});

test("lists main caches and deletes only generations older than the completed seed", () => {
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
        return { status: 0, stdout: JSON.stringify([newer, current, older]) };
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
      "10000",
      "--sort",
      "created_at",
      "--order",
      "desc",
      "--json",
      "createdAt,id,key",
    ],
    ["cache", "delete", "1", "--repo", "example/veln"],
  ]);
  assert.deepEqual(deleted, [1]);
  assert.deepEqual(notices, [
    `Deleted 1 Rust test build cache older than ${currentKey}; that baseline and any newer cache remain available.`,
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
    /Inspect the cleanup job's Actions permission and rerun it/u,
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
            return { status: 0, stdout: JSON.stringify([current, older]) };
          }
          return { status: 1, stdout: "" };
        },
      }),
    /Delete obsolete cache 1 and rerun the cleanup job/u,
  );
});
