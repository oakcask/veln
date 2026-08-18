import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  parseRawDiff,
  renderGitHubErrorAnnotation,
  umbrellaPath,
  validateClosureTransition,
  validatePlatformMatrix,
} from "./check-agent-language-platform-matrix.mjs";

const expectedKeys = [
  "codex/x86_64-unknown-linux-gnu",
  "claude-code/x86_64-unknown-linux-gnu",
];
const validDocument = fs.readFileSync(umbrellaPath, "utf8");
const validatorPath = fileURLToPath(new URL("./check-agent-language-platform-matrix.mjs", import.meta.url));
const rows = validDocument.split("\n").filter((line) => /^\| `(?:codex|claude-code)` \|/.test(line));
const [codexRow, claudeRow] = rows;

test("accepts the repository matrix with independent exact keys and row count", () => {
  const result = validatePlatformMatrix(validDocument);
  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
  assert.deepEqual(rows.map(rowKey), expectedKeys);
});

test("rejects missing, duplicate, reordered, and unexpected client-platform keys", () => {
  const cases = [
    replace(validDocument, `${codexRow}\n`, ""),
    replace(validDocument, claudeRow, codexRow),
    replace(validDocument, `${codexRow}\n${claudeRow}`, `${claudeRow}\n${codexRow}`),
    replace(validDocument, "`claude-code` | `x86_64-unknown-linux-gnu`", "`other-client` | `x86_64-unknown-linux-gnu`"),
  ];
  for (const document of cases) {
    const result = validatePlatformMatrix(document);
    assert.equal(result.valid, false);
    assert.match(result.errors.join("\n"), /matrix (?:rows|key|keys)/);
  }
});

test("rejects empty, ranged, wildcard, placeholder, and catch-all keys", () => {
  for (const invalid of ["", "codex-1..2", "codex-*", "placeholder", "all"]) {
    const result = validatePlatformMatrix(replace(validDocument, "| `codex` |", `| \`${invalid}\` |`));
    assert.equal(result.valid, false, invalid);
    assert.match(result.errors.join("\n"), /matrix row 1 Client/);
  }
  const emptyPlatform = validatePlatformMatrix(replace(validDocument, "| `codex` | `x86_64-unknown-linux-gnu` |", "| `codex` | `` |"));
  assert.equal(emptyPlatform.valid, false);
  assert.match(emptyPlatform.errors.join("\n"), /matrix row 1 Platform/);
});

test("rejects missing, empty, ranged, wildcard, and placeholder compatibility fields", () => {
  const cells = codexRow.slice(1, -1).split("|");
  const missing = [...cells.slice(0, 4), ...cells.slice(5)].join("|");
  const empty = [...cells];
  empty[4] = " `` ";
  const missingResult = validatePlatformMatrix(replace(validDocument, codexRow, `|${missing}|`));
  const emptyResult = validatePlatformMatrix(replace(validDocument, codexRow, `|${empty.join("|")}|`));
  assert.match(missingResult.errors.join("\n"), /restore all 11 compatibility fields/);
  assert.match(emptyResult.errors.join("\n"), /restore a nonempty exact literal/);

  for (const invalid of ["1..2", "contract-*", "latest"]) {
    const result = validatePlatformMatrix(replace(validDocument, "`codex-host-1`", `\`${invalid}\``));
    assert.equal(result.valid, false, invalid);
    assert.match(result.errors.join("\n"), /matrix row 1 Host build/);
  }
});

test("rejects malformed digests and values outside exact code spans", () => {
  const digest = "a02d8a7d8298ecd869813e0d4f5ea6334ebfb088c2686e67f8f8c2661e136cc9";
  for (const invalid of [digest.slice(0, -1), `${digest.slice(0, -1)}G`, `${digest}0`]) {
    const result = validatePlatformMatrix(replace(validDocument, digest, invalid));
    assert.equal(result.valid, false, invalid);
    assert.match(result.errors.join("\n"), /exactly 64 lowercase hexadecimal digits/);
  }
  const unquoted = validatePlatformMatrix(replace(validDocument, "`codex-host-1`", "codex-host-1"));
  assert.match(unquoted.errors.join("\n"), /wrap the exact value in one Markdown code span/);
});

test("rejects a different exact-looking compatibility value", () => {
  const result = validatePlatformMatrix(replace(validDocument, "`codex-host-1`", "`codex-host-2`"));
  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /matrix row 1 Host build: restore the closed value `codex-host-1`/);
});

test("rejects missing, duplicate, and malformed matrix structure", () => {
  const missingHeading = validatePlatformMatrix(replace(validDocument, "### Closed Client-Platform Matrix", "### Client Matrix"));
  const duplicateHeading = validatePlatformMatrix(`${validDocument}\n### Closed Client-Platform Matrix\n`);
  const badCount = validatePlatformMatrix(replace(validDocument, "row count: `2`", "row count: `3`"));
  const matrixDelimiter = "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |";
  const badDelimiter = validatePlatformMatrix(replace(validDocument, matrixDelimiter, matrixDelimiter.replace("| --- |", "| :--- |")));
  assert.match(missingHeading.errors.join("\n"), /restore exactly one/);
  assert.match(duplicateHeading.errors.join("\n"), /found 2/);
  assert.match(badCount.errors.join("\n"), /replace `3` with `2`/);
  assert.match(badDelimiter.errors.join("\n"), /matrix delimiter/);
});

test("rejects every required reference when it becomes unbound", () => {
  const references = [
    ["Every row in the Closed Client-Platform Matrix starts one server", "Each client starts one server", "plugin server lifecycle"],
    ["both clients in the Closed Client-Platform Matrix expose", "supported clients expose", "plugin installer boundary"],
    ["Every row in the Closed Client-Platform Matrix uses client-native installation", "Every platform uses client-native installation", "plugin native validation"],
    ["row in the Closed Client-Platform\nMatrix.", "supported client-platform cell.", "conformance requirement coverage"],
    ["For every row in the Closed Client-Platform Matrix:", "For every supported platform:", "Q21 evidence"],
    ["missing Closed Client-Platform Matrix row", "missing matrix cell", "Q22 totality"],
    ["every row in the Closed Client-Platform Matrix passes", "every declared cell passes", "completion rule"],
    ["row in the Closed Client-Platform Matrix passes with no orphan", "supported client-platform cell passes with no orphan", "plugin acceptance completion"],
  ];
  for (const [source, replacement, label] of references) {
    const result = validatePlatformMatrix(replace(validDocument, source, replacement));
    assert.equal(result.valid, false, label);
    assert.match(result.errors.join("\n"), new RegExp(label));
  }
});

test("activates only for matrix addition and retires for later documentation", () => {
  const preClosure = replace(validDocument, "### Closed Client-Platform Matrix", "### Planned Client Matrix");
  const allowed = [{ oldMode: "100644", newMode: "100644", status: "M", path: umbrellaPath }];
  const allowedReferenceRoute = [{ oldMode: "100644", newMode: "100644", status: "M", path: "docs/reference/README.md" }];
  assert.deepEqual(validateClosureTransition({ baseText: preClosure, headText: validDocument, changes: allowed }), {
    active: true, errors: [], valid: true,
  });
  assert.deepEqual(validateClosureTransition({ baseText: preClosure, headText: validDocument, changes: allowedReferenceRoute }), {
    active: true, errors: [], valid: true,
  });
  assert.deepEqual(validateClosureTransition({
    baseText: validDocument,
    headText: validDocument,
    changes: [{ oldMode: "100644", newMode: "100644", status: "M", path: "docs/README.md" }],
  }), { active: false, errors: [], valid: true });
  assert.deepEqual(validateClosureTransition({ baseText: preClosure, headText: preClosure, changes: allowed }), {
    active: false, errors: [], valid: true,
  });
});

test("closure transition rejects out-of-scope paths, protected renames, and Git type or executable changes", () => {
  const preClosure = replace(validDocument, "### Closed Client-Platform Matrix", "### Planned Client Matrix");
  const cases = [
    [{ oldMode: "100644", newMode: "100644", status: "M", path: "crates/veln-mcp/src/main.rs" }, /out-of-scope/],
    [{ oldMode: "100644", newMode: "100644", status: "R100", oldPath: umbrellaPath, path: "docs/proposals/renamed.md" }, /instead of renaming/],
    [{ oldMode: "100644", newMode: "120000", status: "T", path: umbrellaPath }, /type or mode changes/],
    [{ oldMode: "100644", newMode: "100755", status: "M", path: umbrellaPath }, /type or mode changes/],
  ];
  for (const [change, error] of cases) {
    const result = validateClosureTransition({ baseText: preClosure, headText: validDocument, changes: [change] });
    assert.equal(result.active, true);
    assert.equal(result.valid, false);
    assert.match(result.errors.join("\n"), error);
  }
});

test("closure transition permits the proposal archive move", () => {
  const preClosure = replace(validDocument, "### Closed Client-Platform Matrix", "### Planned Client Matrix");
  const result = validateClosureTransition({
    baseText: preClosure,
    headText: validDocument,
    changes: [{
      oldMode: "100644", newMode: "100644", status: "R100",
      oldPath: "docs/proposals/agent-language-services-platform-matrix-closure.md",
      path: "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md",
    }],
  });
  assert.deepEqual(result, { active: true, errors: [], valid: true });
});

test("real Git ranges activate the closure guard once and retire it afterward", () => {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), "veln-platform-matrix-"));
  try {
    git(repo, "init");
    git(repo, "config", "user.email", "agent@example.invalid");
    git(repo, "config", "user.name", "Agent");
    const umbrella = path.join(repo, umbrellaPath);
    fs.mkdirSync(path.dirname(umbrella), { recursive: true });
    fs.writeFileSync(umbrella, replace(validDocument, "### Closed Client-Platform Matrix", "### Planned Client Matrix"));
    git(repo, "add", umbrellaPath);
    git(repo, "commit", "-m", "base");
    const base = git(repo, "rev-parse", "HEAD").stdout.trim();

    fs.writeFileSync(umbrella, validDocument);
    git(repo, "add", umbrellaPath);
    git(repo, "commit", "-m", "close matrix");
    const closure = git(repo, "rev-parse", "HEAD").stdout.trim();
    assert.equal(runValidator(repo, base, closure).status, 0);

    const unrelated = path.join(repo, "docs/README.md");
    fs.writeFileSync(unrelated, "# Documentation\n");
    git(repo, "add", "docs/README.md");
    git(repo, "commit", "-m", "later docs");
    const later = git(repo, "rev-parse", "HEAD").stdout.trim();
    assert.equal(runValidator(repo, closure, later).status, 0);

  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});

test("parses modifications, renames, copies, and Git type changes", () => {
  const raw = [
    ":100644 100644 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb M", umbrellaPath,
    ":100644 100644 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb R100", "docs/proposals/old.md", "docs/reference/new.md",
    ":100644 100644 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb C100", "docs/source.md", "docs/copy.md",
    ":100644 120000 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb T", "docs/proposals/type.md", "",
  ].join("\0");
  assert.deepEqual(parseRawDiff(raw), [
    { oldMode: "100644", newMode: "100644", status: "M", path: umbrellaPath },
    { oldMode: "100644", newMode: "100644", status: "R100", oldPath: "docs/proposals/old.md", path: "docs/reference/new.md" },
    { oldMode: "100644", newMode: "100644", status: "C100", oldPath: "docs/source.md", path: "docs/copy.md" },
    { oldMode: "100644", newMode: "120000", status: "T", path: "docs/proposals/type.md" },
  ]);
});

test("renders an actionable multiline GitHub annotation", () => {
  assert.equal(
    renderGitHubErrorAnnotation("matrix row 1: restore it\nfinite coverage depends on it"),
    "::error title=Invalid agent language platform matrix::matrix row 1: restore it%0Afinite coverage depends on it",
  );
});

function rowKey(line) {
  const cells = line.slice(1, -1).split("|").map((cell) => cell.trim().slice(1, -1));
  return `${cells[0]}/${cells[1]}`;
}

function replace(text, source, replacement) {
  assert.ok(text.includes(source), `fixture source is present: ${source}`);
  return text.replace(source, replacement);
}

function git(repo, ...args) {
  const result = spawnSync("git", args, { cwd: repo, encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  return result;
}

function runValidator(repo, base, head) {
  return spawnSync(process.execPath, [validatorPath], {
    cwd: repo,
    encoding: "utf8",
    env: {
      ...process.env,
      AGENT_PLATFORM_MATRIX_BASE_SHA: base,
      AGENT_PLATFORM_MATRIX_HEAD_SHA: head,
    },
  });
}
