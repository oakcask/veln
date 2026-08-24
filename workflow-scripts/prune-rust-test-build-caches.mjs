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
    throw new Error(`Set ${name} so obsolete Rust test build caches can be pruned safely.`);
  }
  return value;
}

export function selectObsoleteCaches(caches, { prefix, currentKey }) {
  if (!Array.isArray(caches)) {
    throw new Error("Inspect the cache listing response before rerunning; pruning requires a JSON array.");
  }

  return caches.filter((cache) => {
    if (!Number.isSafeInteger(cache?.id) || typeof cache?.key !== "string") {
      throw new Error(
        "Inspect the cache listing response before rerunning; every cache needs an integer id and string key.",
      );
    }
    return cache.key.startsWith(prefix) && cache.key !== currentKey;
  });
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
    "100",
    "--json",
    "id,key",
  ]);
  if (listed.error || listed.status !== 0) {
    throw new Error(
      "Inspect GitHub cache permissions and rerun the seed job; old main build caches must be listed before they can be pruned.",
    );
  }

  let caches;
  try {
    caches = JSON.parse(listed.stdout);
  } catch {
    throw new Error(
      "Inspect the GitHub cache listing response and rerun the seed job; pruning requires valid JSON.",
    );
  }

  const obsolete = selectObsoleteCaches(caches, { prefix, currentKey });
  for (const cache of obsolete) {
    const deleted = runCommand(["cache", "delete", String(cache.id), "--repo", repository]);
    if (deleted.error || deleted.status !== 0) {
      throw new Error(
        `Delete obsolete cache ${cache.id} and rerun the seed job; retaining old main builds would grow cache storage without bound.`,
      );
    }
  }

  notice(
    `Pruned ${obsolete.length} obsolete main Rust test build cache${obsolete.length === 1 ? "" : "s"}; ${currentKey} remains the rolling baseline.`,
  );
  return obsolete.map((cache) => cache.id);
}

function main() {
  pruneRustTestBuildCaches({
    prefix: requireValue(process.env.RUST_TEST_BUILD_CACHE_PREFIX, "RUST_TEST_BUILD_CACHE_PREFIX"),
    currentKey: requireValue(
      process.env.RUST_TEST_BUILD_CACHE_KEY,
      "RUST_TEST_BUILD_CACHE_KEY",
    ),
    ref: requireValue(process.env.GITHUB_REF, "GITHUB_REF"),
    repository: requireValue(process.env.GITHUB_REPOSITORY, "GITHUB_REPOSITORY"),
  });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
