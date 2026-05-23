#!/usr/bin/env node

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, normalize, sep } from "node:path";

const outputPath = process.argv[2];

if (!outputPath) {
  console.error("usage: cache-stdin.mjs OUTPUT_PATH");
  process.exit(2);
}

const normalizedPath = normalize(outputPath);

if (isAbsolute(normalizedPath)) {
  console.error("OUTPUT_PATH must be relative");
  process.exit(2);
}

const allowedPrefixes = [`.cache${sep}`, `tmp${sep}`];
const isAllowedPath = allowedPrefixes.some((prefix) => normalizedPath.startsWith(prefix));

if (!isAllowedPath || normalizedPath.includes(`..${sep}`) || normalizedPath === "..") {
  console.error("OUTPUT_PATH must be under .cache/ or tmp/");
  process.exit(2);
}

const chunks = [];

for await (const chunk of process.stdin) {
  chunks.push(chunk);
}

await mkdir(dirname(normalizedPath), { recursive: true });
await writeFile(normalizedPath, Buffer.concat(chunks));
