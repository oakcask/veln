import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

function runGh(args) {
  return spawnSync("gh", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function requireValue(value, name) {
  if (!value) {
    throw new Error(`Set ${name} so obsolete Rust test build caches can be deleted safely.`);
  }
  return value;
}

function validateCache(cache) {
  const createdAt = Date.parse(cache?.createdAt);
  if (
    !Number.isSafeInteger(cache?.id) ||
    typeof cache?.key !== "string" ||
    !Number.isFinite(createdAt)
  ) {
    throw new Error(
      "Inspect the cache listing response before rerunning; every cache needs an integer id, string key, and valid creation time.",
    );
  }
  return createdAt;
}

export function selectObsoleteCaches(caches, { prefix, currentKey }) {
  if (!Array.isArray(caches)) {
    throw new Error("Inspect the cache listing response before rerunning; deletion requires a JSON array.");
  }

  const creationTimes = new Map(caches.map((cache) => [cache.id, validateCache(cache)]));
  const current = caches.find((cache) => cache.key === currentKey);
  if (!current) {
    throw new Error(
      `Rerun the cache cleanup after ${currentKey} is visible; deleting without the current baseline could remove a usable build cache.`,
    );
  }

  const currentCreationTime = creationTimes.get(current.id);
  return caches.filter(
    (cache) =>
      cache.key.startsWith(prefix) && creationTimes.get(cache.id) < currentCreationTime,
  );
}

export function pruneRustTestBuildCaches({
  prefix,
  currentKey,
  ref,
  repository,
  runCommand = runGh,
  notice = console.log,
}) {
  const listed = runCommand([
    "cache",
    "list",
    "--repo",
    repository,
    "--key",
    prefix,
    "--ref",
    ref,
    "--limit",
    "10000",
    "--sort",
    "created_at",
    "--order",
    "desc",
    "--json",
    "createdAt,id,key",
  ]);
  if (listed.error || listed.status !== 0) {
    throw new Error(
      "Inspect the cleanup job's Actions permission and rerun it; old main build caches must be listed before deletion.",
    );
  }

  let caches;
  try {
    caches = JSON.parse(listed.stdout);
  } catch {
    throw new Error(
      "Inspect the GitHub cache listing response and rerun the cleanup job; deletion requires valid JSON.",
    );
  }

  const obsolete = selectObsoleteCaches(caches, { prefix, currentKey });
  for (const cache of obsolete) {
    const deleted = runCommand(["cache", "delete", String(cache.id), "--repo", repository]);
    if (deleted.error || deleted.status !== 0) {
      throw new Error(
        `Delete obsolete cache ${cache.id} and rerun the cleanup job; retaining old main builds consumes the Actions cache quota.`,
      );
    }
  }

  notice(
    `Deleted ${obsolete.length} Rust test build cache${obsolete.length === 1 ? "" : "s"} older than ${currentKey}; that baseline and any newer cache remain available.`,
  );
  return obsolete.map((cache) => cache.id);
}

function main() {
  requireValue(process.env.GH_TOKEN, "GH_TOKEN");
  pruneRustTestBuildCaches({
    prefix: requireValue(process.env.RUST_TEST_BUILD_CACHE_PREFIX, "RUST_TEST_BUILD_CACHE_PREFIX"),
    currentKey: requireValue(process.env.RUST_TEST_BUILD_CACHE_KEY, "RUST_TEST_BUILD_CACHE_KEY"),
    ref: requireValue(process.env.GITHUB_REF, "GITHUB_REF"),
    repository: requireValue(process.env.GITHUB_REPOSITORY, "GITHUB_REPOSITORY"),
  });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
