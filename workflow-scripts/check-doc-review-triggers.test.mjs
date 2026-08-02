import assert from "node:assert/strict";
import test from "node:test";
import {
  documentationMarkdownPaths,
  validateDocumentReviewTrigger,
  validateDocumentationReviewTriggers,
} from "./check-doc-review-triggers.mjs";

test("accepts one concrete project-state review trigger", () => {
  const result = validateDocumentationReviewTriggers([
    {
      file: "docs/specification/example.md",
      text: [
        "---",
        "review-when: The documented command output or its executable fixture changes.",
        "---",
        "",
        "# Example",
        "",
        "Status: implemented",
      ].join("\n"),
    },
  ]);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects a missing review trigger", () => {
  const errors = validateDocumentReviewTrigger({
    file: "docs/reference/example.md",
    text: "# Example\n",
  });

  assert.deepEqual(errors, [
    "docs/reference/example.md: add YAML frontmatter at the start of the document with one review-when: field",
  ]);
});

test("rejects empty and vague review triggers", () => {
  const result = validateDocumentationReviewTriggers([
    {
      file: "docs/empty.md",
      text: "---\nreview-when:\n---\n\n# Empty\n",
    },
    {
      file: "docs/vague.md",
      text: "---\nreview-when: 'as needed'\n---\n\n# Vague\n",
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/empty.md:2: name the project-state change after review-when: so maintainers can tell when this document may be stale",
    "docs/vague.md:2: replace vague review trigger \"as needed\" with a concrete project-state change",
  ]);
});

test("rejects duplicate review triggers", () => {
  const errors = validateDocumentReviewTrigger({
    file: "docs/proposals/example.md",
    text: [
      "---",
      "review-when: Its implementation status changes.",
      "review-when: Its acceptance evidence changes.",
      "---",
      "",
      "# Example",
    ].join("\n"),
  });

  assert.deepEqual(errors, [
    "docs/proposals/example.md:3: keep exactly one review-when: field so the document has one review contract",
  ]);
});

test("requires review triggers in frontmatter rather than the document body", () => {
  const errors = validateDocumentReviewTrigger({
    file: "docs/example.md",
    text: [
      "---",
      "status: routing",
      "---",
      "",
      "# Example",
      "",
      "review-when: A body example changes.",
    ].join("\n"),
  });

  assert.deepEqual(errors, [
    "docs/example.md: add exactly one review-when: field to the YAML frontmatter naming the project-state change that requires this document to be checked again",
  ]);
});

test("rejects unclosed frontmatter and multiline YAML values", () => {
  const result = validateDocumentationReviewTriggers([
    {
      file: "docs/unclosed.md",
      text: "---\nreview-when: Its route changes.\n# Unclosed\n",
    },
    {
      file: "docs/multiline.md",
      text: "---\nreview-when: >\n  Its route changes.\n---\n",
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/unclosed.md: close the opening YAML frontmatter with --- before the document body",
    "docs/multiline.md:2: use a single-line plain or quoted YAML scalar for review-when:",
  ]);
});

test("selects unique Markdown files under docs", () => {
  assert.deepEqual(
    documentationMarkdownPaths([
      "src/example.md",
      "docs/example.md",
      "docs/example.md",
      "docs/fixture.veln",
      "docs/reference/EXAMPLE.MD",
    ]),
    ["docs/example.md", "docs/reference/EXAMPLE.MD"],
  );
});
