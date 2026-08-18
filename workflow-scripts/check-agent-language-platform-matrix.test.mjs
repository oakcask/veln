import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import {
  expectedKeys,
  parseRawDiff,
  umbrellaPath,
  validateClosureTransition,
  validatePlatformMatrix,
} from "./check-agent-language-platform-matrix.mjs";

const validDocument = fs.readFileSync(umbrellaPath, "utf8");
const codexRow = validDocument.split("\n").find((line) => line.startsWith("| `codex` |"));
const claudeRow = validDocument.split("\n").find((line) => line.startsWith("| `claude-code` |"));

test("accepts the repository matrix with exact independent keys and row count", () => {
  const result = validatePlatformMatrix(validDocument);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
  assert.deepEqual(expectedKeys, [
    "codex/x86_64-unknown-linux-gnu",
    "claude-code/x86_64-unknown-linux-gnu",
  ]);
});

test("rejects missing, duplicate, reordered, and unexpected client-platform keys", () => {
  const cases = [
    mutate(validDocument, `${codexRow}\n`, ""),
    mutate(validDocument, claudeRow, codexRow),
    mutate(validDocument, `${codexRow}\n${claudeRow}`, `${claudeRow}\n${codexRow}`),
    mutate(validDocument, "`claude-code` | `x86_64-unknown-linux-gnu`", "`other-client` | `x86_64-unknown-linux-gnu`"),
  ];

  for (const document of cases) {
    assert.equal(validatePlatformMatrix(document).valid, false);
    assert.match(validatePlatformMatrix(document).errors.join("\n"), /matrix (?:rows|key|keys)/);
  }
});

test("rejects empty client and platform identifiers", () => {
  for (const [source, replacement] of [
    ["| `codex` | `x86_64-unknown-linux-gnu` |", "| `` | `x86_64-unknown-linux-gnu` |"],
    ["| `codex` | `x86_64-unknown-linux-gnu` |", "| `codex` | `` |"],
  ]) {
    const result = validatePlatformMatrix(mutate(validDocument, source, replacement));
    assert.equal(result.valid, false);
    assert.match(result.errors.join("\n"), /nonempty literal client and platform/);
  }
});

test("rejects ranged, wildcard, placeholder, and catch-all client-platform keys", () => {
  for (const invalid of ["codex-1..2", "codex-*", "placeholder", "all"]) {
    const result = validatePlatformMatrix(mutate(validDocument, "| `codex` |", `| \`${invalid}\` |`));
    assert.equal(result.valid, false, invalid);
    assert.match(result.errors.join("\n"), /ranges, wildcards, placeholders, and catch-all values/);
  }
});

test("rejects missing and empty compatibility fields", () => {
  const cells = codexRow.slice(1, -1).split("|");
  const missing = [...cells.slice(0, 4), ...cells.slice(5)].join("|");
  const empty = [...cells];
  empty[4] = " `` ";

  const missingResult = validatePlatformMatrix(mutate(validDocument, codexRow, `|${missing}|`));
  const emptyResult = validatePlatformMatrix(mutate(validDocument, codexRow, `|${empty.join("|")}|`));

  assert.equal(missingResult.valid, false);
  assert.match(missingResult.errors.join("\n"), /restore all 11 compatibility fields/);
  assert.equal(emptyResult.valid, false);
  assert.match(emptyResult.errors.join("\n"), /restore a nonempty exact literal/);
});

test("rejects ranged, wildcard, and placeholder compatibility values", () => {
  for (const invalid of ["1..2", "contract-*", "latest"]) {
    const result = validatePlatformMatrix(mutate(validDocument, "`codex-host-1`", `\`${invalid}\``));
    assert.equal(result.valid, false, invalid);
    assert.match(result.errors.join("\n"), /matrix row 1 Host build/);
  }
});

test("rejects malformed validator integrity digests", () => {
  const digest = "a02d8a7d8298ecd869813e0d4f5ea6334ebfb088c2686e67f8f8c2661e136cc9";
  for (const invalid of [digest.slice(0, -1), `${digest.slice(0, -1)}G`, `${digest}0`]) {
    const result = validatePlatformMatrix(mutate(validDocument, digest, invalid));
    assert.equal(result.valid, false, invalid);
    assert.match(result.errors.join("\n"), /exactly 64 lowercase hexadecimal digits/);
  }
});

test("rejects a checked row count that differs from the table", () => {
  const result = validatePlatformMatrix(mutate(validDocument, "row count: `2`", "row count: `3`"));

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /replace `3` with `2`/);
});

test("rejects unnamed platform universes in every required reference", () => {
  const references = [
    ["Every row in the Closed Client-Platform Matrix uses client-native installation", "Every supported platform uses client-native installation", "plugin requirement reference"],
    ["For every row in the Closed Client-Platform Matrix:", "For every supported client and platform:", "Q21 evidence reference"],
    ["missing Closed Client-Platform Matrix row", "missing supported platform", "Q22 totality reference"],
    ["every row in the Closed Client-Platform Matrix passes", "every supported client-platform cell passes", "completion rule reference"],
  ];

  for (const [source, replacement, error] of references) {
    const result = validatePlatformMatrix(mutate(validDocument, source, replacement));
    assert.equal(result.valid, false, error);
    assert.match(result.errors.join("\n"), new RegExp(error));
  }
});

test("activates closure scope only for the exact matrix-addition transition", () => {
  const preClosure = mutate(validDocument, "### Closed Client-Platform Matrix", "### Planned Client Matrix");
  const allowed = [{
    oldMode: "100644",
    newMode: "100644",
    status: "M",
    path: umbrellaPath,
  }];

  const active = validateClosureTransition({ baseText: preClosure, headText: validDocument, changes: allowed });
  const laterDocs = validateClosureTransition({ baseText: validDocument, headText: validDocument, changes: [{
    oldMode: "100644",
    newMode: "100644",
    status: "M",
    path: "docs/README.md",
  }] });

  assert.deepEqual(active, { active: true, errors: [], valid: true });
  assert.deepEqual(laterDocs, { active: false, errors: [], valid: true });
});

test("closure transition rejects out-of-scope changes, protected renames, and Git type changes", () => {
  const preClosure = mutate(validDocument, "### Closed Client-Platform Matrix", "### Planned Client Matrix");
  const cases = [
    {
      change: { oldMode: "100644", newMode: "100644", status: "M", path: "crates/veln-mcp/src/main.rs" },
      error: /out-of-scope change/,
    },
    {
      change: { oldMode: "100644", newMode: "100644", status: "R100", oldPath: umbrellaPath, path: "docs/proposals/renamed.md" },
      error: /restore the protected path instead of renaming it/,
    },
    {
      change: { oldMode: "100644", newMode: "120000", status: "T", path: umbrellaPath },
      error: /restore a regular file Git type/,
    },
  ];

  for (const { change, error } of cases) {
    const result = validateClosureTransition({ baseText: preClosure, headText: validDocument, changes: [change] });
    assert.equal(result.active, true);
    assert.equal(result.valid, false);
    assert.match(result.errors.join("\n"), error);
  }
});

test("closure transition permits the one proposal archive move", () => {
  const preClosure = mutate(validDocument, "### Closed Client-Platform Matrix", "### Planned Client Matrix");
  const result = validateClosureTransition({
    baseText: preClosure,
    headText: validDocument,
    changes: [{
      oldMode: "100644",
      newMode: "100644",
      status: "R100",
      oldPath: "docs/proposals/agent-language-services-platform-matrix-closure.md",
      path: "docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md",
    }],
  });

  assert.deepEqual(result, { active: true, errors: [], valid: true });
});

test("parses modifications, renames, and Git type changes from raw diff records", () => {
  const raw = [
    ":100644 100644 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb M",
    umbrellaPath,
    ":100644 100644 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb R100",
    "docs/proposals/old.md",
    "docs/reference/new.md",
    ":100644 120000 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb T",
    "docs/proposals/type.md",
    "",
  ].join("\0");

  assert.deepEqual(parseRawDiff(raw), [
    { oldMode: "100644", newMode: "100644", status: "M", path: umbrellaPath },
    { oldMode: "100644", newMode: "100644", status: "R100", oldPath: "docs/proposals/old.md", path: "docs/reference/new.md" },
    { oldMode: "100644", newMode: "120000", status: "T", path: "docs/proposals/type.md" },
  ]);
});

function mutate(text, source, replacement) {
  assert.ok(text.includes(source), `fixture source is present: ${source}`);
  return text.replace(source, replacement);
}
