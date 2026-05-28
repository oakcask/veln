import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const prologSpec = path.join(
  "docs",
  "specification",
  "source-surface-executable.pl",
);
const sourceSurfaceDoc = path.join(
  "docs",
  "specification",
  "source-surface-full.md",
);
const startMarker = "<!-- source-surface-grammar:start -->";
const endMarker = "<!-- source-surface-grammar:end -->";

const write = process.argv.includes("--write");

const check = runSwipl(["--check"]);
if (check.skipped) {
  console.log("SWI-Prolog (swipl) not found; skipping executable source surface check.");
  process.exit(0);
}
if (check.status !== 0) {
  process.stderr.write(check.stderr);
  process.exit(check.status ?? 1);
}

const grammar = runSwipl(["--grammar"]);
if (grammar.status !== 0) {
  process.stderr.write(grammar.stderr);
  process.exit(grammar.status ?? 1);
}

const generatedBlock = `\`\`\`text\n${grammar.stdout.trimEnd()}\n\`\`\``;
const docPath = path.join(repoRoot, sourceSurfaceDoc);
const doc = fs.readFileSync(docPath, "utf8");
const replacement = replaceGeneratedBlock(doc, generatedBlock);

if (replacement === undefined) {
  console.error("Generated source-surface grammar markers are missing.");
  process.exit(1);
}

if (replacement !== doc) {
  if (write) {
    fs.writeFileSync(docPath, replacement);
    console.log("Updated generated source-surface grammar block.");
  } else {
    console.error(
      "Generated source-surface grammar block is out of date. Run the check with --write after updating the Prolog spec.",
    );
    process.exit(1);
  }
}

console.log("Executable source-surface spec fixtures and generated grammar are current.");

function runSwipl(args) {
  const result = spawnSync(
    "swipl",
    ["-q", "-s", prologSpec, "--", ...args],
    {
      cwd: repoRoot,
      encoding: "utf8",
    },
  );

  if (result.error?.code === "ENOENT") {
    if (process.env.CI) {
      console.error("SWI-Prolog (swipl) is required in CI for source-surface checks.");
      process.exit(1);
    }
    return { skipped: true };
  }

  return result;
}

function replaceGeneratedBlock(text, generatedBlock) {
  const start = text.indexOf(startMarker);
  const end = text.indexOf(endMarker);
  if (start === -1 || end === -1 || end < start) {
    return undefined;
  }

  const before = text.slice(0, start + startMarker.length);
  const after = text.slice(end);
  return `${before}\n${generatedBlock}\n${after}`;
}
