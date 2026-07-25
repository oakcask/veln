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

test("rejects specification links to proposals", () => {
  using fixture = tempDocs("doc-links-specification-proposals");
  fixture.write(
    "specification/README.md",
    [
      "# Specification",
      "",
      "[planned work](../proposals/future.md)",
      "[planned section](../proposals/future.md#scope)",
    ].join("\n"),
  );
  fixture.write("proposals/future.md", ["# Future", "", "## Scope"].join("\n"));

  const result = validateDocsLinks(fixture.root);

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "specification/README.md:3: remove specification-to-proposal link: ../proposals/future.md; specification pages must describe current behavior without routing readers to planned work",
    "specification/README.md:4: remove specification-to-proposal link: ../proposals/future.md#scope; specification pages must describe current behavior without routing readers to planned work",
  ]);
});

test("allows proposal links to specifications", () => {
  using fixture = tempDocs("doc-links-proposals-specification");
  fixture.write(
    "proposals/future.md",
    ["# Future", "", "[current behavior](../specification/README.md)"].join(
      "\n",
    ),
  );
  fixture.write("specification/README.md", "# Specification\n");

  const result = validateDocsLinks(fixture.root);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("allows archival routes from proposals through implemented records", () => {
  using fixture = tempDocs("doc-links-implemented-proposal-route");
  fixture.write(
    "proposals/README.md",
    [
      "# Proposals",
      "",
      "[archived boundary](../reference/implemented-proposals/schema-boundary.md)",
    ].join("\n"),
  );
  fixture.write(
    "reference/implemented-proposals/schema-boundary.md",
    [
      "# Schema Boundary",
      "",
      "[current syntax](../../specification/source-surface.md)",
    ].join("\n"),
  );
  fixture.write("specification/source-surface.md", "# Source Surface\n");

  const result = validateDocsLinks(fixture.root);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects bare implemented proposal paths in the proposal catalog", () => {
  using fixture = tempDocs("doc-links-proposal-catalog-bare-path");
  fixture.git("init");
  fixture.write(
    "proposals/README.md",
    [
      "# Proposals",
      "",
      "Archived under `../reference/implemented-proposals/schema-boundary.md`.",
    ].join("\n"),
  );
  fixture.write(
    "reference/implemented-proposals/schema-boundary.md",
    "# Schema Boundary\n",
  );
  fixture.git("add", ".");

  const result = validateDocsLinks(fixture.root);

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "proposals/README.md:3: use a Markdown link for implemented proposal route: ../reference/implemented-proposals/schema-boundary.md",
  ]);
});

test("allows linked implemented proposal paths in the proposal catalog", () => {
  using fixture = tempDocs("doc-links-proposal-catalog-linked-path");
  fixture.git("init");
  fixture.write(
    "proposals/README.md",
    [
      "# Proposals",
      "",
      "Archived under [schema-boundary.md](../reference/implemented-proposals/schema-boundary.md).",
    ].join("\n"),
  );
  fixture.write(
    "reference/implemented-proposals/schema-boundary.md",
    "# Schema Boundary\n",
  );
  fixture.git("add", ".");

  const result = validateDocsLinks(fixture.root);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects implemented records listed as remaining proposal routes", () => {
  using fixture = tempDocs("doc-links-implemented-remaining-route");
  fixture.write(
    "reference/implemented-proposals/driver.md",
    [
      "# Driver",
      "",
      "Remaining work is split into these proposal routes:",
      "",
      "- [active](../../proposals/active.md)",
      "- [completed](completed.md)",
    ].join("\n"),
  );
  fixture.write("proposals/active.md", "# Active\n");
  fixture.write(
    "reference/implemented-proposals/completed.md",
    "# Completed\n",
  );

  const result = validateDocsLinks(fixture.root);

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "reference/implemented-proposals/driver.md:6: remove implemented proposal from remaining-work routes: completed.md; completed routes must point readers to current specification and executable evidence",
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
