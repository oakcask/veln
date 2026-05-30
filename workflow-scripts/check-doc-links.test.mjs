import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { validateDocsLinks } from "./check-doc-links.mjs";

test("repository documentation links resolve", () => {
  const result = validateDocsLinks(path.resolve("docs"));

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("reports missing markdown files and anchors", () => {
  using fixture = tempDocs("doc-links-broken");
  fixture.write(
    "README.md",
    [
      "# Start",
      "",
      "[missing file](missing.md)",
      "[missing anchor](target.md#missing)",
      "[external](https://example.test/missing)",
    ].join("\n"),
  );
  fixture.write("target.md", "# Present\n");

  const result = validateDocsLinks(fixture.root);

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "README.md:3: missing target: missing.md",
    "README.md:4: missing anchor: target.md#missing",
  ]);
});

test("resolves duplicate heading anchors and ignores fenced code links", () => {
  using fixture = tempDocs("doc-links-anchors");
  fixture.write(
    "README.md",
    [
      "# Start",
      "",
      "[second details](target.md#details-1)",
      "",
      "```",
      "[not a link](missing.md)",
      "```",
    ].join("\n"),
  );
  fixture.write("target.md", ["# Details", "", "# Details"].join("\n"));

  const result = validateDocsLinks(fixture.root);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("ignores image links and inline code links", () => {
  using fixture = tempDocs("doc-links-ignored-syntax");
  fixture.write(
    "README.md",
    [
      "# Start",
      "",
      "![diagram](missing-image.md)",
      "`[not a link](missing-inline.md)`",
      "[real link](target.md)",
    ].join("\n"),
  );
  fixture.write("target.md", "# Present\n");

  const result = validateDocsLinks(fixture.root);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects links escaping the docs root", () => {
  using fixture = tempDocs("doc-links-escape");
  fixture.write(
    "README.md",
    ["# Start", "", "[outside](../outside.md)"].join("\n"),
  );

  const result = validateDocsLinks(fixture.root);

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "README.md:3: link escapes docs: ../outside.md",
  ]);
});

test("rejects references to unversioned paths", () => {
  using fixture = tempDocs("doc-links-unversioned");
  fixture.git("init");
  fixture.write("kept/file.md", "# Present\n");
  fixture.git("add", "kept/file.md");
  fixture.write(
    "README.md",
    [
      "# Start",
      "",
      "Versioned paths like `kept/file.md` are allowed.",
      "Do not cite `cache/generated.md`.",
      "Do not mention build output like `Entry.class`.",
    ].join("\n"),
  );

  const result = validateDocsLinks(fixture.root);

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "README.md:4: references unversioned path: cache/generated.md",
    "README.md:5: references unversioned path: Entry.class",
  ]);
});

test("resolves percent-encoded local paths and anchors", () => {
  using fixture = tempDocs("doc-links-encoded");
  fixture.write(
    "README.md",
    ["# Start", "", "[encoded](topic%20map.md#named-values)"].join("\n"),
  );
  fixture.write("topic map.md", "# Named Values\n");

  const result = validateDocsLinks(fixture.root);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

function tempDocs(name) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-`));
  return {
    root,
    git(...args) {
      const result = spawnSync("git", args, { cwd: root });
      assert.equal(result.status, 0, result.stderr.toString());
    },
    write(relativePath, text) {
      const target = path.join(root, relativePath);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, text);
    },
    [Symbol.dispose]() {
      fs.rmSync(root, { force: true, recursive: true });
    },
  };
}
