import assert from "node:assert/strict";
import test from "node:test";
import {
  documentationMarkdownPaths,
  renderGitHubErrorAnnotation,
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
        "update-when: The documented command output or its executable fixture changes.",
      ]),
    },
    {
      file: "docs/navigation.md",
      text: document([
        "role: routing",
        "update-when: A routed document is moved or reclassified.",
      ]),
    },
    {
      file: "docs/reference/record.md",
      text: document([
        "role: implementation-record",
        "status: superseded",
        "update-when: Its replacement or supporting evidence changes.",
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
    "docs/reference/example.md: add YAML frontmatter at the start of the document with one role: field and one update-when: field",
  ]);
});

test("rejects missing, duplicate, and unsupported roles", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/missing.md",
      text: document([
        "update-when: Its route changes.",
      ]),
    },
    {
      file: "docs/duplicate.md",
      text: document([
        "role: routing",
        "role: reference",
        "update-when: Its route changes.",
      ]),
    },
    {
      file: "docs/unsupported.md",
      text: document([
        "role: guide",
        "update-when: Its guidance changes.",
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
        "update-when: Its behavior changes.",
      ]),
    },
    {
      file: "docs/reference/wrong.md",
      text: document([
        "role: reference",
        "authority: archival",
        "update-when: Its evidence changes.",
      ]),
    },
    {
      file: "docs/proposals/claimed.md",
      text: document([
        "role: proposal",
        "authority: normative",
        "update-when: Its implementation status changes.",
      ]),
    },
    {
      file: "docs/navigation.md",
      text: document([
        "role: routing",
        "authority: supporting",
        "update-when: Its routes change.",
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
        "update-when: Its evidence changes.",
      ]),
    },
    {
      file: "docs/duplicate.md",
      text: document([
        "role: implementation-record",
        "status: closed",
        "status: superseded",
        "update-when: Its replacement changes.",
      ]),
    },
    {
      file: "docs/specification/rejected.md",
      text: document([
        "role: specification",
        "authority: normative",
        "status: rejected",
        "update-when: Its behavior changes.",
      ]),
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/implemented.md:3: remove status \"implemented\" or replace it with one of: closed, rejected, superseded; status records only exceptional lifecycle states",
    "docs/duplicate.md:4: keep at most one status: field so the exceptional lifecycle state is unambiguous",
    "docs/specification/rejected.md:4: remove status \"rejected\" from role \"specification\" or reclassify the document; this lifecycle state is not valid for the role",
  ]);
});

test("rejects empty, vague, duplicate, and body-only update triggers", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/empty.md",
      text: document(["role: routing", "update-when:"]),
    },
    {
      file: "docs/vague.md",
      text: document(["role: routing", "update-when: 'as needed'"]),
    },
    {
      file: "docs/duplicate.md",
      text: document([
        "role: routing",
        "update-when: Its route changes.",
        "update-when: Its audience changes.",
      ]),
    },
    {
      file: "docs/body.md",
      text: document(["role: routing"], "# Example\n\nupdate-when: A body example changes."),
    },
  ]);

  assert.deepEqual(result.errors, [
    "docs/empty.md:3: name the project-state change after update-when: so maintainers can tell when to update this document",
    "docs/vague.md:3: replace vague update trigger \"as needed\" with a concrete project-state change",
    "docs/duplicate.md:4: keep exactly one update-when: field so the document has one update contract",
    "docs/body.md: add exactly one update-when: field naming the project-state change that can make this document stale",
  ]);
});

test("rejects the legacy review-when field", () => {
  const errors = validateDocumentFrontmatter({
    file: "docs/reference/example.md",
    text: document([
      "role: reference",
      "authority: supporting",
      "review-when: The supporting evidence changes.",
    ]),
  });

  assert.deepEqual(errors, [
    "docs/reference/example.md:4: replace review-when: with update-when: so the field identifies when project changes can make the document stale",
  ]);
});

test("keeps punctuation readable in GitHub annotation messages", () => {
  assert.equal(
    renderGitHubErrorAnnotation("docs/example.md:4: replace review-when: with update-when:, then retry\nnext step"),
    "::error title=Invalid documentation frontmatter::docs/example.md:4: replace review-when: with update-when:, then retry%0Anext step",
  );
});

test("rejects unclosed frontmatter and multiline YAML values", () => {
  const result = validateDocumentationFrontmatter([
    {
      file: "docs/unclosed.md",
      text: "---\nrole: routing\nupdate-when: Its route changes.\n# Unclosed\n",
    },
    {
      file: "docs/multiline.md",
      text: document([
        "role: >",
        "  routing",
        "update-when: Its route changes.",
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
          "update-when: Its implementation status changes.",
        ],
        "# Proposal\n\nStatus: proposed",
      ),
    },
    {
      file: "docs/example.md",
      text: document(
        [
          "role: routing",
          "update-when: Its example changes.",
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
