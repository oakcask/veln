import assert from "node:assert/strict";
import test from "node:test";
import {
  documentationMarkdownPaths,
  validateDocumentFrontmatter,
  validateDocumentationFrontmatter,
} from "./check-doc-frontmatter.mjs";

function document(frontmatter, body = "# Example") {
  return ["---", ...frontmatter, "---", "", body].join("\n");
}

test("accepts role-specific authority and an exceptional lifecycle status", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/specification/example.md",
      text: document([
        "role: specification",
        "authority: normative",
        "review-when: The documented command output or its executable fixture changes.",
      ]),
    },
    {
      file: "docs/navigation.md",
      text: document([
        "role: routing",
        "review-when: A routed document is moved or reclassified.",
      ]),
    },
    {
      file: "docs/reference/record.md",
      text: document([
        "role: implementation-record",
        "status: superseded",
        "review-when: Its replacement or supporting evidence changes.",
      ]),
    },
  ]);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects missing frontmatter", () => {
  const errors = validateDocumentFrontmatter({
    file: "docs/reference/example.md",
    text: "# Example\n",
  });

  assert.deepEqual(errors, [
    "docs/reference/example.md: add YAML frontmatter at the start of the document with one role: field and one review-when: field",
  ]);
});

test("rejects missing, duplicate, and unsupported roles", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/missing.md",
      text: document([
        "review-when: Its route changes.",
      ]),
    },
    {
      file: "docs/duplicate.md",
      text: document([
        "role: routing",
        "role: reference",
        "review-when: Its route changes.",
      ]),
    },
    {
      file: "docs/unsupported.md",
      text: document([
        "role: guide",
        "review-when: Its guidance changes.",
      ]),
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/missing.md: add exactly one role: field so readers know why the document should be opened",
    "docs/duplicate.md:3: keep exactly one role: field so the document has one purpose",
    "docs/unsupported.md:2: replace unsupported role \"guide\" with one of: implementation-record, proposal, reference, review, routing, specification",
  ]);
});

test("enforces authority from the document role", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/specification/missing.md",
      text: document([
        "role: specification",
        "review-when: Its behavior changes.",
      ]),
    },
    {
      file: "docs/reference/wrong.md",
      text: document([
        "role: reference",
        "authority: archival",
        "review-when: Its evidence changes.",
      ]),
    },
    {
      file: "docs/proposals/claimed.md",
      text: document([
        "role: proposal",
        "authority: normative",
        "review-when: Its implementation status changes.",
      ]),
    },
    {
      file: "docs/navigation.md",
      text: document([
        "role: routing",
        "authority: supporting",
        "review-when: Its routes change.",
      ]),
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/specification/missing.md: add authority: normative for role \"specification\" so readers know how its claims may be used",
    "docs/reference/wrong.md:3: replace authority \"archival\" with normative or supporting for role \"reference\"",
    "docs/proposals/claimed.md:3: remove authority from role \"proposal\"; this role does not make an authority claim",
    "docs/navigation.md:3: remove authority from role \"routing\"; this role does not make an authority claim",
  ]);
});

test("limits status to exceptional lifecycle states", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/implemented.md",
      text: document([
        "role: implementation-record",
        "status: implemented",
        "review-when: Its evidence changes.",
      ]),
    },
    {
      file: "docs/duplicate.md",
      text: document([
        "role: implementation-record",
        "status: closed",
        "status: superseded",
        "review-when: Its replacement changes.",
      ]),
    },
    {
      file: "docs/specification/rejected.md",
      text: document([
        "role: specification",
        "authority: normative",
        "status: rejected",
        "review-when: Its behavior changes.",
      ]),
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/implemented.md:3: remove status \"implemented\" or replace it with one of: closed, rejected, superseded; status records only exceptional lifecycle states",
    "docs/duplicate.md:4: keep at most one status: field so the exceptional lifecycle state is unambiguous",
    "docs/specification/rejected.md:4: remove status \"rejected\" from role \"specification\" or reclassify the document; this lifecycle state is not valid for the role",
  ]);
});

test("rejects empty, vague, duplicate, and body-only review triggers", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/empty.md",
      text: document(["role: routing", "review-when:"]),
    },
    {
      file: "docs/vague.md",
      text: document(["role: routing", "review-when: 'as needed'"]),
    },
    {
      file: "docs/duplicate.md",
      text: document([
        "role: routing",
        "review-when: Its route changes.",
        "review-when: Its audience changes.",
      ]),
    },
    {
      file: "docs/body.md",
      text: document(["role: routing"], "# Example\n\nreview-when: A body example changes."),
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/empty.md:3: name the project-state change after review-when: so maintainers can tell when this document may be stale",
    "docs/vague.md:3: replace vague review trigger \"as needed\" with a concrete project-state change",
    "docs/duplicate.md:4: keep exactly one review-when: field so the document has one review contract",
    "docs/body.md: add exactly one review-when: field naming the project-state change that requires this document to be checked again",
  ]);
});

test("rejects unclosed frontmatter and multiline YAML values", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/unclosed.md",
      text: "---\nrole: routing\nreview-when: Its route changes.\n# Unclosed\n",
    },
    {
      file: "docs/multiline.md",
      text: document([
        "role: >",
        "  routing",
        "review-when: Its route changes.",
      ]),
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/unclosed.md: close the opening YAML frontmatter with --- before the document body",
    "docs/multiline.md:2: use a single-line plain or quoted YAML scalar for role:",
  ]);
});

test("rejects legacy body status outside fenced examples", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/legacy.md",
      text: document(
        [
          "role: proposal",
          "review-when: Its implementation status changes.",
        ],
        "# Proposal\n\nStatus: proposed",
      ),
    },
    {
      file: "docs/example.md",
      text: document(
        [
          "role: routing",
          "review-when: Its example changes.",
        ],
        "# Example\n\n```markdown\nStatus: proposed\n```",
      ),
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/legacy.md:8: remove the legacy Status: line; use role, authority, and exceptional frontmatter status instead",
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
